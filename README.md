# Rich-Chat

[![CI](https://github.com/wpm/Rich-Chat/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/Rich-Chat/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wpm/Rich-Chat/graph/badge.svg)](https://codecov.io/gh/wpm/Rich-Chat)

A chat interface with rich formatting: Markdown, LaTeX math, and
syntax-highlighted code in every language, rendered as you type.

The interface is the [`leptos-rich-chat`](leptos-rich-chat/) crate, a set
of [Leptos](https://leptos.dev) components published to crates.io. Its
[README](leptos-rich-chat/README.md) is the full guide; this page is the
short one. A live demo is at <https://wpm.github.io/Rich-Chat/>.

## Using the crate

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

`Chat` is the whole window, a scrolling transcript over a text box, and
fills its container, so give the parent a height. `RichChatStyle`
injects the stylesheets and fonts once; place it anywhere. A message is
a `Message`: `Message::new(id, name, markdown)` for finished text,
`Message::streaming(id, name, signal)` for text still arriving, which
renders as a draft while it grows, and `Message::view(id, name, view)`
for a body you draw yourself, in place of the bubble.

Everything about the crate is set in one of four places: props on
`Chat`, props on `RichChatStyle`, CSS custom properties, and Cargo
features.

### `Chat` props

| Prop            | Type                     | Default                                  | Controls                                                                                                 |
|-----------------|--------------------------|------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `messages`      | `Signal<Vec<Message>>`   | required                                 | The transcript, oldest first.                                                                            |
| `on_send`       | `Callback<String>`       | required                                 | Receives the text of each message sent.                                                                  |
| `draft`         | `RwSignal<String>`       | the composer's own                       | The text in the box, held by you: to prefill it, read it, or keep it across unmounting.                  |
| `placeholder`   | `String`                 | `"Write a message…"`                     | The text box's placeholder.                                                                              |
| `hint`          | `String`                 | explains the keys                        | The line under the text box. `""` leaves it out.                                                         |
| `preview`       | `bool`                   | `true`                                   | Whether the composer shows a live preview of the draft.                                                  |
| `preview_label` | `String`                 | `"Preview"`                              | The heading over the preview.                                                                            |
| `preview_name`  | `Signal<String>`         | `""`, the theme's tint colors            | Whose bubble the preview looks like, by name.                                                            |
| `send`          | view function            | the word "Send"                          | The send button's content, so it can be an icon.                                                         |
| `empty`         | `String`                 | nothing                                  | Shown in the transcript while it has no messages.                                                        |
| `show_names`    | `Signal<bool>`           | `false`                                  | Writes each message's name over its bubble, as a group chat does.                                        |
| `disabled`      | `Signal<bool>`           | `false`                                  | The composer off: the text box is disabled, there is no preview, nothing sends. The draft is kept.       |
| `busy`          | `Signal<bool>`           | `false`                                  | A reply in flight: the text box and preview carry on, but nothing sends until it clears. The draft is kept. |
| `busy_label`    | `String`                 | `"Sending is paused"`                    | What a screen reader is told while `busy` holds a send back.                                             |
| `options`       | `RenderOptions`          | everything on                            | What the Markdown renderer does; see below.                                                              |
| `on_link`       | `Callback<String>`       | links open in a new browsing context     | Receives a link's destination instead of following it. A Tauri window hands this to its opener plugin.   |
| `warm_up`       | `bool`                   | `true`                                   | Compiles the common languages' grammars in idle time after mount, so the first code block does not pause. |

Signal-typed props take a plain value or a signal, so `busy=true` and
`busy=is_waiting` both work. Attributes on `Chat` land on the window,
`.rc-chat`; a host that needs an attribute on the text box itself
composes the window from the pieces below.

`RenderOptions` is a plain struct; build it with `..Default::default()`:

| Field               | Default | Off                                                               |
|---------------------|---------|-------------------------------------------------------------------|
| `math`              | `true`  | `$…$` and `$$…$$` are text.                                       |
| `highlight`         | `true`  | Fenced code is plain. Also needs the `highlight` feature.         |
| `images`            | `true`  | Only the alt text shows. Off keeps remote images from revealing a reader's address. |
| `smart_punctuation` | `false` | On, straight quotes, `--` and `...` become typographic characters. |
| `draft`             | `false` | On, constructs left open at the end are closed for display.       |

### `RichChatStyle` props

| Prop        | Type            | Default              | Controls                                                                        |
|-------------|-----------------|----------------------|---------------------------------------------------------------------------------|
| `theme`     | `bool`          | `true`               | The default look: colors, fonts, spacing, radii. Off, you style the `rc-*` classes yourself. |
| `highlight` | `bool`          | `true`               | The code colors. Off, serve your own; the crate's `highlight_css` example generates one from any syntect theme. |
| `fonts`     | `bool`          | `true`               | The `@font-face` rules for the bundled math fonts.                              |
| `names`     | `Signal<Names>` | `user` right, `assistant` left | Where each name's bubbles sit and what colors they have.              |

A `Names` table gives every name a `Look`: a `Position` (`Left`,
`Center`, `Right`) and, optionally, a background and a text color, both
as CSS values, so `var(--…)` and `light-dark(…, …)` work:

```rust
use leptos_rich_chat::{Look, Names, Position};

let names = Names::none()
    .name("me", Look::at(Position::Right).background("#ddf4ff"))
    .name("alice", Look::at(Position::Left).background("var(--alice)"))
    .name("notice", Look::at(Position::Center).background("transparent").foreground("var(--rc-muted)"));

view! { <RichChatStyle names=names /> }
```

A name with no entry gets the plain bubble on the left.

The crate's stylesheets are in [cascade layers](https://developer.mozilla.org/en-US/docs/Web/CSS/@layer),
so any unlayered rule in your own stylesheet wins over it, whatever its
specificity. `rich-chat.structure` is what the components need to work
and is always injected; `rich-chat.theme` is the look, the code colors,
and the names' rules, the parts the props above turn off.

### CSS custom properties

The theme declares these on `:root`. Override any of them on `:root`,
on `.rc-chat`, or on anything between; the nearest declaration wins, and
a token you take over is yours in both light and dark mode. Dark mode
follows `prefers-color-scheme` unless the document sets
`data-theme="light"` or `"dark"` on its root element.

| Property                | Sets                                                                                          |
|-------------------------|-----------------------------------------------------------------------------------------------|
| `--rc-font`, `--rc-mono`, `--rc-math`, `--rc-math-text` | The prose, code, math, and math-text font stacks.                         |
| `--rc-font-size`, `--rc-line-height` | The base type size (`15px`) and leading.                                         |
| `--rc-radius`, `--rc-tail` | A bubble's corner radius, and the smaller radius of the corner on its side.                |
| `--rc-bubble-max-width` | The widest a bubble gets, `min(85%, 76ch)`. One value for the whole chat; the crate does not clamp it. |
| `--rc-bg`               | The neutral surface: the window, the composer strip and its text box, button hover fills.     |
| `--rc-chat-bg`          | The window alone, the ground behind the bubbles. Undeclared by default, so it follows `--rc-bg`. A gradient or image works too. |
| `--rc-fg`, `--rc-muted` | The text color, and the color of the name over a bubble and the empty transcript's message.   |
| `--rc-bubble-bg`, `--rc-bubble-fg` | The plain bubble.                                                                  |
| `--rc-tint-bg`, `--rc-tint-fg` | The tinted bubble the default names give `user`, and the composer's preview.           |
| `--rc-surface`, `--rc-border` | Table headers and code block bars, and the edges of tables, code blocks, and the composer. |
| `--rc-accent`, `--rc-accent-fg` | The send button, focus rings, and task-list checkboxes, and text on the accent.        |
| `--rc-code-bg`, `--rc-inline-code-bg` | Fenced and inline code backgrounds.                                             |
| `--rc-selection`        | The selection highlight.                                                                      |
| `--rc-alert-note`, `-tip`, `-important`, `-warning`, `-caution` | The five GitHub alert colors.                                         |
| `--rc-error`            | A math error, where an equation would not parse.                                              |

```css
.rc-chat { --rc-accent: #7c3aed; --rc-tint-bg: #ede9fe; --rc-chat-bg: #fff8e7; }
```

For a value chosen at runtime, set it as an attribute:
`<Chat … attr:style="--rc-chat-bg: #fff8e7" />`. The size of the name
over a bubble is a rule of yours: `.rc-sender { font-size: 1em; }`.

### Cargo features

| Feature         | Default | Costs                     | Without it                                                                        |
|-----------------|---------|---------------------------|-----------------------------------------------------------------------------------|
| `highlight`     | on      | about a megabyte of grammars | Code blocks are plain.                                                          |
| `bundled-fonts` | on      | about half a megabyte of fonts | Serve `style::FONT_FILES` yourself and inject `style::font_faces_from("/fonts")`. |

### The pieces

`Chat` is built from parts that stand alone, with the same props where
they share them:

| Component     | Renders                                                                                    |
|---------------|--------------------------------------------------------------------------------------------|
| `Composer`    | The text box, its collapsible preview, and send button; attributes reach the box.          |
| `MessageView` | One message, placed and colored by its name, and named on request.                         |
| `RichText`    | Any Markdown, from a `Signal<String>`.                                                     |
| `CodeBlock`   | One highlighted block with language label and copy button.                                 |

The copy button's words are a `CodeLabels { copy, copied }` provided as
context, so one `provide_context` changes them everywhere. The `render`
module underneath (`render_html`, `render_blocks`, `complete_draft`) is
plain Rust with no DOM dependency, for tests and for hosts that are not
Leptos.

## Running the app

The [`app`](app/) crate is just a demo of `leptos-rich-chat`: a chat
window that opens on a tour of what renders, with a bar above it for
trying its settings, and a [Tauri](https://tauri.app) shell in
[`app/src-tauri`](app/src-tauri/) that puts the same page in a desktop
window. The hosted copy at <https://wpm.github.io/Rich-Chat/> is this
build.

Requirements: Rust with the `wasm32-unknown-unknown` target,
[Trunk](https://trunkrs.dev), [Tauri's CLI](https://tauri.app) and its
[platform prerequisites](https://tauri.app/start/prerequisites/).

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk tauri-cli --locked
cd app && cargo tauri dev
```

In the browser instead of a window: `cd app && trunk serve` and open
<http://localhost:1420>.

## Development

```sh
cargo test -p leptos-rich-chat     # the library, natively
cargo clippy --workspace --all-targets
cd app && trunk build --release    # the frontend, to app/dist
cd app/e2e && npm install && npm test   # the built app in headless Chromium
cargo run -p leptos-rich-chat --example highlight_css -- OneHalfDark dark   # regenerate assets/highlight.css, see the library README
cargo llvm-cov -p leptos-rich-chat --open   # test coverage, as a report in the browser; needs cargo-llvm-cov
```

The [CI workflow](.github/workflows/ci.yml) runs the library's tests
natively, builds the frontend and drives it in headless Chromium, checks
the desktop shell on every platform, and measures the library's test
coverage, which goes to [Codecov](https://codecov.io/gh/wpm/Rich-Chat)
and is the badge above; [`codecov.yml`](codecov.yml) says how coverage
is judged.

**Supplying `CODECOV_TOKEN`.** The upload identifies itself to Codecov
with a token that the workflow reads from the repository secret
`CODECOV_TOKEN`. To set it up once: on Codecov, open the repository,
then Settings > General, and copy the *Repository Upload Token*; on
GitHub, open the repository's Settings > Secrets and variables >
Actions, and add a repository secret named `CODECOV_TOKEN` with that
value. Without the secret the coverage job fails, and with it the CI
check, on purpose: an upload that could not happen is a broken
workflow, not a quiet gap in the graph. Pull requests from forks do not
see the secret, and Codecov accepts their uploads without it.

## License

MIT. See [LICENSE](LICENSE). The bundled Latin Modern fonts are under
the GUST Font License; see
[leptos-rich-chat/assets/fonts/LICENSE.md](leptos-rich-chat/assets/fonts/LICENSE.md).
