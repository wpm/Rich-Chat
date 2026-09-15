// End-to-end check of the built app in headless Chromium.
//
// Serves a Trunk `dist` directory, opens it in light and dark mode, types
// Markdown, math, and code into the composer, and asserts what appears:
// the welcome bubble, the live preview, the sent bubble, fonts,
// highlighting, the copy button, progressive rendering of an unfinished
// equation, and a clean console. Then it works the app's controls: the
// theme switch, the bubble side and colour, dragging the text box taller,
// and that all of it survives a reload. Screenshots land in ./screenshots
// for a human to look at.
//
//   npm test                 # against ../dist, from this directory
//   node check.mjs <dist>    # against another build

import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { chromium } from 'playwright';

const dist = resolve(process.argv[2] ?? '../dist');
const shots = resolve('screenshots');
await mkdir(shots, { recursive: true });

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css' };
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
  const file = join(dist, path === '/' ? 'index.html' : path);
  try {
    const body = await readFile(file);
    response.writeHead(200, { 'content-type': types[extname(file)] ?? 'application/octet-stream' });
    response.end(body);
  } catch {
    response.writeHead(404).end();
  }
});
await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
const origin = `http://127.0.0.1:${server.address().port}/`;

const sample = `# Rich Chat check

Some *emphasis*, **strong**, ~~struck~~, \`inline code\`, a [link](https://example.com), and https://leptos.dev bare.

Inline math $e^{i\\pi} + 1 = 0$ and $\\frac{a}{b}$ in a sentence, then display:

$$
\\int_0^\\infty e^{-x^2}\\,dx = \\frac{\\sqrt{\\pi}}{2}
$$

\`\`\`rust
fn main() {
    let xs: Vec<u32> = (1..=5).map(|x| x * x).collect();
    println!("{xs:?}"); // squares
}
\`\`\`

| Language | Highlighted |
|---|:--:|
| Rust | yes |

- [x] tables
- [ ] footnotes[^1]

> [!TIP]
> Alerts render too.

[^1]: A footnote.
`;

let failures = 0;
function check(condition, label) {
  console.log(`${condition ? 'ok  ' : 'FAIL'} ${label}`);
  if (!condition) failures += 1;
}

/** Sets the composer's text the way typing does, through the input event. */
async function type(page, text) {
  await page.evaluate((text) => {
    const box = document.querySelector('.rc-composer-input');
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(box, text);
    box.dispatchEvent(new Event('input', { bubbles: true }));
  }, text);
  await page.waitForTimeout(150);
}

