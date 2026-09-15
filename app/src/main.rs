//! The test window: one chat, no model. It opens on a tour of what the
//! components render, and every message sent appears as a bubble, which
//! is all that is needed to exercise the rendering.

mod opener;

use leptos::prelude::*;
use leptos_rich_chat::{Chat, Message, RichChatStyle, Role};

/// The first bubble, from the assistant's side: Markdown, code in two
/// languages, and some display math, so the window shows what it can do
/// before anything is typed.
const WELCOME: &str = include_str!("../welcome.md");

#[component]
fn App() -> impl IntoView {
    let messages = RwSignal::new(vec![Message::new("welcome", Role::Assistant, WELCOME)]);
    let send = move |text: String| {
        let id = format!("m{}", messages.read_untracked().len());
        messages.update(|all| all.push(Message::new(id, Role::User, text)));
    };
    view! {
        <RichChatStyle />
        <main class="app">
            <Chat messages=messages on_send=send on_link=Callback::new(opener::open_url) />
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[cfg(test)]
mod tests {
    use super::WELCOME;
    use leptos_rich_chat::render::{RenderOptions, render_html};

    /// The tour has to show off everything it claims to, and nothing in it
    /// may be a construct the renderer rejects.
    #[test]
    fn the_welcome_renders_everything_it_shows_off() {
        let html = render_html(WELCOME, &RenderOptions::default());
        assert!(
            !html.contains("<merror"),
            "an equation failed to parse: {html}"
        );
        assert!(html.contains("<table"), "{html}");
        assert!(html.contains("class=\"markdown-alert-tip\""), "{html}");
        assert!(html.contains("class=\"rc-footnotes\""), "{html}");
        assert!(html.contains("<input"), "no task list: {html}");
        assert_eq!(
            html.matches("<pre class=\"rc-code\"").count(),
            2,
            "two code blocks: {html}"
        );
        assert!(html.contains("data-language=\"Rust\""), "{html}");
        assert!(html.contains("data-language=\"Python\""), "{html}");
        assert!(
            html.matches("<math display=\"block\"").count() >= 3,
            "three display equations: {html}"
        );
        assert!(
            html.contains("<mtable"),
            "no aligned or cases environment: {html}"
        );
    }
}
