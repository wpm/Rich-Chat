# Rich-Chat

A chat interface with rich formatting: Markdown, LaTeX math, and
syntax-highlighted code in every language, rendered as you type.

Two crates:

- [`leptos-rich-chat`](leptos-rich-chat/) — the Leptos components, published to
  crates.io. See its [README](leptos-rich-chat/README.md) for the API.
- [`app`](app/) — a Tauri desktop app for trying them: a chat window with
  a text box at the bottom. What you send appears as a bubble.

## Running the app

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
cargo test -p leptos-rich-chat            # the rendering core, natively
cargo clippy --workspace --all-targets
cd app && trunk build              # the frontend, to app/dist
cargo run -p leptos-rich-chat --example theme_css -- OneHalfDark dark
```

## License

MIT. See [LICENSE](LICENSE). The bundled Latin Modern fonts are under
the GUST Font License; see
[leptos-rich-chat/assets/fonts/LICENSE.md](leptos-rich-chat/assets/fonts/LICENSE.md).
