// End-to-end check of the built app in headless Chromium.
//
// Serves a Trunk `dist` directory, opens it in light and dark mode, types
// Markdown, math, and code into the composer, and asserts what appears:
// the four bubbles of the tour, the live preview, the sent bubble, fonts,
// highlighting, the copy button, progressive rendering of an unfinished
// equation, collapsing the preview, reaching the preview by key and
// coming back, and a clean console. Then it works
// the app's controls: the users, who send as whom, the selected user's
// bubble side and color changing every bubble of theirs, the names over
// the bubbles and the slider that sizes them, the Busy switch holding
// what is sent while the text box and the preview carry on, the Disabled
// switch turning the composer off and back with its draft and collapsed
// preview intact and the caret back in the box where the reader left it,
// adding and deleting a user, the theme switch, dragging the text box
// taller, the Width slider widening every user's wrapped bubbles at once
// up to the transcript's edges, and that all of it survives a reload.
// Last, that the transcript keeps its end in view:
// through a burst of messages, a growing composer, and
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

* bullets
  * nested
    * and nested again

1. numbers

+ [x] loose

+ [ ] tasks

> [!TIP]
> Alerts render too.

[^1]: A footnote.
`;

let failures = 0;
function check(condition, label) {
  console.log(`${condition ? 'ok  ' : 'FAIL'} ${label}`);
  if (!condition) failures += 1;
}

/** Whether the focus is on the element `selector` finds. */
const focusIs = (page, selector) => page.evaluate((selector) => document.activeElement === document.querySelector(selector), selector);

/** What is in the composer's text box. */
const boxText = (page) => page.$eval('.rc-composer-input', (el) => el.value);

/** Sets the composer's text the way typing does, through the input event. */
async function type(page, text) {
  await page.evaluate((text) => {
    const box = document.querySelector('.rc-composer-input');
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(box, text);
    box.dispatchEvent(new Event('input', { bubbles: true }));
  }, text);
  await page.waitForTimeout(150);
}

// The bubbles the window opens on: the introduction, then the tour of
// Markdown, code and math.
// Every count of messages below is this many plus what the test sent, and
// the sent messages' ids count on from it.
const TOUR = 4;

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

    // The window opens on a tour: an introduction from the assistant,
    // Markdown from the user, code from the assistant, then math from the
    // user.
    const welcome = '.rc-message[data-message-id="welcome"]';
    const markdown = '.rc-message[data-message-id="welcome-markdown"]';
    const code = '.rc-message[data-message-id="welcome-code"]';
    const math = '.rc-message[data-message-id="welcome-math"]';
    check(await page.$(welcome) !== null, 'the welcome bubble is there on startup');
    check((await page.$$('.rc-message')).length === TOUR, 'with the Markdown, code and math bubbles after it');
    check(await page.$eval(welcome, (el) => el.dataset.name) === 'Assistant', 'the welcome is from the assistant');
    check(await page.$eval(markdown, (el) => el.dataset.name) === 'User', 'the Markdown from the user');
    check(await page.$eval(code, (el) => el.dataset.name) === 'Assistant', 'the code from the assistant');
    check(await page.$eval(math, (el) => el.dataset.name) === 'User', 'and the math from the user');
    const welcomeBox = await page.$eval(`${welcome} .rc-bubble`, (el) => el.getBoundingClientRect());
    const paneBox = await page.$eval('.rc-messages', (el) => el.getBoundingClientRect());
    check(welcomeBox.left - paneBox.left < paneBox.right - welcomeBox.right, 'the welcome bubble sits on the left');
    const markdownBox = await page.$eval(`${markdown} .rc-bubble`, (el) => el.getBoundingClientRect());
    check(markdownBox.left - paneBox.left > paneBox.right - markdownBox.right, 'the Markdown bubble sits on the right');
    check(await page.$(`${welcome} h1`) !== null, 'the welcome has a heading');
    check(await page.$(`${welcome} table`) === null, 'and no table');
    check(await page.$(`${welcome} .rc-codeblock`) === null, 'no code block');
    check(await page.$(`${welcome} math`) === null, 'and no math');
    check(await page.$(`${markdown} table`) !== null, 'the Markdown bubble has a table');
    check(await page.$(`${markdown} .rc-codeblock`) === null, 'and no code block');
    check(await page.$(`${markdown} math`) === null, 'and no math');
    check((await page.$$(`${code} .rc-codeblock`)).length === 1, 'the code bubble has one code block');
    check((await page.$$(`${code} pre.rc-code span[class^="hl-"]`)).length > 20, 'which is highlighted');
    check(await page.$(`${code} math`) === null, 'and no math');
    check((await page.$$(`${math} math[display=block]`)).length >= 3, 'the math bubble has display math');
    check(await page.$(`${math} merror`) === null, 'every welcome equation parsed');
    check(await page.$(`${math} .rc-codeblock`) === null, 'and no code block');
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

    // The preview is the fair copy, and a reader who does not see the
    // screen goes to it by key: Alt+Shift+P in the box puts the focus in
    // the preview, a region named as its heading is, and Escape puts the
    // caret back where it was. The preview says nothing on its own; the
    // hint, which names the key, describes the box.
    const caret = () => page.$eval('.rc-composer-input', (el) => el.selectionStart);
    check((await page.$eval('.rc-composer-preview', (el) => [el.getAttribute('role'), el.getAttribute('aria-label'), el.getAttribute('tabindex'), el.hasAttribute('aria-live')].join())) === 'region,Preview,-1,false', 'the preview is a named region, focusable, and not live');
    const hintId = await page.$eval('.rc-composer-hint', (el) => el.id);
    check(hintId.startsWith('rc-composer-hint-') && (await page.$eval('.rc-composer-input', (el) => el.getAttribute('aria-describedby'))) === hintId, 'the hint describes the text box');
    check((await page.$eval('.rc-composer-hint', (el) => el.textContent)).includes('Alt+Shift+P'), 'and names the key');
    await page.focus('.rc-composer-input');
    await page.evaluate(() => document.querySelector('.rc-composer-input').setSelectionRange(5, 5));
    await page.keyboard.press('Alt+Shift+P');
    check(await focusIs(page, '.rc-composer-preview'), 'Alt+Shift+P in the box puts the focus in the preview');
    await page.keyboard.press('Escape');
    check(await focusIs(page, '.rc-composer-input'), 'Escape puts it back in the box');
    check((await caret()) === 5, 'with the caret where it was');
    await page.click('.rc-preview-toggle');
    await page.focus('.rc-composer-input');
    await page.keyboard.press('Alt+Shift+P');
    check(await focusIs(page, '.rc-composer-preview') && await page.$('.rc-composer-preview math') !== null, 'collapsed, the key expands the preview and goes to it');
    check((await page.$eval('.rc-preview-toggle', (el) => el.getAttribute('aria-expanded'))) === 'true', 'and the button says so');
    // The draft goes while the reader is in the preview (here by the
    // input event alone, which moves no focus, as a host clearing the
    // draft does): the preview goes, and the focus goes to the box.
    const draft = await boxText(page);
    await type(page, '');
    check(await page.$('.rc-composer-preview') === null && await focusIs(page, '.rc-composer-input'), 'the draft going takes the focus to the box, not the body');
    await page.keyboard.press('Alt+Shift+P');
    check(await focusIs(page, '.rc-composer-input') && (await boxText(page)) === '', 'with no draft the key does nothing');
    await type(page, draft);

    // An open fence is a code block already.
    await type(page, 'Open fence:\n\n```python\nfor i in range(3):\n    print(i');
    check(await page.$('.rc-composer-preview .rc-codeblock') !== null, 'open fence renders as a code block');
    check((await page.$$('.rc-composer-preview .rc-codeblock span')).length > 0, 'open fence is highlighted');

    // The full sample: preview, then send.
    await type(page, sample);
    const previewHtml = await page.$eval('.rc-composer-preview .rc-rich', (el) => el.innerHTML);
    await page.screenshot({ path: `${shots}/${colorScheme}-2-preview.png` });
    await page.press('.rc-composer-input', 'Enter');
    // The sent message: the one after the tour, whose id counts from it.
    const sent = `.rc-message[data-message-id="m${TOUR}"]`;
    await page.waitForSelector(sent);
    await page.waitForTimeout(300);
    check(await page.$eval(sent, (el) => el.dataset.name) === 'User', 'the sent message is from the selected user');
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
    check((await boxText(page)) === '', 'Enter clears the box');
    check(await page.$('.rc-composer-preview') === null, 'the preview goes away when the box is empty');
    check((await page.$$(`${sent} math`)).length === 3, 'three equations in the bubble');
    check((await page.$$(`${sent} pre.rc-code span[class^="hl-"]`)).length > 20, 'the Rust block is highlighted');
    check(await page.$(`${sent} table`) !== null, 'the table rendered');
    check(await page.$(`${sent} input[type=checkbox]`) !== null, 'task list boxes rendered');

    // The loose task list, whose items blank lines separate, puts each
    // checkbox in a paragraph rather than directly in the item.
    check((await page.$$(`${sent} li > p:first-child > input[type=checkbox]`)).length === 2, 'the loose task list boxes are in paragraphs');

    // List markers are the theme's own, so they outlast a host's reset in
    // a layer beneath the crate's: Tailwind's preflight, say.
    const markers = () => page.$$eval(`${sent} .rc-rich li:not(.rc-footnotes li)`, (items) => items.map((li) => getComputedStyle(li).listStyleType).join(' '));
    const expected = 'none none disc circle square decimal none none';
    check((await markers()) === expected, 'bullets, nested bullets and numbers have markers, task lists tight and loose none');
    await page.evaluate(() => {
      const reset = document.createElement('style');
      reset.id = 'host-reset';
      reset.textContent = '@layer base { ol, ul, menu, li { list-style: none; } }';
      document.head.prepend(reset);
    });
    check((await markers()) === expected, "and keep them under a host's reset");
    await page.evaluate(() => document.getElementById('host-reset').remove());

    check(await page.$(`${sent} blockquote.markdown-alert-tip`) !== null, 'the alert rendered');
    check(await page.$(`${sent} .rc-footnotes`) !== null, 'footnotes collected at the end');
    check((await page.$$(`${sent} a[href="https://leptos.dev"]`)).length === 1, 'bare URL autolinked');

    const fonts = await page.evaluate(async () => {
      await document.fonts.ready;
      return [...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family);
    });
    check(fonts.includes('Latin Modern Math'), 'the math font loaded');
    const mathFont = await page.$eval(`${math} math`, (el) => getComputedStyle(el).fontFamily);
    check(mathFont.startsWith('"Latin Modern Math"'), 'equations are set in Latin Modern Math');

    await page.click(`${code} .rc-copy`);
    check((await page.$eval(`${code} .rc-copy`, (el) => el.textContent)) === 'Copied', 'copy button acknowledges');

    await type(page, 'Shift+Enter keeps typing\nline two');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await page.$$('.rc-message')).length === TOUR + 2, 'a second sent message appends');

    // The button sends too. Clicking it takes the focus out of the box;
    // the send puts it back, ready for the next message.
    await type(page, 'By the button');
    await page.click('.rc-send');
    await page.waitForTimeout(200);
    check((await page.$$('.rc-message')).length === TOUR + 3, 'the Send button sends');
    check(await focusIs(page, '.rc-composer-input'), 'and the caret is back in the box');

    // A send while the reader is in the preview: the preview goes, and
    // the focus goes to the box with it rather than to the body. The
    // button is pressed by script, which moves no focus, as a click does.
    await type(page, 'Sent from the preview');
    await page.focus('.rc-composer-input');
    await page.keyboard.press('Alt+Shift+P');
    check(await focusIs(page, '.rc-composer-preview'), 'the reader is in the preview');
    await page.$eval('.rc-send', (el) => el.click());
    await page.waitForTimeout(200);
    check((await page.$$('.rc-message')).length === TOUR + 4, 'a send from there sends');
    check(await page.$('.rc-composer-preview') === null && await focusIs(page, '.rc-composer-input'), 'and the focus lands in the box, not the body');

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
    // A style of every bubble of a name; one value when they all agree.
    const bubbleStyle = async (name, property) => {
      const values = await page.$$eval(`.rc-message[data-name="${name}"] .rc-bubble`, (els, property) => els.map((el) => getComputedStyle(el)[property]), property);
      return values.every((value) => value === values[0]) ? values[0] : values;
    };
    // The gaps between the bubbles of the rows `selector` finds and the
    // transcript's edges (a row spans it, inside its padding) say where
    // they sit; the largest of each, so every bubble must agree.
    const rowGaps = async (selector) => {
      const all = await page.$$eval(selector, (els) => els.map((row) => {
        const bubble = row.querySelector('.rc-bubble').getBoundingClientRect();
        const pane = row.getBoundingClientRect();
        return { left: bubble.left - pane.left, right: pane.right - bubble.right };
      }));
      return { left: Math.max(...all.map((g) => g.left)), right: Math.max(...all.map((g) => g.right)), count: all.length };
    };
    const gaps = (name) => rowGaps(`.rc-message[data-name="${name}"]`);
    const users = () => page.$$eval('.control-users option', (els) => els.map((el) => el.value));
    const selected = () => page.$eval('.control-users', (el) => el.value);
    const pressedSide = () => page.$$eval('.control-side[aria-pressed="true"]', (els) => els.map((el) => el.value).join());
    const namesOn = () => page.$eval('.control-names', (el) => el.getAttribute('aria-checked') === 'true');
    // The slider that sizes the names: whether it can be moved, where it
    // is, and what a screen reader hears of that.
    const sliderOff = () => page.$eval('.control-sender-size', (el) => el.disabled);
    const sliderAt = () => page.$eval('.control-sender-size', (el) => el.value);
    const sliderSays = () => page.$eval('.control-sender-size', (el) => el.getAttribute('aria-valuetext'));
    const slide = async (to) => {
      await page.fill('.control-sender-size', to);
      await page.waitForTimeout(100);
    };
    // The name written over each bubble, null where there is none: its
    // text against the name the bubble carries, and where it sits.
    const labels = () => page.$$eval('.rc-message', (rows) => rows.map((row) => {
      const label = row.querySelector('.rc-sender');
      if (!label) return null;
      const name = label.getBoundingClientRect();
      const bubble = row.querySelector('.rc-bubble').getBoundingClientRect();
      const style = getComputedStyle(label);
      return {
        text: label.textContent, carried: row.dataset.name,
        above: name.bottom <= bubble.top, inside: name.left >= bubble.left - 1 && name.right <= bubble.right + 1,
        leftGap: name.left - bubble.left, rightGap: bubble.right - name.right,
        color: style.color, size: parseFloat(style.fontSize),
      };
    }));
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
    await page.waitForSelector('.rc-message[data-name="User"]');
    check((await gaps('User')).right < 1, 'a sent message is from User, on the right');
    check((await bubbleStyle('User', 'backgroundColor')) === 'rgb(47, 133, 90)', 'in green');
    check((await gaps('Assistant')).left < 1, 'the welcome and the code are from Assistant, on the left');
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(42, 100, 200)', 'in blue');
    await page.selectOption('.control-users', 'Assistant');
    await page.waitForTimeout(100);
    check((await pressedSide()) === 'left', 'selecting Assistant shows the assistant\'s side');
    check((await page.$eval('.control-color', (el) => el.value)) === '#2a64c8', 'and color');
    const assistantBubbles = (await gaps('Assistant')).count;
    await send('From the assistant');
    check((await gaps('Assistant')).count === assistantBubbles + 1, 'and sends as the assistant');

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
    check((await gaps('Assistant')).right < 1, 'the Right button moves all three of the assistant\'s bubbles to the right');
    check((await bubbleStyle('Assistant', 'borderBottomRightRadius')) === '5px', 'the tail moves with them');
    check((await bubbleStyle('Assistant', 'borderBottomLeftRadius')) === '16px', 'and leaves the left corner');
    check((await gaps('User')).right < 1, 'the user\'s bubbles stay where they were');
    await page.click('.control-side[value="center"]');
    await page.waitForTimeout(100);
    const centered = await gaps('Assistant');
    check(centered.left > 1 && Math.abs(centered.left - centered.right) < 2, 'the Center button puts them in the middle');
    check((await bubbleStyle('Assistant', 'borderBottomLeftRadius')) === '16px', 'with no tail');

    // Color: the selected user's, every bubble of theirs.
    await page.fill('.control-color', '#ff8800');
    await page.waitForTimeout(100);
    check((await bubbleStyle('Assistant', 'backgroundColor')) === 'rgb(255, 136, 0)', 'all of the assistant\'s bubbles take the chosen color');
    check((await bubbleStyle('Assistant', 'color')) === 'rgb(31, 35, 40)', 'and dark text, since orange is light');
    check((await bubbleStyle('User', 'backgroundColor')) === 'rgb(47, 133, 90)', 'the user\'s bubbles keep their green');
    await type(page, 'Preview in color');
    check((await page.$eval('.rc-composer-preview', (el) => getComputedStyle(el).backgroundColor)) === 'rgb(255, 136, 0)', 'the preview takes it too');
    await page.fill('.control-color', '#123456');
    await page.waitForTimeout(100);
    check((await bubbleStyle('Assistant', 'color')) === 'rgb(230, 237, 243)', 'a dark color gets light text');
    await type(page, '');

    // Names over the bubbles: off to begin with, then on for every bubble
    // at once, each on its bubble's side.
    check(!(await namesOn()), 'the bubbles start unnamed');
    check((await page.$$('.rc-sender')).length === 0, 'and carry no name');
    check(await sliderOff(), 'and the slider that sizes the names is off with them');
    check((await sliderAt()) === '0.95', 'at its default');
    await page.click('.control-names');
    await page.waitForTimeout(100);
    check(await namesOn(), 'the Names switch reports it is on');
    const named = await labels();
    check(named.length > 0 && named.every((label) => label !== null), 'and every bubble has a name over it');
    check(named.every((label) => label.text === label.carried), 'the name over each bubble is the one it carries');
    check(named.every((label) => label.above && label.inside), 'above the bubble, within its width');
    const assistantLabels = named.filter((label) => label.carried === 'Assistant');
    const userLabels = named.filter((label) => label.carried === 'User');
    check(assistantLabels.length > 0 && assistantLabels.every((label) => Math.abs(label.leftGap - label.rightGap) < 2), 'centered over a centered bubble');
    check(userLabels.length > 0 && userLabels.every((label) => label.rightGap < label.leftGap), 'at the right of a bubble on the right');
    const bubbleSize = await page.$eval('.rc-bubble', (el) => parseFloat(getComputedStyle(el).fontSize));
    // Whether every name over a bubble is `em` times the chat's text.
    const namesAre = (labels, em) => labels.length > 0 && labels.every((label) => Math.abs(label.size - em * bubbleSize) < 0.1);
    check(named.every((label) => label.size < bubbleSize), 'set smaller than the text');
    check(named.every((label) => label.color === named[0].color), 'in one color whatever the bubble\'s');
    await page.screenshot({ path: `${shots}/controls-0-names.png` });
    await page.click('.control-side[value="left"]');
    await page.waitForTimeout(100);
    check((await labels()).filter((label) => label.carried === 'Assistant').every((label) => label.leftGap < label.rightGap), 'moving the bubbles to the left takes the names with them');
    await page.click('.control-side[value="center"]');
    await page.waitForTimeout(100);
    await page.click('.control-names');
    await page.waitForTimeout(100);
    check(!(await namesOn()) && (await page.$$('.rc-sender')).length === 0, 'the switch takes the names off again');
    await page.click('.control-names');
    await page.waitForTimeout(100);
    check((await page.$$('.rc-sender')).length === (await page.$$('.rc-message')).length, 'and back on');

    // How big the names are: the slider beside the switch, live only while
    // the names are on. It starts just under the text and runs past it.
    check(!(await sliderOff()), 'the slider is live while the names are on');
    check((await sliderSays()) === '0.95 em', 'and a screen reader hears its default in em');
    check(namesAre(await labels(), 0.95), 'which is the size of the names against the text');
    check(await page.$eval('.control-sender-size', (el) => el.min === '0.7' && el.max === '1.4'), 'the slider runs from 0.7em to 1.4em');
    await slide('1.4');
    check((await sliderAt()) === '1.4' && (await sliderSays()) === '1.4 em', 'and goes to its top');
    const grown = await labels();
    check(namesAre(grown, 1.4), 'which grows every name past the text');
    check(grown.every((label) => label.above && label.inside), 'still above the bubble, within its width');
    await page.screenshot({ path: `${shots}/controls-0-names-large.png` });
    await page.click('.control-names');
    await page.waitForTimeout(100);
    check((await page.$$('.rc-sender')).length === 0, 'the switch takes the names off whatever their size');
    check(await sliderOff() && (await sliderAt()) === '1.4', 'and the slider goes off where it was left');
    await page.click('.control-names');
    await page.waitForTimeout(100);
    check(namesAre(await labels(), 1.4), 'and the names come back at that size');

    // Busy: a reply is in flight, so sending waits, and the reader goes on
    // writing the next message, preview and all.
    const busyOn = () => page.$eval('.control-busy', (el) => el.getAttribute('aria-checked') === 'true');
    const composerBusy = () => page.$eval('.rc-composer', (el) => el.classList.contains('rc-busy'));
    const messageCount = async () => (await page.$$('.rc-message')).length;
    check(!(await busyOn()) && !(await composerBusy()), 'the chat starts with nothing in flight');
    await page.click('.control-busy');
    await page.waitForTimeout(100);
    check(await busyOn(), 'the Busy switch reports it is on');
    check(await composerBusy(), 'and the composer carries rc-busy');
    check(!(await page.$eval('.rc-composer-input', (el) => el.disabled)), 'the text box stays open while busy');
    const beforeBusy = await messageCount();
    await page.click('.rc-composer-input');
    await page.keyboard.type('Written while **busy**');
    await page.waitForTimeout(150);
    check((await boxText(page)) === 'Written while **busy**', 'and takes what is typed into it');
    check(await page.$('.rc-composer-preview strong') !== null, 'which the preview renders');
    check(await page.$('.rc-preview-toggle') !== null, 'collapse toggle included');
    // The wait is spoken, not only shown: the send button is held rather
    // than disabled, and says why; a refused press fills the status line.
    const status = () => page.$eval('.rc-composer-status', (el) => el.textContent);
    const sendName = () => page.$eval('.rc-send', (el) => el.getAttribute('aria-label'));
    const sendHeld = () => page.$eval('.rc-send', (el) => !el.disabled && el.getAttribute('aria-disabled') === 'true');
    const sendOpen = () => page.$eval('.rc-send', (el) => !el.disabled && el.getAttribute('aria-disabled') !== 'true');
    check((await page.$eval('.rc-composer-status', (el) => el.getAttribute('role'))) === 'status', 'the composer has a status line');
    check((await status()) === '', 'which the wait beginning leaves empty');
    check(await sendHeld(), 'the send button is held, not disabled');
    check((await sendName()) === 'Send, Sending is paused', 'and is named for the wait');
    await page.focus('.rc-send');
    check(await page.evaluate(() => document.activeElement === document.querySelector('.rc-send')), 'and can be reached');
    // Pressed from the keyboard: Playwright's click refuses a button that
    // says aria-disabled, and a reader on the button presses Enter.
    await page.keyboard.press('Enter');
    await page.waitForTimeout(200);
    check((await messageCount()) === beforeBusy, 'pressing it sends nothing while busy');
    check((await status()) === 'Sending is paused', 'and the status line says why');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await messageCount()) === beforeBusy, 'Enter sends nothing while busy');
    check((await status()) === 'Sending is paused', 'the status line still says why');
    check((await boxText(page)) === 'Written while **busy**', 'and neither clears the draft nor breaks a line');
    await page.press('.rc-composer-input', 'Shift+Enter');
    await page.waitForTimeout(150);
    check((await boxText(page)) === 'Written while **busy**\n', 'Shift+Enter still breaks a line');
    await page.keyboard.type('and after');
    await page.waitForTimeout(150);
    await page.screenshot({ path: `${shots}/controls-0-busy.png` });
    await page.click('.control-busy');
    await page.waitForTimeout(200);
    check(!(await busyOn()) && !(await composerBusy()), 'the switch ends the wait');
    check((await messageCount()) === beforeBusy, 'which sends nothing on its own');
    check((await boxText(page)) === 'Written while **busy**\nand after', 'the draft is still in the box');
    check(await sendOpen(), 'and the send button is back');
    check((await sendName()) === 'Send', 'under its plain name');
    check((await status()) === '', 'and the status line is empty again');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await messageCount()) === beforeBusy + 1, 'Enter sends it now');
    check((await page.$eval('.rc-message:last-child .rc-bubble', (el) => el.textContent)).includes('and after'), 'as it was written');
    check((await boxText(page)) === '', 'and the box is empty again');

    // Disabled: the composer off. A draft already in the box is kept but
    // not previewed, and comes back as it was, the preview still collapsed.
    const disabledOn = () => page.$eval('.control-disabled', (el) => el.getAttribute('aria-checked') === 'true');
    await page.click('.rc-composer-input');
    await page.keyboard.type('Typed *before* the composer went off');
    await page.waitForTimeout(150);
    await page.click('.rc-preview-toggle');
    check(await page.$('.rc-composer-preview.rc-collapsed') !== null, 'a draft is previewed, and its preview collapsed');
    const beforeOff = await messageCount();
    await page.click('.control-disabled');
    await page.waitForTimeout(150);
    check(await disabledOn(), 'the Disabled switch reports it is on');
    check(await page.$eval('.rc-composer-input', (el) => el.disabled), 'the text box is disabled');
    check(await page.$('.rc-composer-preview') === null, 'and the preview goes, draft or no draft');
    check(await page.$eval('.rc-send', (el) => el.disabled), 'the send button is disabled');
    check(!(await composerBusy()), 'off is not waiting: no rc-busy');
    check((await boxText(page)) === 'Typed *before* the composer went off', 'the draft is kept');
    await page.screenshot({ path: `${shots}/controls-0-disabled.png` });
    await page.click('.control-disabled');
    await page.waitForTimeout(150);
    check(!(await disabledOn()) && !(await page.$eval('.rc-composer-input', (el) => el.disabled)), 'the switch turns the composer back on');
    check((await messageCount()) === beforeOff, 'which sends nothing on its own');
    check(await page.$('.rc-composer-preview.rc-collapsed') !== null, 'the preview is back, collapsed as it was left');
    await page.click('.rc-preview-toggle');
    check(await page.$('.rc-composer-preview em') !== null, 'and expands to the same draft');
    await page.press('.rc-composer-input', 'Enter');
    await page.waitForTimeout(200);
    check((await messageCount()) === beforeOff + 1, 'which Enter sends');

    // Off and back on, the reader keeps their place. The browser blurs a
    // control that becomes disabled and leaves the focus on the body; the
    // composer puts the caret back if the reader has not moved. The switch
    // is worked by script here, as a host works the signal: a click on it
    // would take the focus to the switch itself.
    const toggleDisabled = async () => {
      await page.$eval('.control-disabled', (el) => el.click());
      await page.waitForTimeout(150);
    };
    const caret = () => page.$eval('.rc-composer-input', (el) => `${el.selectionStart}:${el.selectionEnd}`);
    await page.click('.rc-composer-input');
    await page.keyboard.type('Typing when the composer went off');
    const middle = 'Typing when the composer '.length;
    await page.$eval('.rc-composer-input', (el, at) => el.setSelectionRange(at, at), middle);
    const place = await caret();
    check(place === `${middle}:${middle}`, 'the caret is in the middle of the draft');
    await toggleDisabled();
    check(await disabledOn() && await focusIs(page, 'body'), 'off, the browser drops the focus on the body');
    await toggleDisabled();
    check(await focusIs(page, '.rc-composer-input'), 'on again, the caret is back in the box');
    check((await caret()) === place, 'where it was');
    await toggleDisabled();
    await page.click('.control-new-user');
    check(await focusIs(page, '.control-new-user'), 'a reader can go elsewhere while it is off');
    await toggleDisabled();
    check(await focusIs(page, '.control-new-user'), 'and stays there when it comes back on');
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
    check((await labels()).at(-1)?.text === 'Alice', 'with her name over it');
    await page.screenshot({ path: `${shots}/controls-0-three-users.png` });

    // Deleting one.
    await page.click('.control-delete');
    await page.waitForTimeout(100);
    check(JSON.stringify(await users()) === '["Assistant","User"]', 'Delete removes Alice from the list');
    check((await selected()) === 'User', 'and selects the user before her');
    check((await page.$('.control-users option[value=""]')) === null, 'there is no Nobody to select');
    check((await pressedSide()) === 'right', 'whose side the buttons show');
    check((await page.$eval('.control-color', (el) => el.value)) === '#2f855a', 'and color');
    check(!(await page.$eval('.rc-composer-input', (el) => el.disabled)), 'and the composer stays enabled, with someone to send as');
    check((await bubbleStyle('Alice', 'backgroundColor')) === 'rgb(14, 116, 144)', 'Alice\'s bubble keeps its color');
    check((await gaps('Alice')).left < 1, 'and its side');
    await page.click('.control-delete');
    await page.waitForTimeout(100);
    check(JSON.stringify(await users()) === '["Assistant"]', 'deleting User leaves the Assistant');
    check((await selected()) === 'Assistant', 'selected, there being no one before');
    check(await page.$eval('.control-delete', (el) => el.disabled), 'and the last user cannot be deleted');
    await page.fill('.control-new-user', 'User');
    await page.press('.control-new-user', 'Enter');
    await page.waitForTimeout(100);
    check(JSON.stringify(await users()) === '["Assistant","User"]', 'User is added back');
    check(!(await page.$eval('.control-delete', (el) => el.disabled)), 'and Delete works again');

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

    // The width: the widest a message gets, for every user at once. A
    // bubble that wraps is exactly as wide as its maximum, so the wrapped
    // bubbles of the tour, the assistant's welcome and the user's
    // Markdown, are the measure of it; a message that fits on one line
    // is as wide as its text and must not move.
    const width = () => page.$eval('.control-width', (el) => el.value);
    const readout = () => page.$eval('.control-width-readout', (el) => el.textContent.trim());
    const spoken = () => page.$eval('.control-width', (el) => el.getAttribute('aria-valuetext'));
    const setWidth = async (value) => {
      await page.fill('.control-width', value);
      await page.waitForTimeout(100);
    };
    const bubbleWidth = (id) => page.$eval(`.rc-message[data-message-id="${id}"] .rc-bubble`, (el) => el.getBoundingClientRect().width);
    const gapsOf = (id) => rowGaps(`.rc-message[data-message-id="${id}"]`);
    const wrapped = ['welcome', 'welcome-markdown'];
    const oneLine = `m${TOUR}`;
    const track = await page.$eval('.control-width', (el) => ({ type: el.type, min: el.min, max: el.max, step: el.step }));
    check(track.type === 'range' && track.min === '76' && track.max === '161' && track.step === '1', 'the width is a slider from 76 to one past 160');
    // The track's last position is Full, and the one before it the widest measure.
    const fullPosition = track.max;
    const ceiling = String(Number(track.max) - 1);
    check((await width()) === '76' && (await readout()) === '76' && (await spoken()) === '76 characters', 'which starts at 76, and says so in words');
    check((await page.$eval('.control-width', (el) => el.labels[0]?.textContent)) === 'Width', 'labelled Width');
    const order = await page.$$eval('.controls > *', (els) => els.map((el) => (el.querySelector('.control-color') ? 'color' : el.querySelector('.control-width') ? 'width' : el.querySelector('.control-names') ? 'names' : null)).filter(Boolean).join());
    check(order === 'color,width,names', 'between the color and the Names switch');
    const atFloor = await Promise.all(wrapped.map(bubbleWidth));
    const shortAtFloor = await bubbleWidth(oneLine);
    check(shortAtFloor < atFloor[0] && atFloor[0] === atFloor[1], 'the wrapped bubbles of both users are one width, and a one-line message is narrower');
    // With the app's token taken off, the library's own maximum shows:
    // the same, since the slider starts where the library does.
    await page.evaluate(() => document.querySelector('.app').style.removeProperty('--rc-bubble-max-width'));
    check((await bubbleWidth('welcome')) === atFloor[0], 'at 76 a wrapped bubble is exactly as wide as the library alone makes it');
    await setWidth('120');
    check((await width()) === '120' && (await readout()) === '120' && (await spoken()) === '120 characters', 'dragging the slider to 120 says so');
    const atMiddle = await Promise.all(wrapped.map(bubbleWidth));
    check(atMiddle.every((now, i) => now > atFloor[i]), `and widens the wrapped bubbles of both users (${atFloor} to ${atMiddle})`);
    check((await bubbleWidth(oneLine)) === shortAtFloor, 'while the message that fits on a line does not move');
    await setWidth(fullPosition);
    check((await width()) === fullPosition && (await readout()) === 'Full' && (await spoken()) === 'Full width', 'one past 160 is Full');
    const full = await gapsOf('welcome');
    check(full.left < 1 && full.right < 1, 'at which a long message spans the transcript from one padding edge to the other');
    check((await bubbleWidth('welcome')) > atMiddle[0], 'wider than any measure');
    check((await bubbleWidth(oneLine)) === shortAtFloor, 'and the one-line message still does not move');
    await page.screenshot({ path: `${shots}/controls-1-full-width.png` });
    // The floor: the slider cannot go below the library's measure.
    await page.focus('.control-width');
    await page.keyboard.press('Home');
    await page.waitForTimeout(100);
    check((await width()) === '76' && (await bubbleWidth('welcome')) === atFloor[0], 'Home takes it back to 76');
    await page.keyboard.press('ArrowLeft');
    await page.waitForTimeout(100);
    check((await width()) === '76' && (await readout()) === '76', 'and it cannot be dragged below 76');
    // In a window narrow enough for the gutter to bind, every measure
    // looks like the first: only Full lifts the gutter.
    await page.setViewportSize({ width: 600, height: 900 });
    await page.waitForTimeout(100);
    const narrowAtFloor = await bubbleWidth('welcome');
    const narrowGutter = await gapsOf('welcome');
    check(narrowGutter.left + narrowGutter.right > 1, 'a narrow window keeps a gutter at 76');
    await setWidth(ceiling);
    check((await bubbleWidth('welcome')) === narrowAtFloor, 'and looks the same at 160');
    await setWidth(fullPosition);
    const narrowFull = await gapsOf('welcome');
    check(narrowFull.left < 1 && narrowFull.right < 1, 'and Full still spans it');
    await page.setViewportSize({ width: 1000, height: 900 });
    await setWidth('120');
    check((await bubbleWidth('welcome')) === atMiddle[0], 'back at 120 in the wide window, the bubble is as it was');
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
    check((await width()) === '120' && (await readout()) === '120', 'and the width');
    check((await bubbleWidth('welcome')) === atMiddle[0], 'which the welcome is wrapped at');
    check(await namesOn(), 'and the names');
    check((await page.$$('.rc-sender')).length === (await page.$$('.rc-message')).length, 'which every bubble still has');
    check((await sliderAt()) === '1.4', 'and their size');
    check(namesAre(await labels(), 1.4), 'which they are written at');
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
    check((await page.$$('.rc-message')).length === TOUR + 5, 'a burst of five messages all arrive');
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
