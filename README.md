# Rich-Chat

[![CI](https://github.com/wpm/Rich-Chat/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/Rich-Chat/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/wpm/Rich-Chat/graph/badge.svg)](https://codecov.io/gh/wpm/Rich-Chat)

A chat interface with rich formatting: Markdown, LaTeX math, and
syntax-highlighted code in every language, rendered as you type.

**Try it with nothing installed:** <https://wpm.github.io/Rich-Chat/>.
Every push to `main` redeploys it from the [Pages workflow](.github/workflows/pages.yml),
and every pull request gets its own copy at `https://wpm.github.io/Rich-Chat/pr/<number>/`
from the [Preview workflow](.github/workflows/preview.yml): it is the
deployment linked from the pull request, follows each push, and goes away
when the pull request closes. The site is the `gh-pages` branch, which
those workflows write with [`publish-site.sh`](.github/scripts/publish-site.sh)
and which Pages must be set to serve (Settings > Pages > Source: Deploy
from a branch); the workflows check that and say so when it is not.

Three crates:

- [`leptos-rich-chat`](leptos-rich-chat/) — the Leptos components, published to
  crates.io. See its [README](leptos-rich-chat/README.md) for the API.
- [`app`](app/) — a browser app for trying them: a chat window with a
  text box at the bottom. It opens on a tour of what renders, in four
  bubbles: the introduction in [`app/welcome.md`](app/welcome.md) from
  the Assistant, the Markdown in
  [`app/welcome-markdown.md`](app/welcome-markdown.md) from the User, the
  code in [`app/welcome-code.md`](app/welcome-code.md) from the
  Assistant, then the math in
  [`app/welcome-math.md`](app/welcome-math.md) from the User. What you
  send appears as a bubble from the selected user. A bar above the chat
  lists the users, an Assistant and a User to begin with, and lets you
  add and delete them; puts the selected user's bubbles on the left, in
  the center, or on the right, and sets their color, for every bubble
  of theirs; sets the widest a message gets, for every user at once,
  from the library's own measure up to the whole transcript; colors the
  window, the ground behind the bubbles, and puts it back to the
  theme's; writes each user's name over their bubbles, or not, and sets
  how big that name is; makes the chat busy, as a host waiting on a
  reply does, so that you can go on writing and nothing sends, or
  disabled, the composer off; and switches between light and dark.
  Drag the top edge of the text box to make it taller. The choices are
  kept between runs.
  This is what the site above serves.
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

## Releasing

The library and the app are released separately, each from its own kind
of tag, and the version of each is what its manifest says: the tag has
to match it or nothing is built.

- **The library, to crates.io.** Set the version in
  [`leptos-rich-chat/Cargo.toml`](leptos-rich-chat/Cargo.toml), add the
  entry to its [changelog](leptos-rich-chat/CHANGELOG.md), and push a tag
  `leptos-rich-chat-v<version>`. The
  [Publish the crate workflow](.github/workflows/publish-crate.yml)
  tests the crate, builds its documentation as docs.rs will, and
  publishes it. The first version has to be published by hand
  (`cargo publish -p leptos-rich-chat`), since trusted publishing is
  set up on a crate that exists: on crates.io, under the crate's
  Settings > Trusted Publishing, add GitHub, owner `wpm`, repository
  `Rich-Chat`, workflow `publish-crate.yml`. No token is stored here.
  Run by hand from the Actions tab, the workflow does everything but
  the publishing.
- **The app, as installers.** Set the version in
  [`app/src-tauri/Cargo.toml`](app/src-tauri/Cargo.toml) and
  [`app/Cargo.toml`](app/Cargo.toml) (the Tauri config takes it from
  the former), and push a tag `app-v<version>`. The
  [Release the app workflow](.github/workflows/release-app.yml) opens a
  draft release, builds a `.dmg` for macOS (Apple silicon and Intel in
  one), a setup `.exe` and an `.msi` for Windows, and a `.deb`, an
  `.rpm` and an `.AppImage` for Linux, and publishes the release once
  they are all on it. Run by hand, it builds them as workflow
  artifacts and makes no release. The macOS build is signed and
  notarized when the repository has the secrets `APPLE_CERTIFICATE`
  (the Developer ID Application certificate, exported from Keychain
  Access as a `.p12` and base64-encoded), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_ID`, `APPLE_PASSWORD` (an app-specific password) and
  `APPLE_TEAM_ID`; without them it is unsigned, and the release notes
  say how to open it.

## License

MIT. See [LICENSE](LICENSE). The bundled Latin Modern fonts are under
the GUST Font License; see
[leptos-rich-chat/assets/fonts/LICENSE.md](leptos-rich-chat/assets/fonts/LICENSE.md).
