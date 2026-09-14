//! Chat components for [Leptos](https://leptos.dev) that render Markdown,
//! LaTeX math, and syntax-highlighted code.
//!
//! ```ignore
//! use leptos::prelude::*;
//! use rich_chat::{Chat, Message, RichChatStyle, Role};
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     let messages = RwSignal::new(Vec::<Message>::new());
//!     let send = move |text: String| {
//!         let id = messages.read().len().to_string();
//!         messages.update(|all| all.push(Message::new(id, Role::User, text)));
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
//!   colours the stylesheet themes for light and dark.
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
//! # Styling
//!
//! [`RichChatStyle`] injects [`style::STYLESHEET`] and the fonts. Every
//! colour is a `--rc-*` custom property; dark mode follows the system
//! or a `data-theme` attribute on the root element. See [`style`].
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
mod message;
pub mod render;
pub mod style;

pub use components::{Chat, CodeBlock, Composer, MessageBubble, RichChatStyle, RichText};
pub use message::{Message, Role};
pub use render::RenderOptions;
