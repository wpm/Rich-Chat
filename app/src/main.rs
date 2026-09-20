//! The test window: one chat, no model. It opens on a tour of what the
//! components render, and every message sent appears as a bubble from
//! the selected user, which is all that is needed to exercise the
//! rendering.
//!
//! Above the chat is a bar of controls: the users, who can be added and
//! removed; where the selected user's bubbles land and their color,
//! which changes every bubble of theirs; the window's background, the
//! ground behind the bubbles alone; whether every bubble has its
//! user's name over it; whether the chat is busy, as a host waiting on a
//! model's reply sets it, so that the text box and the preview stay and
//! only the sending waits, or disabled, the composer off; light or dark.
//! The composer's top edge can be dragged to make the text box taller.
//! All of that is this app's: the
//! library gets a table of names saying where each user's bubbles go
//! and in what colors, a flag for the names, and the stylesheet gets a
//! custom property for the text box and one for the window.

mod controls;
mod opener;
mod settings;

use leptos::ev;
use leptos::prelude::*;
use leptos_rich_chat::{Chat, Message, RichChatStyle};
use wasm_bindgen::JsCast;

use crate::controls::Controls;
use crate::settings::{ASSISTANT, Settings, Theme, USER};

/// How far below the composer's top edge a press still grabs it, in CSS
/// pixels. The same as the hit zone the stylesheet draws.
const GRIP: f64 = 12.0;
/// The text box is never dragged shorter than its one-line height.
const MIN_INPUT_HEIGHT: f64 = 42.0;
/// Nor taller than this much of the window, so the transcript stays.
const MAX_INPUT_SHARE: f64 = 0.7;

/// A drag of the composer's top edge in progress: where the pointer went
/// down and how tall the text box was then.
#[derive(Clone, Copy)]
struct Drag {
    pointer_y: f64,
    height: f64,
}

/// The first bubble, from the assistant.
const WELCOME: &str = include_str!("../welcome.md");
/// The second bubble, from the user: the Markdown.
const WELCOME_MARKDOWN: &str = include_str!("../welcome-markdown.md");
/// The third, from the assistant again: the code.
const WELCOME_CODE: &str = include_str!("../welcome-code.md");
/// The fourth, from the user again: the math.
const WELCOME_MATH: &str = include_str!("../welcome-math.md");

