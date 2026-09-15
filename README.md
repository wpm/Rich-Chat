# Rich-Chat

A chat interface with rich formatting: Markdown, LaTeX math, and
syntax-highlighted code in every language, rendered as you type.

**Try it with nothing installed:** <https://wpm.github.io/Rich-Chat/>.
Every push to `main` redeploys it from the [Pages workflow](.github/workflows/pages.yml),
and every pull request gets its own copy at `https://wpm.github.io/Rich-Chat/pr/<number>/`
from the [Preview workflow](.github/workflows/preview.yml): it is the
deployment linked from the pull request, follows each push, and goes away
when the pull request closes. The site is the `gh-pages` branch, which
those workflows write with [`publish-site.sh`](.github/scripts/publish-site.sh).

Three crates:

- [`leptos-rich-chat`](leptos-rich-chat/) — the Leptos components, published to
  crates.io. See its [README](leptos-rich-chat/README.md) for the API.
- [`app`](app/) — a browser app for trying them: a chat window with a
  text box at the bottom. It opens on a tour of what renders (the
  Markdown in [`app/welcome.md`](app/welcome.md)), and what you send
  appears as a bubble. A bar above the chat switches between light and
  dark, puts the bubbles on the left or the right, and sets their colour;
  drag the top edge of the text box to make it taller. The choices are
  kept between runs. This is what the site above serves.
- [`app/src-tauri`](app/src-tauri/) — the Tauri shell that puts the same
  app in a desktop window.

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
<http://localhost:1420>. The hosted copy above is the same build, made
with `trunk build --release --public-url /Rich-Chat/`.

## Development

```sh
cargo test -p leptos-rich-chat     # the library, natively
cargo clippy --workspace --all-targets
cd app && trunk build --release    # the frontend, to app/dist
cd app/e2e && npm install && npm test   # the built app in headless Chromium
cargo run -p leptos-rich-chat --example highlight_css -- OneHalfDark dark   # regenerate assets/highlight.css, see the library README
```

## License

MIT. See [LICENSE](LICENSE). The bundled Latin Modern fonts are under
the GUST Font License; see
[leptos-rich-chat/assets/fonts/LICENSE.md](leptos-rich-chat/assets/fonts/LICENSE.md).
