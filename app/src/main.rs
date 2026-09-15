//! The test window: one chat, no model. It opens on a tour of what the
//! components render, and every message sent appears as a bubble, which
//! is all that is needed to exercise the rendering.
//!
//! Above the chat is a bar of controls (light or dark, which side the
//! bubbles land on, their colour), and the composer's top edge can be
//! dragged to make the text box taller. All of that is this app's: the
//! library only gets attributes and custom properties to style by.

mod controls;
mod opener;
mod settings;

use leptos::ev;
use leptos::prelude::*;
use leptos_rich_chat::{Chat, Message, RichChatStyle, Role};
use wasm_bindgen::JsCast;

use crate::controls::Controls;
use crate::settings::{Settings, Theme, text_on};

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

/// The first bubble, from the assistant's side: Markdown, code in two
/// languages, and some display math, so the window shows what it can do
/// before anything is typed.
const WELCOME: &str = include_str!("../welcome.md");

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

    let messages = RwSignal::new(vec![Message::new("welcome", Role::Assistant, WELCOME)]);
    let send = move |text: String| {
        let id = format!("m{}", messages.read_untracked().len());
        messages.update(|all| all.push(Message::new(id, Role::User, text)));
    };

    // The settings the stylesheet reads as custom properties.
    let style = move || {
        let settings = settings.read();
        let mut css = String::new();
        if let Some(bubble) = &settings.bubble {
            css.push_str(&format!("--app-bubble-bg: {bubble};"));
            if let Some(text) = text_on(bubble) {
                css.push_str(&format!("--app-bubble-fg: {text};"));
            }
        }
        if let Some(height) = settings.input_height {
            css.push_str(&format!("--app-input-height: {height}px;"));
        }
        css
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
        <RichChatStyle />
        <main
            id="app"
            class="app"
            class:app-bubble=move || settings.read().bubble.is_some()
            class:app-resizing=move || drag.read().is_some()
            data-side=move || settings.read().side.as_str()
            style=style
            on:pointerdown=press
            on:pointermove=moved
            on:pointerup=release
            on:pointercancel=release
        >
            <Controls settings=settings theme=theme />
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
