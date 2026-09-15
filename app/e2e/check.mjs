// End-to-end check of the built app in headless Chromium.
//
// Serves a Trunk `dist` directory, opens it in light and dark mode, types
// Markdown, math, and code into the composer, and asserts what appears:
// the welcome bubble, the live preview, the sent bubble, fonts,
// highlighting, the copy button, progressive rendering of an unfinished
// equation, and a clean console.
// Screenshots land in ./screenshots for a human to look at.
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

    check(errors.length === 0, `console is clean${errors.length ? `: ${errors.join(' | ')}` : ''}`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}

console.log(failures === 0 ? '\nall checks passed' : `\n${failures} check(s) failed`);
process.exit(failures === 0 ? 0 : 1);
