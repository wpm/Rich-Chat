# Changelog

All notable changes to `leptos-rich-chat`. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/): while the major
version is 0, a minor release may change the API.

## [Unreleased]

### Added

- `selected` and `on_select` on `Transcript`, for a host that steps
  through a finished conversation. `selected` names a message by id:
  it is `rc-selected` and `aria-current`, outlined in
  `--rc-selected-outline`; every message after it is `rc-later`, faded
  to `--rc-later-opacity`; it scrolls into view whenever the selection
  changes; and the transcript stops following its end while there is
  one. `on_select` receives the id of a message the reader clicks,
  though not through a link or a button in it, and makes the pane
  `rc-selectable`.

## [0.2.1] - 2026-10-17

### Fixed

- The transcript no longer stops following its end when a bubble goes
  away and another arrives before the scroll event for the first. Only
  a scroll up that comes with the reader's own input to the pane leaves
  the end: the wheel turned up, a key that scrolls up, a finger moving,
  or a press on the scrollbar. A scroll up from anything else, such as
  the browser's find in page, no longer does. The wheel and touch
  listeners are passive, so they never hold up scrolling.

## [0.2.0] - 2026-10-16

### Changed

- `Look` is `#[non_exhaustive]`, and gains a public `properties`
  field. A look is made with `Look::at` and its methods; code that
  built one by listing its fields no longer compiles. Its fields can
  still be read.

### Added

- `sender`, to draw the name over a bubble: `Chat`, `Transcript` and
  `MessageView` take a `Callback<String, AnyView>` that is given the
  message's name and returns what goes in `div.rc-sender`, such as a
  name in bold with a role in italics, or a badge.
- `Look::property`, to carry custom properties onto a name's messages
  and composer preview, where a host's static rules read them: the
  color of the name over a bubble, a border, a badge.

## [0.1.0] - 2026-10-14

The first release: `Chat`, `Transcript`, `Composer`, `MessageView`,
`RichText`, `CodeBlock`, `RichChatStyle`, and the rendering core behind
them.

- Markdown through pulldown-cmark: CommonMark plus GitHub’s tables, task
  lists, strikethrough, footnotes, alerts, and bare-URL autolinks. Raw
  HTML is shown as text; links keep only `http`, `https`, and `mailto`.
- LaTeX math, inline and display, rendered to MathML by pulldown-latex
  and set in the bundled Latin Modern Math font (`bundled-fonts`).
- Fenced code blocks highlighted by syntect with two-face’s grammars, in
  class-based colors themed for light and dark (`highlight`), with a
  language label and a copy button on each block.
- Progressive rendering: text renders block by block with stable keys,
  and constructs left open at the end of a draft are closed for display.
- Any number of speakers: a `Names` table says which side each name’s
  bubbles sit on and what colors they have, and `show_names` writes the
  name over each bubble.
- Entries the host draws itself, `Body::View` via `Message::view`, for
  what isn’t speech: a tool call, a failure, a prompt to answer. The
  transcript still places, keys, and scrolls them.
- A thinking indicator: a live message whose text is still empty shows
  dancing dots, which turn into its text in place. Several speakers can
  be thinking at once.
- `busy`, for a host waiting on a reply: the reader keeps writing and
  only the send waits.
- `Composer` and `Chat` take an optional draft, an `RwSignal<String>`
  the host holds, to prefill the text box, read what is being typed, or
  keep a draft across unmounting.
- Styles in cascade layers, with every color and size a `--rc-*` custom
  property, so a host can restyle any of it.

[Unreleased]: https://github.com/wpm/Rich-Chat/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/wpm/Rich-Chat/releases/tag/v0.2.1
[0.2.0]: https://github.com/wpm/Rich-Chat/releases/tag/v0.2.0
[0.1.0]: https://github.com/wpm/Rich-Chat/releases/tag/leptos-rich-chat-v0.1.0
