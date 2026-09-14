//! The test window: one chat, no model. Every message sent appears as a
//! bubble, which is all that is needed to exercise the rendering.

mod opener;

use leptos::prelude::*;
use rich_chat::{Chat, Message, RichChatStyle, Role};

#[component]
fn App() -> impl IntoView {
    let messages = RwSignal::new(Vec::<Message>::new());
    let send = move |text: String| {
        let id = format!("m{}", messages.read_untracked().len());
        messages.update(|all| all.push(Message::new(id, Role::User, text)));
    };
    view! {
        <RichChatStyle />
        <main class="app">
            <Chat
                messages=messages
                on_send=send
                on_link=Callback::new(opener::open_url)
                empty="Write Markdown, $\\LaTeX$, or a ```code``` fence below. It renders as you type and lands here as a bubble."
            />
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
