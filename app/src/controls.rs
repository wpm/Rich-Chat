//! The bar above the chat: the users, the selected user's bubbles, the
//! theme.
//!
//! Each control edits the app's [`Settings`]; the app root turns those
//! into the library's table of kinds, the root element's theme
//! attribute, and a custom property for the stylesheet.

use leptos::ev;
use leptos::prelude::*;

use crate::settings::{Settings, Side, Theme, User};

/// The controls. `theme` is the one in effect, which is the chosen one
/// or, until one is chosen, the system's.
#[component]
pub fn Controls(settings: RwSignal<Settings>, theme: Signal<Theme>) -> impl IntoView {
    let dark = move || theme.get() == Theme::Dark;
    let toggle_theme = move |_| {
        let next = theme.get_untracked().toggled();
        settings.update(|settings| settings.theme = Some(next));
    };

    // The users, and the one the other controls edit.
    let users = Signal::derive(move || settings.read().users.clone());
    let selected = Signal::derive(move || settings.read().selected_user().clone());
    let choose = move |event: ev::Event| {
        let name = event_target_value(&event);
        settings.update(|settings| settings.select(&name));
    };
    let can_delete = move || settings.read().can_delete();
    let delete = move |_| settings.update(|settings| settings.delete_selected());

    // A new user, typed then added with the button or Enter.
    let new_name = RwSignal::new(String::new());
    let can_add = move || settings.read().can_add(&new_name.read());
    let add = move || {
        let name = new_name.get_untracked();
        if settings.write().add_user(&name) {
            new_name.set(String::new());
        }
    };
    let typed = move |event: ev::Event| new_name.set(event_target_value(&event));
    let key = move |event: ev::KeyboardEvent| {
        if event.key() == "Enter" && !event.is_composing() {
            event.prevent_default();
            add();
        }
    };

    let side_button = move |side: Side, label: &'static str| {
        let pressed = move || selected.read().side == side;
        view! {
            <button
                type="button"
                class="control-segment control-side"
                value=side.as_str()
                aria-pressed=move || pressed().to_string()
                on:click=move |_| {
                    settings.update(|settings| settings.selected_user_mut().side = side)
                }
            >
                {label}
            </button>
        }
    };

    let color = move || selected.read().color.clone();
    let recolor = move |event: ev::Event| {
        let value = event_target_value(&event);
        settings.update(|settings| settings.selected_user_mut().color = value);
    };

    view! {
        <header class="controls" aria-label="Appearance">
            <div class="control-group" role="group" aria-label="Users">
                <select class="control-users" aria-label="User" on:change=choose>
                    <For each=move || users.get() key=|user: &User| user.name.clone() let(user)>
                        <option
                            value=user.name.clone()
                            prop:selected={
                                let name = user.name.clone();
                                move || selected.read().name == name
                            }
                        >
                            {user.name.clone()}
                        </option>
                    </For>
                </select>
                <button
                    type="button"
                    class="control-button control-delete"
                    title="Remove the selected user; their bubbles stay as they are"
                    disabled=move || !can_delete()
                    on:click=delete
                >
                    "Delete"
                </button>
                <input
                    type="text"
                    class="control-new-user"
                    placeholder="New user"
                    aria-label="New user's name"
                    prop:value=new_name
                    on:input=typed
                    on:keydown=key
                />
                <button
                    type="button"
                    class="control-button control-add"
                    disabled=move || !can_add()
                    on:click=move |_| add()
                >
                    "Add"
                </button>
            </div>
            <div class="control-group" role="group" aria-label="Bubble side">
                <span class="control-label">"Bubbles"</span>
                <span class="control-segments">
                    {side_button(Side::Left, "Left")}
                    {side_button(Side::Center, "Center")}
                    {side_button(Side::Right, "Right")}
                </span>
            </div>
            <div class="control-group">
                <label class="control-label" for="bubble-color">"Color"</label>
                <input
                    id="bubble-color"
                    type="color"
                    class="control-color"
                    title="Bubble color"
                    prop:value=color
                    on:input=recolor
                />
            </div>
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
        </header>
    }
}
