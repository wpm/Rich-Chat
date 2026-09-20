# leptos-rich-chat

Chat components for [Leptos](https://leptos.dev) that render Markdown,
LaTeX math, and syntax-highlighted code, progressively, as the text
arrives.

- **Markdown**: CommonMark plus GitHub's tables, task lists,
  strikethrough, footnotes, alerts, and bare-URL autolinks, through
  [pulldown-cmark](https://crates.io/crates/pulldown-cmark). Raw HTML is
  shown as text; links keep only `http`, `https`, and `mailto`
  destinations, so a message cannot inject markup.
- **Math**: `$…$` inline and `$$…$$` display, rendered to MathML by
  [pulldown-latex](https://crates.io/crates/pulldown-latex) and set in
  Latin Modern Math, the TeX font. No JavaScript, no KaTeX, no MathJax.
- **Code**: fenced blocks in some two hundred languages, highlighted by
  [syntect](https://crates.io/crates/syntect) with
  [two-face](https://crates.io/crates/two-face)'s grammar set, in
  class-based colors the stylesheet themes for light and dark. Each
  block has a language label and a copy button. Grammars compile on
  first use; `Chat` compiles the common ones in idle time after it
  mounts, and `warm_up()` does the same for a host that uses the pieces
  on their own.
- **Progressive**: text renders block by block with stable keys, so as a
  draft is typed or a streamed message grows only the block being changed
  is touched. Constructs left open at the end of a draft (a `$$` with no
  closer, a half-typed `` `code` `` span, an unbalanced `\frac{`) are closed
  for display, so the preview shows what the text is becoming instead of
  flickering between math and dollar signs.

## Use

```toml
[dependencies]
leptos = { version = "0.8", features = ["csr"] }
leptos-rich-chat = "0.1"
```

```rust
use leptos::prelude::*;
use leptos_rich_chat::{Chat, Message, RichChatStyle};

#[component]
fn App() -> impl IntoView {
    let messages = RwSignal::new(Vec::<Message>::new());
    let send = move |text: String| {
        let id = messages.read_untracked().len().to_string();
        messages.update(|all| all.push(Message::new(id, "user", text)));
    };
    view! {
        <RichChatStyle />
        <Chat messages=messages on_send=send />
    }
}
```

`Chat` fills its container: give the parent a height. `RichChatStyle`
injects the stylesheets and fonts once; place it anywhere.

### Streaming a reply

A text body is a signal. To stream, hold an `RwSignal<String>`,
append to it as tokens arrive, and mark the message finished at the end:

```rust
let reply = RwSignal::new(String::new());
messages.update(|all| all.push(Message::streaming("r1", "assistant", reply)));
// … on each token:
reply.update(|text| text.push_str(token));
// … when done:
messages.update(|all| {
    if let Some(last) = all.last_mut() { *last = last.clone().finished(); }
});
```

A live message is rendered as a draft, so an equation shows as math
before its closing `$$` has arrived.

### Bodies

A message's `body` is a `Body`: `Body::Text`, the Markdown above, which
the crate renders in a bubble; or `Body::View`, a view you draw. That is
where a transcript puts what is not speech, a tool call, a failure, a
prompt the reader has to answer, without leaving the pane that scrolls:

```rust
messages.update(|all| {
    all.push(Message::view("t1", "tool", || {
        view! { <details class="tool-call"><summary>"Ran the tests"</summary><pre>"122 passed"</pre></details> }
    }))
});
```

The crate still places the message by its name, keys it, scrolls to it
and follows it as it grows; only what is inside is yours. The closure
runs once per message, when the transcript builds it, and again only if
its `id` or `live` changes, so a `clone()` inside it is paid once and
not on every render; where it captures more than a couple of small
fields, capture an `Arc` of them, as `Message::view`'s documentation
shows. Your view is
rendered *in place of* the bubble, as the child of `div.rc-message`, so
a body that is deliberately not speech does not start by undoing the
bubble's width, padding and background. To have the bubble around your
own body, give its root element `class="rc-bubble"`: the name's colors
and tail land on it.

### Names

A message's `name` is who it is from, and you choose it. The crate
attaches no meaning to it: the bubble carries it as `data-name`, and a
`Names` table you give `RichChatStyle` says where each name's bubbles
sit and what colors they have. The default table is `user` on the right
in the theme's tint colors and `assistant` on the left, which is a chat
with a model. Any other shape is another table:

```rust
use leptos_rich_chat::{Look, Names, Position};

let names = Names::none()
    .name("me", Look::at(Position::Right).background("#ddf4ff").foreground("#1f2328"))
    .name("alice", Look::at(Position::Left).background("var(--alice)"))
    .name("bob", Look::at(Position::Left).background("var(--bob)"))
    .name("notice", Look::at(Position::Center).background("transparent").foreground("var(--rc-muted)"));

view! { <RichChatStyle names=names /> }
```

The colors are CSS values, so `var(--…)` and `light-dark(…, …)` keep
light and dark mode in your stylesheet. A color left out is the theme's
plain bubble color, and a name with no entry gets the plain bubble on
the left. `names` can be a signal, and only its rules are rewritten when
it changes. Position and the two colors are all a `Look` holds, because
they are what a stylesheet cannot say without knowing the name;
anything else about a name is a rule of yours against
`.rc-message[data-name="…"]`.

The composer's preview takes the theme's tint colors, or those of the
name given as `preview_name`, so it can look like the bubble about to be
sent.

To write the name over every bubble, as a group chat does, give `Chat`
(or `MessageView`) `show_names=true`, or a `Signal<bool>` for a switch
the reader can flip:

```rust
view! { <Chat messages=messages on_send=send show_names=true /> }
```

The name goes in a `div.rc-sender` above the bubble, on the bubble's side,
small and in the theme's muted color so that it reads the same over a
bubble of any color. It is the name as spelled, so a chat that shows
names picks names meant to be read: `Alice`, not `alice`. A name that
is nobody, the `notice` above, keeps its label off with a rule of yours:
`.rc-message[data-name="notice"] > .rc-sender { display: none; }`.

### Links

By default links open in a new browsing context. In a Tauri window the
webview must not navigate, so intercept them:

```rust
view! { <Chat messages=messages on_send=send on_link=Callback::new(open_in_browser) /> }
```

The callback receives the destination; the default is suppressed. The
[test app](https://github.com/wpm/Rich-Chat/tree/main/app) hands it to
Tauri's opener plugin.

### Pieces

`Chat` is the whole window. Its parts stand alone:

| Component       | Renders                                                                          |
|-----------------|----------------------------------------------------------------------------------|
| `Composer`      | the text box, its collapsible preview, and send button; attributes reach the box |
| `MessageView`   | one message, placed and colored by its name, and named on request                |
| `RichText`      | any Markdown, from a `Signal<String>`                                            |
| `CodeBlock`     | one highlighted block with label and copy button                                 |

The `render` module underneath is plain Rust with no DOM dependency:
`render_html(markdown, &options)` gives sanitized HTML,
`render_blocks` the keyed blocks the components diff on, and
`complete_draft` the progressive closing of open constructs. It runs and
is tested natively.

`RenderOptions` switches math, highlighting, images, and smart
punctuation, and marks text as a draft.

### Styling

The components carry `rc-*` classes and no inline styles, so the look is
entirely CSS, and the CSS is yours to decide. What the crate ships comes
in two tiers, each in a [cascade layer](https://developer.mozilla.org/en-US/docs/Web/CSS/@layer),
so any unlayered rule in your own stylesheet wins over it regardless of
specificity, with no `!important` and no matching of the crate's
selectors:

- `style::STRUCTURE`, layer `rich-chat.structure`: what the components
  need to work. The transcript that scrolls, the text box that grows,
  the block wrappers that must not become boxes, and the alignment of
  MathML environments. No colors, fonts, spacing, or radii. Keep it.
- `style::THEME` and `style::HIGHLIGHT`, layer `rich-chat.theme`: the
  default look and the code colors. Neither names anyone; the rules
  for the names are `Names::css()`, in the same layer.

That cuts both ways: an unlayered global reset in your stylesheet, such
as Tailwind 3's preflight, reaches inside the chat too, and you put back
what you want by rule against the `rc-*` classes.

If your own rules are in cascade layers, the order the layers are first
declared in decides, and the crate's CSS is a `<style>` element in the
body, so its own declaration comes after anything in the head and would
put its layers last, over all of yours. Declare the order yourself,
first: the crate's layers go after any layer that resets elements and
before any layer whose rules should win over the theme. `rich-chat`
names the parent of `rich-chat.structure` and `rich-chat.theme`, so one
name places both.

For Tailwind v4 that is between its reset and its utilities:

```css
@layer theme, base, rich-chat, components, utilities;
@import "tailwindcss";
```

`base` comes first because Tailwind's preflight is in it. With `base`
before `rich-chat`, preflight loses to the theme and the Markdown is
styled. With `base` after `rich-chat`, preflight wins: it resets the
margins of paragraphs and lists, the list markers, the heading sizes,
and the `pre` and `code` fonts, and the Markdown comes out flat.
`utilities` comes after, so a utility class on a body you draw or on a
wrapper still wins.

To adjust the theme, override its custom properties anywhere above the
components. Every color is a `--rc-*` property, and so are the fonts
(`--rc-font`, `--rc-mono`, `--rc-math`), the size, and the bubble
radius:

```css
.rc-chat { --rc-accent: #7c3aed; --rc-tint-bg: #ede9fe; }
```

`--rc-bubble-bg` and `--rc-bubble-fg` are the plain bubble;
`--rc-tint-bg` and `--rc-tint-fg` the tinted one the default names give
`user` and the composer's preview; `--rc-tail` the radius of the corner
a bubble has on its side; `--rc-muted` the color of the name over a
bubble.

The theme declares them on the document root, `:root`, and no component
declares anything, so an override reaches the chat from wherever you put
it: on `.rc-chat` as above, on the wrapper your own chrome lives in, or
on `:root` alongside the rest of your palette. The nearest declaration
wins, so yours does. A token you take over that way is yours in both
themes: the theme's dark values are on `:root`, further away, so they no
longer reach it and the dark one is yours to give too. Otherwise dark
mode follows `prefers-color-scheme` unless the document sets
`data-theme="light"` or `"dark"` on its root element.
To go further, restyle any `rc-*` class the same way; or leave the theme
out and write your own against the classes:

```rust
<RichChatStyle theme=false highlight=false />
```

#### Code colors

Highlighting is class-based: a fenced block comes out as `<span>`s with
`hl-keyword`, `hl-string`, `hl-comment`, and so on, and the colors come
from `style::HIGHLIGHT`, which is `assets/highlight.css`. The prefix is
the highlighter's own, not the components' `rc-`, on purpose: every
dotted atom of a scope becomes a class, so a `meta.block` scope would
otherwise be `rc-block`, the wrapper around each Markdown block, and
`entity.name.function` would be `rc-name`, and a rule of yours against
a component's class would land on the code too. A test keeps the two
sets of classes apart. The file is not written by hand. It is the output
of `examples/highlight_css`, a small tool that takes one of
[two-face](https://crates.io/crates/two-face)'s embedded themes, has
syntect print its rules against the `hl-` classes, and scopes every rule
to one side of the light and dark switch so both themes can live in one
stylesheet. The shipped file is two runs of it under a short header:

```sh
cargo run -p leptos-rich-chat --example highlight_css -- OneHalfLight light
cargo run -p leptos-rich-chat --example highlight_css -- OneHalfDark dark
```

Run it with no arguments to list the themes. Use it to change the code
colors without touching anything else: turn the shipped rules off with
`<RichChatStyle highlight=false />`, then serve the tool's output for
the theme you want (one run per side, or one run for a single theme on
both sides) alongside your own CSS. The tool ships in the crate's
package, so it runs the same way from a copy of the source, whether a
checkout of this repository or the package in Cargo's registry cache.

The words in the interface are props: `Chat` and `Composer` take
`placeholder`, `hint` (empty leaves the line out), `preview_label`, and
`send`, the button's content, so it can be an icon; `Chat` takes `empty`
for the bare transcript and `show_names` for the names over the
bubbles; and the copy button's labels are a `CodeLabels` provided as
context, or a prop on `CodeBlock`. The text in the box is
the composer's own unless `draft`, an `RwSignal<String>` the host holds,
is given: to prefill it, read it, or keep it across unmounting.

`Chat` and `Composer` also take two signals that keep a message from
being sent, and they are not each other. `disabled` turns the composer
off, for a host with nothing to send with: the text box is disabled,
there is no preview, and nothing can be sent. `busy` is a wait, for a
host whose reply is still in flight: the text box and the preview carry
on, neither Enter nor the button sends, and `.rc-composer` has the class
`rc-busy` for a host that wants the wait to show. Either way the draft
is kept, and nothing is sent on its own when the state ends. Beside
`busy` goes `busy_label`, the words a screen reader gets for the wait,
"Sending is paused" unless the host says otherwise: while the wait alone
holds a draft back the send button is named `Send, {busy_label}` and
carries `aria-disabled` rather than `disabled`, so it stays in the tab
order, and a press that the wait refuses, Enter or the button, puts the
phrase in the composer's status line, a `role="status"` live region
that the structure stylesheet hides from sight. `rc-busy` is a hook for
the host's CSS and is not announced; a host showing a busy state of its
own gives it a live region of its own.

A host waiting between turns wants `busy`, which leaves the text box
open and the reader's caret in it. `disabled` is the composer off, and
turning it off takes the focus with it, as the browser does to any
control that becomes disabled; the composer restores it when it comes
back on, caret where it was, if the reader has not moved in the meantime.

The text box is set up for prose, with `spellcheck="true"`,
`autocapitalize="sentences"` and `autocorrect="on"`. Anything else it
should carry, an `id` for a label elsewhere on the page to point at, a
`maxlength`, a `data-*` attribute, is passed to `Composer` as an
attribute and lands on the box, where a value of yours for one of the
three above wins over the default:

```rust
view! { <Composer on_send=send attr:id="prompt" attr:maxlength="2000" attr:spellcheck="false" /> }
```

Attributes on `Chat` land on the window, `.rc-chat`, as Leptos puts a
component's attributes on the top of its view; a host that needs to
attribute the text box of a `Chat` composes one from `Composer`. Either
way `attr:class` replaces the element's class attribute, as on any
element, and `class:mine=true` adds to it.

## Features

- `highlight` (default): syntax highlighting. Adds about a megabyte of
  grammar tables to the binary; without it code blocks are plain.
- `bundled-fonts` (default): the Latin Modern fonts embedded as data
  URIs, about half a megabyte. Without it, serve `style::FONT_FILES`
  yourself and inject `style::font_faces_from("/fonts")`.

## Security

Messages are untrusted. Raw HTML becomes text, links are restricted to
`http`, `https`, and `mailto` and open with `rel="noopener noreferrer"`,
images to `http`, `https`, and `data:image`, and the LaTeX source that
rides along in each equation's `<annotation>` is escaped. Set
`RenderOptions { images: false, .. }` to keep remote images from
revealing a reader's address.

## Browser support

MathML Core: Chromium 109+, Firefox, Safari, and the webviews Tauri uses
on every desktop platform.

## License

MIT. The Latin Modern fonts are under the
[GUST Font License](https://www.gust.org.pl/projects/e-foundry/licenses).