#[component]
fn App() -> impl IntoView {
    let settings = RwSignal::new(Settings::load());
    let system_dark = settings::system_prefers_dark();
    let theme = Signal::derive(move || {
        settings.read().theme.unwrap_or(if system_dark {
            Theme::Dark
        } else {
            Theme::Light
        })
    });
    Effect::new(move |_| settings.read().store());
    // The library's stylesheet keys its palette on the root element's
    // data-theme; so does this app's.
    Effect::new(move |_| {
        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
        {
            let _ = root.set_attribute("data-theme", theme.get().as_str());
        }
    });

    // Every message sent is from the selected user.
    let messages = RwSignal::new(vec![
        Message::new("welcome", ASSISTANT, WELCOME),
        Message::new("welcome-markdown", USER, WELCOME_MARKDOWN),
        Message::new("welcome-code", ASSISTANT, WELCOME_CODE),
        Message::new("welcome-math", USER, WELCOME_MATH),
    ]);
    let sender = Signal::derive(move || settings.read().selected().to_string());
    let send = move |text: String| {
        let from = sender.get_untracked();
        let id = format!("m{}", messages.read_untracked().len());
        messages.update(|all| all.push(Message::new(id, from, text)));
    };

    // Where each user's bubbles go and their colors, for the library's
    // stylesheet; and whether it writes their names over them.
    let names = Signal::derive(move || settings.read().names());
    let show_names = Signal::derive(move || settings.read().show_names);
    // The wait a host with a model has between a message and its reply.
    // Not a setting: every run starts with nothing in flight.
    let busy = RwSignal::new(false);
    // And the composer off altogether, as for a host with no key.
    let disabled = RwSignal::new(false);
    // The settings the stylesheets read as custom properties: the app's
    // own takes the text box's height, and the library's window takes
    // --rc-chat-bg, inherited down to .rc-chat. Neither is emitted while
    // unset, so the window follows the theme's light and dark switch
    // until a background is chosen.
    let style = move || {
        let settings = settings.read();
        let mut style = String::new();
        if let Some(height) = settings.input_height {
            style.push_str(&format!("--app-input-height: {height}px;"));
        }
        if let Some(background) = &settings.background {
            style.push_str(&format!("--rc-chat-bg: {background};"));
        }
        style
    };

    // Dragging the composer's top edge sets the text box's height.
    let drag = RwSignal::new(Option::<Drag>::None);
    let press = move |event: ev::PointerEvent| {
        if !event.is_primary() {
            return;
        }
        let Some(composer) = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".rc-composer").ok().flatten())
        else {
            return;
        };
        let pointer_y = f64::from(event.client_y());
        if pointer_y - composer.get_bounding_client_rect().top() > GRIP {
            return;
        }
        let Some(input) = composer.query_selector(".rc-composer-input").ok().flatten() else {
            return;
        };
        event.prevent_default();
        // Keep receiving the pointer once it leaves the window.
        if let Some(root) = event
            .current_target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        {
            let _ = root.set_pointer_capture(event.pointer_id());
        }
        drag.set(Some(Drag {
            pointer_y,
            height: input.get_bounding_client_rect().height(),
        }));
    };
    let moved = move |event: ev::PointerEvent| {
        let Some(Drag { pointer_y, height }) = drag.get_untracked() else {
            return;
        };
        let ceiling = web_sys::window()
            .and_then(|window| window.inner_height().ok())
            .and_then(|height| height.as_f64())
            .map_or(f64::INFINITY, |window| window * MAX_INPUT_SHARE)
            .max(MIN_INPUT_HEIGHT);
        let wanted =
            (height + pointer_y - f64::from(event.client_y())).clamp(MIN_INPUT_HEIGHT, ceiling);
        settings.update(|settings| settings.input_height = Some(wanted.round()));
    };
    let release = move |_| drag.set(None);

    view! {
        <RichChatStyle names=names />
        <main
            class="app"
            class:app-resizing=move || drag.read().is_some()
            style=style
            on:pointerdown=press
            on:pointermove=moved
            on:pointerup=release
            on:pointercancel=release
        >
            <Controls settings=settings theme=theme busy=busy disabled=disabled />
            <Chat
                messages=messages
                on_send=send
                on_link=Callback::new(opener::open_url)
                preview_name=sender
                show_names=show_names
                busy=busy
                disabled=disabled
            />
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[cfg(test)]
mod tests {
    use super::{WELCOME, WELCOME_CODE, WELCOME_MARKDOWN, WELCOME_MATH};
    use leptos_rich_chat::render::{RenderOptions, render_html};

    /// The first bubble introduces the tour and leaves the showing off to
    /// the bubbles after it.
    #[test]
    fn the_welcome_is_only_the_introduction() {
        let html = render_html(WELCOME, &RenderOptions::default());
        assert!(html.contains("<h1"), "{html}");
        assert!(html.contains("<strong"), "{html}");
        assert!(
            !html.contains("<table"),
            "the Markdown is its own bubble's: {html}"
        );
        assert!(
            !html.contains("<pre"),
            "the code is its own bubble's: {html}"
        );
        assert!(
            !html.contains("<math"),
            "the math is its own bubble's: {html}"
        );
    }

    /// The tour has to show off everything it claims to, and nothing in it
    /// may be a construct the renderer rejects.
    #[test]
    fn the_markdown_welcome_renders_everything_it_shows_off() {
        let html = render_html(WELCOME_MARKDOWN, &RenderOptions::default());
        assert!(html.contains("<table"), "{html}");
        assert!(html.contains("class=\"markdown-alert-tip\""), "{html}");
        assert!(html.contains("class=\"rc-footnotes\""), "{html}");
        assert!(html.contains("<input"), "no task list: {html}");
        assert!(
            !html.contains("<pre"),
            "the code is its own bubble's: {html}"
        );
        assert!(
            !html.contains("<math display=\"block\""),
            "the math is its own bubble's: {html}"
        );
    }

    #[test]
    fn the_code_welcome_is_one_highlighted_block() {
        let html = render_html(WELCOME_CODE, &RenderOptions::default());
        assert_eq!(
            html.matches("<pre class=\"rc-code\"").count(),
            1,
            "one code block: {html}"
        );
        assert!(html.contains("data-language=\"Python\""), "{html}");
        assert!(!html.contains("<math"), "no math in the code: {html}");
    }

    #[test]
    fn the_math_welcome_renders_every_equation() {
        let html = render_html(WELCOME_MATH, &RenderOptions::default());
        assert!(
            !html.contains("<merror"),
            "an equation failed to parse: {html}"
        );
        assert!(html.contains("<math display=\"inline\""), "{html}");
        assert!(
            html.matches("<math display=\"block\"").count() >= 3,
            "three display equations: {html}"
        );
        assert!(
            html.contains("<mtable"),
            "no aligned or cases environment: {html}"
        );
        assert!(!html.contains("<pre"), "no code in the math: {html}");
    }
}
