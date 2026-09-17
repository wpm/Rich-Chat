# Changelog

All notable changes to `leptos-rich-chat`. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/): while the major
version is 0, a minor release may change the API.

## [Unreleased]

### Added

- `Chat` and `MessageBubble` take `show_names`, which writes each
  message's name over its bubble in a `div.rc-sender`, on the bubble's
  side, in the theme's muted color. Off by default; may be a signal.

### Changed

- The theme's custom properties are declared on the document root, in
  place of the outermost of `.rc-chat`, `.rc-rich` and `.rc-composer`,
  so a host can declare them on any ancestor of the components rather
  than only at the chat's own boundary. `.rc-chat { --rc-accent: … }`
  keeps winning; a declaration further up now wins too, by proximity. A
  host that takes a token over on an ancestor owns it in both themes,
  since the crate's dark values are on `:root` and no longer reach it.
- The highlighter's classes are `hl-keyword`, `hl-string`, and so on,
  in place of `rc-keyword` and the rest: a prefix of their own, since
  every dotted atom of a scope became a class under the components'
  prefix, so `meta.block` was `rc-block`, the wrapper around each
  Markdown block, and `entity.name.function` was `rc-name`, and a rule
  against a component's class landed on the code too. A host with rules
  of its own against the highlight classes renames them; the shipped
  `assets/highlight.css` and `examples/highlight_css` already have.
- `Message::kind` is `Message::name`: who the message is from. With it,
  the bubble's `data-kind` attribute is `data-name`, the `Kinds` table
  is `Names` with `Names::name` in place of `Kinds::kind`, and the
  composer's `preview_kind` is `preview_name`.
- A message, `div.rc-message`, is a column, and the generated rules
  place its bubble with `align-items` on it rather than auto margins on
  the bubble, so that the name and the bubble share a side. A host that
  placed bubbles with a rule on `.rc-bubble`'s margins sets
  `align-items` on `.rc-message` instead.

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