const browser = await chromium.launch();
try {
  for (const colorScheme of ['light', 'dark']) {
    const page = await browser.newPage({ colorScheme, viewport: { width: 900, height: 900 }, deviceScaleFactor: 2 });
    const errors = [];
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
    page.on('pageerror', (error) => errors.push(String(error)));
    console.log(`\n${colorScheme} mode`);

    await page.goto(origin, { waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');

    // The window opens on a tour from the assistant's side.
    const welcome = await page.$('.rc-message-assistant');
    check(welcome !== null, 'the welcome bubble is there on startup');
    check((await page.$$('.rc-message')).length === 1, 'and it is the only message');
    const welcomeBox = await page.$eval('.rc-message-assistant .rc-bubble', (el) => el.getBoundingClientRect());
    const paneBox = await page.$eval('.rc-messages', (el) => el.getBoundingClientRect());
    check(welcomeBox.left - paneBox.left < paneBox.right - welcomeBox.right, 'the welcome bubble sits on the left');
    check((await page.$$('.rc-message-assistant .rc-codeblock')).length === 2, 'the welcome has two code blocks');
    check((await page.$$('.rc-message-assistant pre.rc-code span')).length > 40, 'both are highlighted');
    check((await page.$$('.rc-message-assistant math[display=block]')).length >= 3, 'the welcome has display math');
    check(await page.$('.rc-message-assistant merror') === null, 'every welcome equation parsed');
    check(await page.$('.rc-message-assistant table') !== null, 'the welcome has a table');
    await page.screenshot({ path: `${shots}/${colorScheme}-0-welcome.png` });

    // Progressive rendering: an unfinished equation with unbalanced braces.
    await type(page, 'Progressive: $$\\sum_{k=1}^n k^2 = \\frac{n(n+1)(2n+');
    check(await page.$('.rc-composer-preview math') !== null, 'unfinished $$ renders as math in the preview');
    await page.screenshot({ path: `${shots}/${colorScheme}-1-draft-math.png` });

    // An open fence is a code block already.
    await type(page, 'Open fence:\n\n```python\nfor i in range(3):\n    print(i');
    check(await page.$('.rc-composer-preview .rc-codeblock') !== null, 'open fence renders as a code block');
    check((await page.$$('.rc-composer-preview .rc-codeblock span')).length > 0, 'open fence is highlighted');

    // The full sample: preview, then send.
    await type(page, sample);
    const previewHtml = await page.$eval('.rc-composer-preview .rc-rich', (el) => el.innerHTML);
    await page.screenshot({ path: `${shots}/${colorScheme}-2-preview.png` });
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForSelector('.rc-message-user');
    await page.waitForTimeout(300);
    // Captured before the screenshot: Playwright hides the caret for a
    // screenshot by touching inline styles on form controls, which leaves
    // an empty style attribute on the task-list checkboxes.
    const bubbleHtml = await page.$eval('.rc-message-user .rc-rich', (el) => el.innerHTML);
    await page.screenshot({ path: `${shots}/${colorScheme}-3-sent.png` });

    // Leptos leaves comment markers around keyed blocks whose placement
    // depends on the edits a view has been through; the elements are what
    // has to match.
    const markup = (html) => html.replace(/<!--.*?-->/g, '').replace(/ style=""/g, '');
    if (markup(previewHtml) !== markup(bubbleHtml)) {
      const a = markup(previewHtml), b = markup(bubbleHtml);
      let i = 0; while (i < a.length && a[i] === b[i]) i += 1;
      console.log(`     preview: ${JSON.stringify(a.slice(Math.max(0, i - 40), i + 120))}`);
      console.log(`     bubble:  ${JSON.stringify(b.slice(Math.max(0, i - 40), i + 120))}`);
    }
    check(markup(previewHtml) === markup(bubbleHtml), 'the bubble renders exactly what the preview showed');
    check((await page.$eval('.rc-composer-input', (el) => el.value)) === '', 'Enter clears the box');
    check(await page.$('.rc-composer-preview') === null, 'the preview goes away when the box is empty');
    check((await page.$$('.rc-message-user math')).length === 3, 'three equations in the bubble');
    check((await page.$$('.rc-message-user pre.rc-code span')).length > 20, 'the Rust block is highlighted');
    check(await page.$('.rc-message-user table') !== null, 'the table rendered');
    check(await page.$('.rc-message-user input[type=checkbox]') !== null, 'task list boxes rendered');
    check(await page.$('.rc-message-user blockquote.markdown-alert-tip') !== null, 'the alert rendered');
    check(await page.$('.rc-message-user .rc-footnotes') !== null, 'footnotes collected at the end');
    check((await page.$$('.rc-message-user a[href="https://leptos.dev"]')).length === 1, 'bare URL autolinked');

    const fonts = await page.evaluate(async () => {
      await document.fonts.ready;
      return [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family);
    });
    check(fonts.includes('Latin Modern Math'), 'the math font loaded');
    const mathFont = await page.$eval('.rc-message-user math', (el) => getComputedStyle(el).fontFamily);
    check(mathFont.startsWith('"Latin Modern Math"'), 'equations are set in Latin Modern Math');

    await page.click('.rc-message-user .rc-copy');
    check((await page.$eval('.rc-message-user .rc-copy', (el) => el.textContent)) === 'Copied', 'copy button acknowledges');

    await type(page, 'Shift+Enter keeps typing\nline two');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await page.$$('.rc-message')).length === 3, 'a second sent message appends');

    const background = await page.$eval('.rc-chat', (el) => getComputedStyle(el).backgroundColor);
    check(colorScheme === 'dark' ? background !== 'rgb(255, 255, 255)' : background === 'rgb(255, 255, 255)', `${colorScheme} palette applied`);

    // A host's plain rule beats the crate's layered ones, in both modes,
    // however specific the crate's dark-mode selectors are.
    await page.addStyleTag({ content: '.rc-chat { --rc-accent: rgb(1, 2, 3); }' });
    const accent = await page.$eval('.rc-send', (el) => getComputedStyle(el).backgroundColor);
    check(accent === 'rgb(1, 2, 3)', `a host override of --rc-accent wins in ${colorScheme} mode`);

    check(errors.length === 0, `console is clean${errors.length ? `: ${errors.join(' | ')}` : ''}`);
    await page.close();
  }

  // The app's controls, on a fresh page (a new context, so fresh storage).
  {
    const page = await browser.newPage({ colorScheme: 'light', viewport: { width: 900, height: 900 }, deviceScaleFactor: 2 });
    const errors = [];
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
    page.on('pageerror', (error) => errors.push(String(error)));
    console.log('\ncontrols');
    const theme = () => page.$eval('html', (el) => el.dataset.theme);
    const chatBackground = () => page.$eval('.rc-chat', (el) => getComputedStyle(el).backgroundColor);
    const bubbleStyle = (property) => page.$eval('.rc-message-user .rc-bubble', (el, property) => getComputedStyle(el)[property], property);
    const boxHeight = () => page.$eval('.rc-composer-input', (el) => el.getBoundingClientRect().height);
    const send = async (text) => {
      await type(page, text);
      await page.press('.rc-composer-input', 'Enter');
      await page.waitForTimeout(200);
    };

    await page.goto(origin, { waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');
    check(await page.$('.controls') !== null, 'the controls bar is there');

    // Theme.
    check((await theme()) === 'light', 'the theme starts as the system\'s');
    await page.click('.control-theme');
    await page.waitForTimeout(100);
    check((await theme()) === 'dark', 'the switch turns the theme dark');
    check((await chatBackground()) === 'rgb(13, 17, 23)', 'the chat repaints in the dark palette');
    check((await page.$eval('.control-theme', (el) => el.getAttribute('aria-checked'))) === 'true', 'the switch reports its state');

    // Side.
    await send('Which side?');
    await page.waitForSelector('.rc-message-user');
    check((await page.$eval('.rc-message-user', (el) => getComputedStyle(el).justifyContent)) === 'flex-end', 'bubbles start on the right');
    await page.click('.control-side[value="left"]');
    await page.waitForTimeout(100);
    check((await page.$eval('.rc-message-user', (el) => getComputedStyle(el).justifyContent)) === 'flex-start', 'the Left button moves them to the left');
    check((await bubbleStyle('borderBottomLeftRadius')) === '5px', 'the bubble\'s tail moves with it');

    // Colour.
    check((await page.$eval('.control-colour', (el) => el.value)) === '#172b45', 'the colour input shows the dark theme\'s bubble colour');
    check(await page.$eval('.control-reset', (el) => el.disabled), 'nothing to reset yet');
    await page.fill('.control-colour', '#ff8800');
    await page.waitForTimeout(100);
    check((await bubbleStyle('backgroundColor')) === 'rgb(255, 136, 0)', 'the bubble takes the chosen colour');
    check((await bubbleStyle('color')) === 'rgb(31, 35, 40)', 'and dark text, since orange is light');
    await type(page, 'Preview in colour');
    check((await page.$eval('.rc-composer-preview', (el) => getComputedStyle(el).backgroundColor)) === 'rgb(255, 136, 0)', 'the preview takes it too');
    await page.fill('.control-colour', '#123456');
    await page.waitForTimeout(100);
    check((await bubbleStyle('color')) === 'rgb(230, 237, 243)', 'a dark colour gets light text');
    await type(page, '');

    // The text box.
    const before = await boxHeight();
    const composer = await (await page.$('.rc-composer')).boundingBox();
    await page.mouse.move(composer.x + composer.width / 2, composer.y + 4);
    await page.mouse.down();
    await page.mouse.move(composer.x + composer.width / 2, composer.y + 4 - 150, { steps: 6 });
    check(await page.$eval('.app', (el) => el.classList.contains('app-resizing')), 'the app knows it is being resized');
    await page.mouse.up();
    await page.waitForTimeout(100);
    const after = await boxHeight();
    check(Math.round(after - before) === 150, `dragging the composer's top edge grows the text box by the same amount (${before} to ${after})`);
    await type(page, 'still typing');
    check((await boxHeight()) === after, 'typing keeps the dragged height');
    await type(page, '');
    await page.screenshot({ path: `${shots}/controls-0-changed.png` });

    // Everything survives a reload.
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');
    check((await theme()) === 'dark', 'the theme survives a reload');
    check((await page.$eval('.control-side[value="left"]', (el) => el.getAttribute('aria-pressed'))) === 'true', 'so does the side');
    check((await page.$eval('.control-colour', (el) => el.value)) === '#123456', 'and the colour');
    check((await boxHeight()) === after, 'and the text box height');

    // Reset puts the theme's colour back.
    await send('Reset me');
    await page.waitForSelector('.rc-message-user');
    await page.click('.control-reset');
    await page.waitForTimeout(100);
    check((await bubbleStyle('backgroundColor')) === 'rgb(23, 43, 69)', 'Reset returns the bubble to the theme\'s colour');
    check(await page.$eval('.control-reset', (el) => el.disabled), 'and has nothing more to do');
    await page.click('.control-theme');
    await page.waitForTimeout(100);
    check((await theme()) === 'light', 'the switch turns the theme light again');
    check((await bubbleStyle('backgroundColor')) === 'rgb(221, 244, 255)', 'and the bubble follows the theme');
    await page.screenshot({ path: `${shots}/controls-1-reset.png` });

    check(errors.length === 0, `console is clean${errors.length ? `: ${errors.join(' | ')}` : ''}`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}

console.log(failures === 0 ? '\nall checks passed' : `\n${failures} check(s) failed`);
process.exit(failures === 0 ? 0 : 1);
