//! Chat components for [Leptos](https://leptos.dev) that render Markdown,
//! LaTeX math, and syntax-highlighted code.
//!
//! ```ignore
//! use leptos::prelude::*;
//! use leptos_rich_chat::{Chat, Message, RichChatStyle};
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     let messages = RwSignal::new(Vec::<Message>::new());
//!     let send = move |text: String| {
//!         let id = messages.read().len().to_string();
//!         messages.update(|all| all.push(Message::new(id, "user", text)));
//!     };
//!     view! {
//!         <RichChatStyle />
//!         <Chat messages=messages on_send=send />
//!     }
//! }
//! ```
//!
//! # What renders
//!
//! - **Markdown**: CommonMark plus GitHub's tables, task lists,
//!   strikethrough, footnotes, alerts, and bare-URL autolinks. Raw HTML
//!   is shown as text; links keep only `http`, `https`, and `mailto`.
//! - **Math**: `$…$` inline and `$$…$$` display, rendered to MathML by
//!   [pulldown-latex](https://crates.io/crates/pulldown-latex) and set
//!   in Latin Modern Math. No JavaScript.
//! - **Code**: fenced blocks in some two hundred languages, highlighted
//!   by [syntect](https://crates.io/crates/syntect) with class-based
//!   colors the stylesheet themes for light and dark. A grammar is
//!   compiled the first time its language appears; [`Chat`] compiles the
//!   common ones in idle time after mounting, and [`warm_up`] does the
//!   same for a host that uses the pieces on their own.
//!
//! # Progressive rendering
//!
//! Text is rendered block by block with stable keys, so as a draft is
//! typed or a streamed message grows only the block being changed is
//! touched. Unfinished constructs at the end of a draft (an open `$$`, a
//! half-typed `` `code` `` span) are closed for display, so the preview
//! shows what the text is becoming rather than flickering.
//!
//! # Pieces
//!
//! [`Chat`] is the whole window; [`Composer`], [`MessageBubble`],
//! [`RichText`], and [`CodeBlock`] are its parts, each usable alone. The
//! [`render`] module is the pure Markdown-to-HTML core, with no DOM
//! dependency, for tests and for hosts that are not Leptos.
//!
//! # Kinds of message
//!
//! A [`Message`] carries a `kind`, a name the host chooses, and the crate
//! attaches no meaning to it. The host's [`Kinds`] table says where each
//! kind sits and what colors it has; [`RichChatStyle`] turns that into
//! CSS. The default table is `user` on the right and `assistant` on the
//! left. A group chat, or a transcript with notices down the middle, is
//! a different table, with a [`Look`] at a [`Position`] per kind.
//!
//! # Styling
//!
//! The components carry `rc-*` classes and no inline styles; the look
//! is all CSS, and all of it is the host's to decide. [`RichChatStyle`]
//! injects the crate's CSS in cascade layers, so a host's own unlayered
//! rules win over it whatever their specificity. It comes in two tiers:
//! [`style::STRUCTURE`], the rules the components need to work, and
//! [`style::THEME`] with [`style::HIGHLIGHT`], the default look, which a
//! host can override property by property (every color is a `--rc-*`
//! custom property) or switch off and replace. Dark mode in the theme
//! follows the system or a `data-theme` attribute on the root element.
//! See [`style`].
//!
//! The words in the interface are props too: the composer's
//! `placeholder`, `hint`, `preview_label`, and `send` content, the
//! transcript's `empty` text, and the copy button's [`CodeLabels`].
//!
//! # Features
//!
//! - `highlight` (default): syntax highlighting. About a megabyte of
//!   grammars.
//! - `bundled-fonts` (default): the math fonts embedded in the binary.
//!   About half a megabyte. Without it, serve [`style::FONT_FILES`]
//!   yourself and inject [`style::font_faces_from`].

#![forbid(unsafe_code)]

mod components;
mod kinds;
mod message;
pub mod render;
pub mod style;

pub use components::{
    Chat, CodeBlock, CodeLabels, Composer, MessageBubble, RichChatStyle, RichText, warm_up,
};
pub use kinds::{Kinds, Look, Position};
pub use message::Message;
pub use render::RenderOptions;
