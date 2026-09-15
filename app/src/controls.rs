//! The bar above the chat: theme, bubble side, bubble colour.
//!
//! Each control edits one field of the app's [`Settings`]; the app root
//! turns those into attributes and custom properties that the stylesheet
//! applies to the library's components.

use leptos::prelude::*;

use crate::settings::{Settings, Side, Theme};

/// The controls. `theme` is the one in effect, which is the chosen one
/// or, until one is chosen, the system's.
#[component]
pub fn Controls(settings: RwSignal<Settings>, theme: Signal<Theme>) -> impl IntoView {
    let dark = move || theme.get() == Theme::Dark;
    let toggle_theme = move |_| {
        let next = theme.get_untracked().toggled();
        settings.update(|settings| settings.theme = Some(next));
    };

    let side_button = move |side: Side, label: &'static str| {
        view! {
            <button
                type="button"
                class="control-segment control-side"
                value=side.as_str()
                aria-pressed=move || (settings.read().side == side).to_string()
                on:click=move |_| settings.update(|settings| settings.side = side)
            >
                {label}
            </button>
        }
    };

    // The colour input shows the theme's own bubble colour until one is
    // chosen, so opening it starts from what is on screen.
    let bubble = move || {
        settings
            .read()
            .bubble
            .clone()
            .unwrap_or_else(|| theme.get().bubble().to_string())
    };
    let custom = move || settings.read().bubble.is_some();

    view! {
        <header class="controls" aria-label="Appearance">
            <button
                type="button"
                class="control-button control-theme"
                role="switch"
                aria-checked=move || dark().to_string()
                title="Switch between light and dark"
                on:click=toggle_theme
            >
                <span class="control-icon" aria-hidden="true">
                    {move || if dark() { "\u{263E}" } else { "\u{2600}" }}
                </span>
                {move || if dark() { "Dark" } else { "Light" }}
            </button>
            <div class="control-group" role="group" aria-label="Bubble side">
                <span class="control-label">"Bubbles"</span>
                <span class="control-segments">
                    {side_button(Side::Left, "Left")}
                    {side_button(Side::Right, "Right")}
                </span>
            </div>
            <div class="control-group">
                <label class="control-label" for="bubble-colour">"Colour"</label>
                <input
                    id="bubble-colour"
                    type="color"
                    class="control-colour"
                    title="Bubble colour"
                    prop:value=bubble
                    on:input=move |event| {
                        let value = event_target_value(&event);
                        settings.update(|settings| settings.bubble = Some(value));
                    }
                />
                <button
                    type="button"
                    class="control-button control-reset"
                    title="Back to the theme's own bubble colour"
                    disabled=move || !custom()
                    on:click=move |_| settings.update(|settings| settings.bubble = None)
                >
                    "Reset"
                </button>
            </div>
        </header>
    }
}
