//! The bar above the chat: the users, the selected user's bubbles,
//! whether bubbles are named and how big the name is, whether the chat
//! is busy or disabled, the theme.
//!
//! Each control edits the app's [`Settings`]; the app root turns those
//! into the library's table of names and its `show_names`, the root
//! element's theme attribute, and custom properties for the stylesheet.
//! The Busy and Disabled switches are the exception: each is a state of
//! the conversation, not a choice to keep, so it is a signal of its own
//! and starts off.

use leptos::ev;
use leptos::prelude::*;

use crate::settings::{
    SENDER_SIZE_MAX, SENDER_SIZE_MIN, SENDER_SIZE_STEP, Settings, Side, Theme, User,
    parse_sender_size,
};

/// The controls. `theme` is the one in effect, which is the chosen one
/// or, until one is chosen, the system's. `busy` and `disabled` are the
/// chat's: there is no model here to wait on, nor a key to be without,
/// so the switches stand in for them.
#[component]
pub fn Controls(
    settings: RwSignal<Settings>,
    theme: Signal<Theme>,
    busy: RwSignal<bool>,
    disabled: RwSignal<bool>,
) -> impl IntoView {
    let dark = move || theme.get() == Theme::Dark;
    let toggle_theme = move |_| {
        let next = theme.get_untracked().toggled();
        settings.update(|settings| settings.theme = Some(next));
    };

    // The users, and the one the other controls edit. Memos, so that a
    // settings change that touches neither (the text box's height, as
    // it is dragged) re-renders neither.
    let users = Memo::new(move |_| settings.read().users().to_vec());
    let selected = Memo::new(move |_| settings.read().selected_user().clone());
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

    // Whether the names are shown and how big they are. Memos, as above.
    let show_names = Memo::new(move |_| settings.read().show_names);
    let toggle_names =
        move |_| settings.update(|settings| settings.show_names = !settings.show_names);
    // The slider is live only while there is a name for it to size.
    let sender_size = Memo::new(move |_| settings.read().sender_size);
    let resize_names = move |event: ev::Event| {
        if let Some(size) = parse_sender_size(&event_target_value(&event)) {
            settings.update(|settings| settings.sender_size = size);
        }
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
            <div class="control-group" role="group" aria-label="Names">
                <button
                    type="button"
                    class="control-button control-toggle control-names"
                    role="switch"
                    aria-checked=move || show_names.get().to_string()
                    title="Write the user's name over every bubble"
                    on:click=toggle_names
                >
                    "Names"
                </button>
                <input
                    type="range"
                    class="control-sender-size"
                    aria-label="Name size"
                    title="How big the name over a bubble is"
                    min=SENDER_SIZE_MIN
                    max=SENDER_SIZE_MAX
                    step=SENDER_SIZE_STEP
                    prop:value=move || sender_size.get().to_string()
                    aria-valuetext=move || format!("{} em", sender_size.get())
                    disabled=move || !show_names.get()
                    on:input=resize_names
                />
            </div>
            <button
                type="button"
                class="control-button control-toggle control-busy"
                role="switch"
                aria-checked=move || busy.get().to_string()
                title="Hold what is sent, as a host waiting on a reply does; the text box stays open"
                on:click=move |_| busy.update(|busy| *busy = !*busy)
            >
                "Busy"
            </button>
            <button
                type="button"
                class="control-button control-toggle control-disabled"
                role="switch"
                aria-checked=move || disabled.get().to_string()
                title="Turn the composer off, as a host with nothing to send with does"
                on:click=move |_| disabled.update(|disabled| *disabled = !*disabled)
            >
                "Disabled"
            </button>
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
