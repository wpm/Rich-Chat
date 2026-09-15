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
  class-based colours the stylesheet themes for light and dark. Each
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
use leptos_rich_chat::{Chat, Message, RichChatStyle, Role};

#[component]
fn App() -> impl IntoView {
    let messages = RwSignal::new(Vec::<Message>::new());
    let send = move |text: String| {
        let id = messages.read_untracked().len().to_string();
        messages.update(|all| all.push(Message::new(id, Role::User, text)));
    };
    view! {
        <RichChatStyle />
        <Chat messages=messages on_send=send />
    }
}
```

`Chat` fills its container: give the parent a height. `RichChatStyle`
injects the stylesheet and fonts once; place it anywhere.

### Streaming a reply

A message's content is a signal. To stream, hold an `RwSignal<String>`,
append to it as tokens arrive, and mark the message finished at the end:

```rust
let reply = RwSignal::new(String::new());
messages.update(|all| all.push(Message::streaming("r1", Role::Assistant, reply)));
// … on each token:
reply.update(|text| text.push_str(token));
// … when done:
messages.update(|all| {
    if let Some(last) = all.last_mut() { *last = last.clone().finished(); }
});
```

A live message is rendered as a draft, so an equation shows as math
before its closing `$$` has arrived.

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
| `Composer`      | the text box with its live preview and send button   |
| `MessageBubble` | one message on its role's side                       |
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

Every colour is a `--rc-*` custom property set on `.rc-chat`, `.rc-rich`,
and `.rc-composer`. Override them on an ancestor:

```css
.my-app .rc-chat { --rc-accent: #7c3aed; --rc-user-bg: #ede9fe; }
```

Dark mode follows `prefers-color-scheme` unless the document sets
`data-theme="light"` or `"dark"` on its root element, which wins. Code
colours are two-face's OneHalfLight and OneHalfDark;
`examples/theme_css.rs` prints the rules for any of its themes if you
want another:

```sh
cargo run -p leptos-rich-chat --example theme_css -- Dracula dark
```

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
