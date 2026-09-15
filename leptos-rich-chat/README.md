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

A message's content is a signal. To stream, hold an `RwSignal<String>`,
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

### Kinds of message

A message's `kind` is a name you choose. The crate attaches no meaning
to it: the bubble carries it as `data-kind`, and a `Kinds` table you
give `RichChatStyle` says where each kind sits and what colors it has.
The default table is `user` on the right in the theme's tint colors
and `assistant` on the left, which is a chat with a model. Any other
shape is another table:

```rust
use leptos_rich_chat::{Kinds, Look, Position};

let kinds = Kinds::none()
    .kind("me", Look::at(Position::Right).background("#ddf4ff").foreground("#1f2328"))
    .kind("alice", Look::at(Position::Left).background("var(--alice)"))
    .kind("bob", Look::at(Position::Left).background("var(--bob)"))
    .kind("notice", Look::at(Position::Center).background("transparent").foreground("var(--rc-muted)"));

view! { <RichChatStyle kinds=kinds /> }
```

The colors are CSS values, so `var(--…)` and `light-dark(…, …)` keep
light and dark mode in your stylesheet. A color left out is the theme's
plain bubble color, and a kind with no entry gets the plain bubble on
the left. `kinds` can be a signal, and only its rules are rewritten when
it changes. Position and the two colors are all a `Look` holds, because
they are what a stylesheet cannot say without knowing the kind's name;
anything else about a kind is a rule of yours against
`.rc-message[data-kind="…"]`.

The composer's preview takes the theme's tint colors, or those of the
kind named in `preview_kind`, so it can look like the bubble about to be
sent.

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

| Component       | Renders                                              |
|-----------------|------------------------------------------------------|
| `Composer`      | the text box, its collapsible preview, and send button |
| `MessageBubble` | one message, placed and colored by its kind         |
| `RichText`      | any Markdown, from a `Signal<String>`                |
| `CodeBlock`     | one highlighted block with label and copy button     |

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
so any rule in your own stylesheet wins over it regardless of
specificity, with no `!important` and no matching of the crate's
selectors:

- `style::STRUCTURE`, layer `rich-chat.structure`: what the components
  need to work. The transcript that scrolls, the text box that grows,
  the block wrappers that must not become boxes, and the alignment of
  MathML environments. No colors, fonts, spacing, or radii. Keep it.
- `style::THEME` and `style::HIGHLIGHT`, layer `rich-chat.theme`: the
  default look and the code colors. Neither names a kind of message;
  the rules for those are `Kinds::css()`, in the same layer.

To adjust the theme, override its custom properties on the component's
root. Every color is a `--rc-*` property, and so are the fonts
(`--rc-font`, `--rc-mono`, `--rc-math`), the size, and the bubble
radius:

```css
.rc-chat { --rc-accent: #7c3aed; --rc-tint-bg: #ede9fe; }
```

`--rc-bubble-bg` and `--rc-bubble-fg` are the plain bubble;
`--rc-tint-bg` and `--rc-tint-fg` the tinted one the default kinds give
`user` and the composer's preview; `--rc-tail` the radius of the corner
a bubble has on its side.

The theme sets them on the outermost root only (`.rc-chat`, or a
`.rc-rich` or `.rc-composer` used on its own), so an override there
reaches everything inside. It holds in dark mode too, which follows
`prefers-color-scheme` unless the document sets `data-theme="light"` or
`"dark"` on its root element.
To go further, restyle any `rc-*` class the same way; or leave the theme
out and write your own against the classes:

```rust
<RichChatStyle theme=false highlight=false />
```

#### Code colors

Highlighting is class-based: a fenced block comes out as `<span>`s with
`rc-keyword`, `rc-string`, `rc-comment`, and so on, and the colors come
from `style::HIGHLIGHT`, which is `assets/highlight.css`. That file is
not written by hand. It is the output of `examples/highlight_css`, a
small tool that takes one of [two-face](https://crates.io/crates/two-face)'s
embedded themes, has syntect print its rules against the crate's `rc-`
classes, and scopes every rule to one side of the light and dark switch
so both themes can live in one stylesheet. The shipped file is two runs
of it under a short header:

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
for the bare transcript; and the copy button's labels are a `CodeLabels`
provided as context, or a prop on `CodeBlock`.

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
