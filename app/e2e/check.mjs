// End-to-end check of the built app in headless Chromium.
//
// Serves a Trunk `dist` directory, opens it in light and dark mode, types
// Markdown, math, and code into the composer, and asserts what appears:
// the welcome bubble, the live preview, the sent bubble, fonts,
// highlighting, the copy button, progressive rendering of an unfinished
// equation, collapsing the preview, and a clean console. Then it works
// the app's controls: the users, who send as whom, the selected user's
// bubble side and color changing every bubble of theirs, adding and
// deleting a user, the theme switch, dragging the text box taller, and
// that all of it survives a reload. Last, that the transcript keeps
// its end in view: through a burst of messages, a growing composer, and
// a shrinking window, but not for a reader who has scrolled up.
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

    // The window opens on a tour: Markdown and code from the assistant,
    // then the math from the user.
    const welcome = await page.$('.rc-message[data-kind="Assistant"]');
    check(welcome !== null, 'the welcome bubble is there on startup');
    check((await page.$$('.rc-message')).length === 2, 'and the math bubble after it');
    check(await page.$eval('.rc-message:last-child', (el) => el.dataset.kind) === 'User', 'from the user');
    const welcomeBox = await page.$eval('.rc-message[data-kind="Assistant"] .rc-bubble', (el) => el.getBoundingClientRect());
    const paneBox = await page.$eval('.rc-messages', (el) => el.getBoundingClientRect());
    check(welcomeBox.left - paneBox.left < paneBox.right - welcomeBox.right, 'the welcome bubble sits on the left');
    const mathBox = await page.$eval('.rc-message[data-kind="User"] .rc-bubble', (el) => el.getBoundingClientRect());
    check(mathBox.left - paneBox.left > paneBox.right - mathBox.right, 'the math bubble sits on the right');
    check((await page.$$('.rc-message[data-kind="Assistant"] .rc-codeblock')).length === 1, 'the welcome has one code block');
    check((await page.$$('.rc-message[data-kind="Assistant"] pre.rc-code span')).length > 20, 'which is highlighted');
    check(await page.$('.rc-message[data-kind="Assistant"] math') === null, 'and no math');
    check((await page.$$('.rc-message[data-kind="User"] math[display=block]')).length >= 3, 'the math bubble has display math');
    check(await page.$('.rc-message[data-kind="User"] merror') === null, 'every welcome equation parsed');
    check(await page.$('.rc-message[data-kind="Assistant"] table') !== null, 'the welcome has a table');
    await page.screenshot({ path: `${shots}/${colorScheme}-0-welcome.png` });

    // Progressive rendering: an unfinished equation with unbalanced braces.
    await type(page, 'Progressive: $$\\sum_{k=1}^n k^2 = \\frac{n(n+1)(2n+');
    check(await page.$('.rc-composer-preview math') !== null, 'unfinished $$ renders as math in the preview');
    await page.screenshot({ path: `${shots}/${colorScheme}-1-draft-math.png` });

    // The preview opens expanded; its button collapses it to the heading and back.
    check((await page.$eval('.rc-preview-toggle', (el) => el.getAttribute('aria-expanded'))) === 'true', 'the preview starts expanded');
    await page.click('.rc-preview-toggle');
    check(await page.$('.rc-composer-preview .rc-rich') === null, 'the collapse button hides the preview body');
    check(await page.$('.rc-composer-preview.rc-collapsed .rc-composer-preview-label') !== null, 'but keeps its heading');
    check((await page.$eval('.rc-preview-toggle', (el) => el.getAttribute('aria-expanded'))) === 'false', 'and says so');
    await page.screenshot({ path: `${shots}/${colorScheme}-1b-preview-collapsed.png` });
    await page.click('.rc-preview-toggle');
    check(await page.$('.rc-composer-preview math') !== null, 'and brings it back');

    // An open fence is a code block already.
    await type(page, 'Open fence:\n\n```python\nfor i in range(3):\n    print(i');
    check(await page.$('.rc-composer-preview .rc-codeblock') !== null, 'open fence renders as a code block');
    check((await page.$$('.rc-composer-preview .rc-codeblock span')).length > 0, 'open fence is highlighted');

    // The full sample: preview, then send.
    await type(page, sample);
    const previewHtml = await page.$eval('.rc-composer-preview .rc-rich', (el) => el.innerHTML);
    await page.screenshot({ path: `${shots}/${colorScheme}-2-preview.png` });
    await page.press('.rc-composer-input', 'Enter');
    // The sent message: the third, after the two of the tour.
    const sent = '.rc-message[data-message-id="m2"]';
    await page.waitForSelector(sent);
    await page.waitForTimeout(300);
    check(await page.$eval(sent, (el) => el.dataset.kind) === 'User', 'the sent message is from the selected user');
    // Captured before the screenshot: Playwright hides the caret for a
    // screenshot by touching inline styles on form controls, which leaves
    // an empty style attribute on the task-list checkboxes.
    const bubbleHtml = await page.$eval(`${sent} .rc-rich`, (el) => el.innerHTML);
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
    check((await page.$$(`${sent} math`)).length === 3, 'three equations in the bubble');
    check((await page.$$(`${sent} pre.rc-code span`)).length > 20, 'the Rust block is highlighted');
    check(await page.$(`${sent} table`) !== null, 'the table rendered');
    check(await page.$(`${sent} input[type=checkbox]`) !== null, 'task list boxes rendered');
    check(await page.$(`${sent} blockquote.markdown-alert-tip`) !== null, 'the alert rendered');
    check(await page.$(`${sent} .rc-footnotes`) !== null, 'footnotes collected at the end');
    check((await page.$$(`${sent} a[href="https://leptos.dev"]`)).length === 1, 'bare URL autolinked');

    const fonts = await page.evaluate(async () => {
      await document.fonts.ready;
      return [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family);
    });
    check(fonts.includes('Latin Modern Math'), 'the math font loaded');
    const mathFont = await page.$eval('.rc-message[data-kind="User"] math', (el) => getComputedStyle(el).fontFamily);
    check(mathFont.startsWith('"Latin Modern Math"'), 'equations are set in Latin Modern Math');

    await page.click('.rc-message[data-kind="User"] .rc-copy');
    check((await page.$eval('.rc-message[data-kind="User"] .rc-copy', (el) => el.textContent)) === 'Copied', 'copy button acknowledges');

    await type(page, 'Shift+Enter keeps typing\nline two');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await page.$$('.rc-message')).length === 4, 'a second sent message appends');

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
    const page = await browser.newPage({ colorScheme: 'light', viewport: { width: 1000, height: 900 }, deviceScaleFactor: 2 });
    const errors = [];
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
    page.on('pageerror', (error) => errors.push(String(error)));
    console.log('\ncontrols');
    const theme = () => page.$eval('html', (el) => el.dataset.theme);
    const chatBackground = () => page.$eval('.rc-chat', (el) => getComputedStyle(el).backgroundColor);
    // A style of every bubble of a kind; one value when they all agree.
    const bubbleStyle = async (kind, property) => {
      const values = await page.$$eval(`.rc-message[data-kind="${kind}"] .rc-bubble`, (els, property) => els.map((el) => getComputedStyle(el)[property]), property);
      return values.every((value) => value === values[0]) ? values[0] : values;
    };
    // The gaps between a kind's bubbles and the transcript's edges say
    // where they sit; the largest of each, so every bubble must agree.
    const gaps = async (kind) => {
      const all = await page.$$eval(`.rc-message[data-kind="${kind}"]`, (els) => els.map((row) => {
        const bubble = row.querySelector('.rc-bubble').getBoundingClientRect();
        const pane = row.getBoundingClientRect();
        return { left: bubble.left - pane.left, right: pane.right - bubble.right };
      }));
      return { left: Math.max(...all.map((g) => g.left)), right: Math.max(...all.map((g) => g.right)), count: all.length };
    };
    const users = () => page.$$eval('.control-users option', (els) => els.map((el) => el.value).filter(Boolean));
    const selected = () => page.$eval('.control-users', (el) => el.value);
    const pressedSide = () => page.$$eval('.control-side[aria-pressed="true"]', (els) => els.map((el) => el.value).join());
    const boxHeight = () => page.$eval('.rc-composer-input', (el) => el.getBoundingClientRect().height);
    const send = async (text) => {
      await type(page, text);
      await page.press('.rc-composer-input', 'Enter');
      await page.waitForTimeout(200);
    };

    await page.goto(origin, { waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');
    check(await page.$('.controls') !== null, 'the controls bar is there');
    check(await page.$eval('.controls', (el) => el.firstElementChild.contains(el.querySelector('.control-users'))), 'the users are at the far left');
    check(await page.$eval('.controls', (el) => el.lastElementChild.classList.contains('control-theme')), 'the theme switch is at the far right');
    const controlsBox = await (await page.$('.controls')).boundingBox();
    const themeBox = await (await page.$('.control-theme')).boundingBox();
    check(controlsBox.x + controlsBox.width - (themeBox.x + themeBox.width) < 24, 'and is drawn against the right edge');
    check(await page.$('.control-reset') === null, 'there is no Reset button');

    // Who is there, and who sends.
    check(JSON.stringify(await users()) === '["Assistant","User"]', 'the list starts with Assistant and User');
    check((await selected()) === 'User', 'and User is selected');
    check((await pressedSide()) === 'right', 'whose bubbles go on the right');
    check((await page.$eval('.control-color', (el) => el.value)) === '#2f855a', 'in green');
    await send('From the user');
    await page.waitForSelector('.rc-message[data-kind="User"]');
    check((await gaps('User')).right < 1, 'a sent message is from User, on the right');
    check((await bubbleStyle('User', 'backgroundColor')) === 'rgb(47, 133, 90)', 'in green');
    check((await gaps('Assistant')).left < 1, 'the welcome is from Assistant, on the left');
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(42, 100, 200)', 'in blue');
    await page.selectOption('.control-users', 'Assistant');
    await page.waitForTimeout(100);
    check((await pressedSide()) === 'left', 'selecting Assistant shows the assistant\'s side');
    check((await page.$eval('.control-color', (el) => el.value)) === '#2a64c8', 'and color');
    await send('From the assistant');
    check((await gaps('Assistant')).count === 2, 'and sends as the assistant');

    // Theme.
    check((await theme()) === 'light', 'the theme starts as the system\'s');
    await page.click('.control-theme');
    await page.waitForTimeout(100);
    check((await theme()) === 'dark', 'the switch turns the theme dark');
    check((await chatBackground()) === 'rgb(13, 17, 23)', 'the chat repaints in the dark palette');
    check((await page.$eval('.control-theme', (el) => el.getAttribute('aria-checked'))) === 'true', 'the switch reports its state');

    // Side: the selected user's, every bubble of theirs.
    check((await bubbleStyle('Assistant', 'borderBottomLeftRadius')) === '5px', 'the tail is at bottom left');
    await page.click('.control-side[value="right"]');
    await page.waitForTimeout(100);
    check((await gaps('Assistant')).right < 1, 'the Right button moves both of the assistant\'s bubbles to the right');
    check((await bubbleStyle('Assistant', 'borderBottomRightRadius')) === '5px', 'the tail moves with them');
    check((await bubbleStyle('Assistant', 'borderBottomLeftRadius')) === '16px', 'and leaves the left corner');
    check((await gaps('User')).right < 1, 'the user\'s bubble stays where it was');
    await page.click('.control-side[value="center"]');
    await page.waitForTimeout(100);
    const centered = await gaps('Assistant');
    check(centered.left > 1 && Math.abs(centered.left - centered.right) < 2, 'the Center button puts them in the middle');
    check((await bubbleStyle('Assistant', 'borderBottomLeftRadius')) === '16px', 'with no tail');

    // Color: the selected user's, every bubble of theirs.
    await page.fill('.control-color', '#ff8800');
    await page.waitForTimeout(100);
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(255, 136, 0)', 'both of the assistant\'s bubbles take the chosen color');
    check((await bubbleStyle('Assistant', 'color')) === 'rgb(31, 35, 40)', 'and dark text, since orange is light');
    check((await bubbleStyle('User', 'backgroundColor')) === 'rgb(47, 133, 90)', 'the user\'s bubble keeps its green');
    await type(page, 'Preview in color');
    check((await page.$eval('.rc-composer-preview', (el) => getComputedStyle(el).backgroundColor)) === 'rgb(255, 136, 0)', 'the preview takes it too');
    await page.fill('.control-color', '#123456');
    await page.waitForTimeout(100);
    check((await bubbleStyle('Assistant', 'color')) === 'rgb(230, 237, 243)', 'a dark color gets light text');
    await type(page, '');

    // Adding a user.
    check(await page.$eval('.control-add', (el) => el.disabled), 'nothing to add until a name is typed');
    await page.fill('.control-new-user', 'User');
    check(await page.$eval('.control-add', (el) => el.disabled), 'nor a name already in the list');
    await page.fill('.control-new-user', 'Alice');
    check(!(await page.$eval('.control-add', (el) => el.disabled)), 'a new name can be added');
    await page.press('.control-new-user', 'Enter');
    await page.waitForTimeout(100);
    check(JSON.stringify(await users()) === '["Assistant","User","Alice"]', 'Enter adds Alice to the list');
    check((await selected()) === 'Alice', 'and selects her');
    check((await page.$eval('.control-new-user', (el) => el.value)) === '', 'and clears the name');
    check((await pressedSide()) === 'left', 'a new user starts on the left');
    check((await page.$eval('.control-color', (el) => el.value)) === '#0e7490', 'in the next color of the palette');
    await send('From Alice');
    check((await gaps('Alice')).count === 1 && (await gaps('Alice')).left < 1, 'and sends as Alice, on the left');
    check((await bubbleStyle('Alice', 'backgroundColor')) === 'rgb(14, 116, 144)', 'in her color');
    await page.screenshot({ path: `${shots}/controls-0-three-users.png` });

    // Deleting one.
    await page.click('.control-delete');
    await page.waitForTimeout(100);
    check(JSON.stringify(await users()) === '["Assistant","User"]', 'Delete removes Alice from the list');
    check((await selected()) === '', 'and leaves nobody selected');
    check(await page.$eval('.control-delete', (el) => el.disabled), 'so there is nobody to delete');
    check(await page.$$eval('.control-side', (els) => els.every((el) => el.disabled)), 'the side buttons are disabled');
    check(await page.$eval('.control-color', (el) => el.disabled), 'so is the color');
    check(await page.$eval('.rc-composer-input', (el) => el.disabled), 'and the composer, with nobody to send as');
    check((await bubbleStyle('Alice', 'backgroundColor')) === 'rgb(14, 116, 144)', 'Alice\'s bubble keeps its color');
    check((await gaps('Alice')).left < 1, 'and its side');
    await page.selectOption('.control-users', 'User');
    await page.waitForTimeout(100);
    check(!(await page.$eval('.rc-composer-input', (el) => el.disabled)), 'selecting a user enables the composer again');

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
    await page.screenshot({ path: `${shots}/controls-1-changed.png` });

    // Everything survives a reload.
    await page.selectOption('.control-users', 'Assistant');
    await page.waitForTimeout(100);
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');
    check((await theme()) === 'dark', 'the theme survives a reload');
    check(JSON.stringify(await users()) === '["Assistant","User"]', 'so does the list');
    check((await selected()) === 'Assistant', 'and the selection');
    check((await pressedSide()) === 'center', 'and the assistant\'s side');
    check((await page.$eval('.control-color', (el) => el.value)) === '#123456', 'and color');
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(18, 52, 86)', 'which the welcome wears');
    check((await boxHeight()) === after, 'and the text box height');
    await page.click('.control-theme');
    await page.waitForTimeout(100);
    check((await theme()) === 'light', 'the switch turns the theme light again');
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(18, 52, 86)', 'and the bubble keeps its own color');

    check(errors.length === 0, `console is clean${errors.length ? `: ${errors.join(' | ')}` : ''}`);
    await page.close();
  }

  // The transcript keeps its end in view, unless the reader has left it.
  {
    const page = await browser.newPage({ colorScheme: 'light', viewport: { width: 900, height: 600 } });
    const errors = [];
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
    page.on('pageerror', (error) => errors.push(String(error)));
    console.log('\nfollowing');
    // How far the transcript is from its end, in CSS pixels.
    const gap = () => page.$eval('.rc-messages', (el) => el.scrollHeight - el.scrollTop - el.clientHeight);
    const lastBubbleVisible = () => page.evaluate(() => {
      const pane = document.querySelector('.rc-messages').getBoundingClientRect();
      const last = [...document.querySelectorAll('.rc-message')].at(-1).getBoundingClientRect();
      return last.bottom <= pane.bottom;
    });
    const send = async (text) => {
      await type(page, text);
      await page.press('.rc-composer-input', 'Enter');
      await page.waitForTimeout(200);
    };

    await page.goto(origin, { waitUntil: 'networkidle' });
    await page.waitForSelector('.rc-composer-input');
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(200);
    check((await gap()) === 0, 'the window opens at the end of the welcome, its fonts loaded');

    // Messages faster than frames: none may be lost.
    for (let i = 0; i < 5; i += 1) {
      await type(page, `burst ${i}`);
      await page.press('.rc-composer-input', 'Enter');
    }
    await page.waitForTimeout(300);
    check((await page.$$('.rc-message')).length === 7, 'a burst of five messages all arrive');
    check((await gap()) === 0, 'and the transcript is at the end after them');

    // The composer growing over the transcript, by a draft or by a drag.
    await type(page, Array.from({ length: 12 }, (_, i) => `line ${i}`).join('\n'));
    check(await lastBubbleVisible(), 'a long draft growing the composer leaves the last bubble showing');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await gap()) === 0, 'and sending it lands at the end');
    const composer = await (await page.$('.rc-composer')).boundingBox();
    await page.mouse.move(composer.x + composer.width / 2, composer.y + 4);
    await page.mouse.down();
    await page.mouse.move(composer.x + composer.width / 2, composer.y + 4 - 200, { steps: 6 });
    await page.mouse.up();
    await page.waitForTimeout(200);
    check((await gap()) === 0, 'dragging the text box taller keeps the end in view');
    check(await lastBubbleVisible(), 'with the last bubble showing');
    await page.setViewportSize({ width: 900, height: 420 });
    await page.waitForTimeout(200);
    check((await gap()) === 0, 'so does a shorter window');
    await page.screenshot({ path: `${shots}/following-0-composer-tall.png` });

    // A reader who scrolled up is reading; a new message must not pull
    // them away. Coming back to the end resumes following.
    await page.evaluate(() => { document.querySelector('.rc-messages').scrollTop = 0; });
    await page.waitForTimeout(100);
    await send('while the reader is up top');
    check((await page.$eval('.rc-messages', (el) => el.scrollTop)) === 0, 'a reader who scrolled up stays where they were');
    await page.evaluate(() => { const pane = document.querySelector('.rc-messages'); pane.scrollTop = pane.scrollHeight; });
    await page.waitForTimeout(100);
    await send('back at the end');
    check((await gap()) === 0, 'and is followed again once back at the end');

    check(errors.length === 0, `console is clean${errors.length ? `: ${errors.join(' | ')}` : ''}`);
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}

console.log(failures === 0 ? '\nall checks passed' : `\n${failures} check(s) failed`);
process.exit(failures === 0 ? 0 : 1);
