# Changelog

All notable changes to `leptos-rich-chat`. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/): while the major
version is 0, a minor release may change the API.

## [Unreleased]

## [0.1.0] - 2026-09-16

The first release: `Chat`, `Message`, `RichChatStyle` and the rendering
pieces behind them.

- Markdown through pulldown-cmark: CommonMark plus GitHub's tables, task
  lists, strikethrough, footnotes, alerts, and bare-URL autolinks. Raw
  HTML is shown as text; links keep only `http`, `https` and `mailto`.
- LaTeX math, inline and display, rendered to MathML by pulldown-latex
  and set in the bundled Latin Modern Math font (`bundled-fonts`).
- Fenced code blocks highlighted by syntect with two-face's grammars, in
  class-based colors themed for light and dark (`highlight`), with a
  language label and a copy button on each block.
- Progressive rendering: text renders block by block with stable keys,
  and constructs left open at the end of a draft are closed for display.
- Styles in cascade layers, so a host can restyle any of it.
- `Composer` and `Chat` take an optional `draft`, an `RwSignal<String>`
  the host holds, in place of the composer's own: to prefill the text
  box, read what is being typed, or keep a draft across unmounting.

[Unreleased]: https://github.com/wpm/Rich-Chat/compare/leptos-rich-chat-v0.1.0...HEAD
[0.1.0]: https://github.com/wpm/Rich-Chat/releases/tag/leptos-rich-chat-v0.1.0
