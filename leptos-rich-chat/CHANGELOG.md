# Changelog

All notable changes to `leptos-rich-chat`. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org/): while the major
version is 0, a minor release may change the API.

## [Unreleased]

### Added

- `Chat` and `MessageView` take `show_names`, which writes each
  message's name over its bubble in a `div.rc-sender`, on the bubble's
  side, in the theme's muted color. Off by default; may be a signal.
- `Body`, what a message shows: `Body::Text`, the Markdown the crate
  renders in a bubble, or `Body::View`, a view the host draws, which is
  where a transcript puts what is not speech — a tool call, a failure, a
  prompt the reader has to answer — without leaving the pane that
  scrolls. The crate still places the message by its name, keys it, and
  scrolls to it and follows it as it grows.
- `Message::view(id, name, view)`, a message whose body the host draws.
  Its root element takes the bubble's place as the child of
  `div.rc-message`, rather than sitting inside the bubble, since the
  bubble is where the width, padding, radius and background live and a
  body that is deliberately not speech would otherwise start by undoing
  them. A host that wants the bubble around its own body puts
  `class="rc-bubble"` on that root element; the name's colors and tail
  land on it, because the generated rules are
  `.rc-message[data-name="…"] > .rc-bubble`.
- `Composer` and `Chat` take `busy`, a `Signal<bool>` for a host whose
  reply is still in flight: nothing more may be sent until it lands, and
  the reader can still write the next message. While it is true the text
  box is editable and keeps its draft, the preview renders it as ever,
  neither Enter nor the send button sends (Enter breaks no line either;
  Shift+Enter does), and `.rc-composer` has the class `rc-busy`, a hook
  for a host's styles that the theme leaves alone. When it falls back
  to false nothing is sent on its own: the draft waits for Enter or the
  button.
- `Composer` and `Chat` take `busy_label`, the words a screen reader
  gets for the wait, "Sending is paused" by default; a host talking to
  a model might pass "Waiting for a reply". `rc-busy` and a disabled
  send button had told a reader who does not see the screen nothing: a
  press went nowhere with no word of why, and "Send, dimmed" did not
  tell a busy composer from an empty one. Now, while the wait alone
  holds a draft back, the send button is named `Send, {busy_label}` and
  carries `aria-disabled="true"` in place of `disabled`, so it stays in
  the tab order and going to it says why it cannot be pressed; the
  theme gives it the disabled look all the same. A composer that is
  off, or a blank draft, still disables the button under the plain
  name. And a press that only the wait refused, by Enter or the button,
  puts `busy_label` in `div.rc-composer-status`, a `role="status"` live
  region that is the first child of `.rc-composer`, always in the
  markup and hidden from sight by the structure stylesheet. The phrase
  stays until the wait ends; repeated presses within one wait announce
  it once, and the wait beginning announces nothing.

### Changed

- The composer's text box has `spellcheck="true"`,
  `autocapitalize="sentences"` and `autocorrect="on"`, the attributes
  prose in a chat box wants; a host that passes one of them wins.
- Attributes passed to `Composer` land on its text box rather than on
  the `.rc-composer` wrapper, so `attr:id`, `attr:maxlength`,
  `attr:data-*` and the rest reach the input. Attributes passed to
  `Chat` land on `.rc-chat`, as before.
- The composer returns focus to the text box after a send, so the caret
  is there for the next message whether the send came by Enter or by a
  click on the button, which had taken the focus with it. Not on
  mount: where the caret goes when a page opens is the host's call.
- The theme's custom properties are declared on the document root, in
  place of the outermost of `.rc-chat`, `.rc-rich` and `.rc-composer`,
  so a host can declare them on any ancestor of the components rather
  than only at the chat's own boundary. `.rc-chat { --rc-accent: … }`
  keeps winning; a declaration further up now wins too, by proximity. A
  host that takes a token over on an ancestor owns it in both themes,
  since the crate's dark values are on `:root` and no longer reach it.
- The theme declares the list markers of rendered Markdown itself:
  `disc` for a bulleted list, `circle` and then `square` for the ones
  nested in it, `decimal` for a numbered one. These are the browser's
  defaults, so a host without a CSS reset sees no change; a host with
  one in a layer beneath the crate's, such as Tailwind's preflight, gets
  its markers back, where `list-style: none` used to go unanswered,
  whether the reset names the list or the item. Only the marker's type
  is declared, so a position or an image a host hands down still
  arrives. Task-list items still have none.
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
- `Message::content`, a `Signal<String>`, is `Message::body`, a `Body`.
  `Message::new` and `Message::streaming` are unchanged and build a
  `Body::Text`; a host that built a `Message` field by field, or read
  `content`, matches on `body` instead.
- `MessageBubble` is `MessageView`. It names what it renders: under
  `Body::View` the thing it draws may deliberately not be a bubble. The
  `rc-*` classes are unchanged, and `.rc-bubble` is now more precise
  than it was, since the bubble appears for `Body::Text` or by the
  host's opt-in.
- `Message` no longer implements `PartialEq`. A view body is a function,
  which has no honest equality, and a dishonest one would be worst in
  the place a host would use it: a `Memo<Vec<Message>>` whose view arms
  never compare equal fires on every read. Use `Signal::derive`, since
  the transcript's keyed diff already decides what is rebuilt.
- A message, `div.rc-message`, is a column, and the generated rules
  place its bubble with `align-items` on it rather than auto margins on
  the bubble, so that the name and the bubble share a side. A host that
  placed bubbles with a rule on `.rc-bubble`'s margins sets
  `align-items` on `.rc-message` instead.
- `disabled`'s documentation, on `Composer` and `Chat`, says what it
  does: the composer is off, the text box disabled, nothing previewed
  and nothing sent, with the draft signal kept. It had said "Blocks
  sending while true; the draft is kept", which describes `busy`.
- A disabled composer shows no preview whatever its draft holds, which
  is what makes "nothing previewed" true of a draft the host put in the
  box. Before, a draft already there was previewed beside a box that
  could not edit it.

### Fixed

- The crate builds without warnings when the `highlight` feature is
  off. `render::code::highlight` reads its `token` only when the
  feature is on, and a host that set `default-features = false` saw an
  unused-variable warning in its own build. The signature and behavior
  are unchanged, and CI now runs clippy with `--no-default-features`
  too.
- A loose task list, its items separated by blank lines, showed a
  bullet beside each checkbox. A loose item wraps its checkbox in a
  paragraph, and the theme's task-list rule matched only a checkbox
  that is the item's own child, which is what a tight list renders.
  The rule now matches both shapes, so a loose task item has no marker
  and sits where a tight one does.

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
