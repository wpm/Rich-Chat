//! The Leptos components.
//!
//! [`Chat`] is the whole thing: a transcript above a [`Composer`]. Each
//! piece is also usable alone: [`MessageBubble`] for one message,
//! [`RichText`] for any Markdown, [`CodeBlock`] for one highlighted block.
//! All of them expect the stylesheet, which [`RichChatStyle`] injects.

use std::time::Duration;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::message::Message;
use crate::render::{self, Block, BlockKind, RenderOptions};

/// Injects the crate's stylesheet and, with the `bundled-fonts` feature,
/// the math fonts. Place it once, anywhere in the tree.
#[component]
pub fn RichChatStyle() -> impl IntoView {
    let css = format!(
        "{}\n{}",
        crate::style::STYLESHEET,
        crate::style::font_faces()
    );
    view! { <style inner_html=css></style> }
}

/// Markdown, rendered.
///
/// Re-renders as `content` changes, block by block: only the top-level
/// blocks whose rendered form changed are touched in the DOM. With
/// `draft`, text still being written renders as what it is becoming (see
/// [`complete_draft`](crate::render::complete_draft)).
///
/// Links open in a new browsing context by default. Give `on_link` to
/// intercept them instead: it receives the destination and the default
/// is suppressed, which is how a Tauri app hands links to the system
/// browser. Pass it as `on_link=Callback::new(move |url: String| …)`.
#[component]
pub fn RichText(
    /// The Markdown source.
    #[prop(into)]
    content: Signal<String>,
    /// Render as unfinished text.
    #[prop(optional)]
    draft: bool,
    /// What to render; the default renders everything.
    #[prop(optional)]
    options: RenderOptions,
    /// Called with a link's destination instead of following it.
    #[prop(into, optional_no_strip)]
    on_link: Option<Callback<String>>,
    /// Extra classes for the container, which always has `rc-rich`.
    #[prop(optional, into)]
    class: String,
) -> impl IntoView {
    let mut options = options;
    options.draft |= draft;
    let blocks = Memo::new(move |_| render::render_blocks(&content.read(), &options));
    let class = if class.is_empty() {
        "rc-rich".to_string()
    } else {
        format!("rc-rich {class}")
    };
    view! {
        <div class=class on:click=move |event| intercept_link(&event, on_link)>
            <For each=move || blocks.get() key=|block| block.key.clone() children=render_block />
        </div>
    }
}

fn render_block(block: Block) -> AnyView {
    match block.kind {
        BlockKind::Html(html) => view! { <div class="rc-block" inner_html=html></div> }.into_any(),
        BlockKind::Code {
            language,
            source,
            html,
        } => view! { <CodeBlock language=language source=source html=html /> }.into_any(),
        BlockKind::Math { mathml, .. } => {
            view! { <div class="rc-math-block" inner_html=mathml></div> }.into_any()
        }
    }
}

/// Routes a click on an anchor to `on_link`, when there is one. In-page
/// anchors (footnotes) are left to the browser.
fn intercept_link(event: &ev::MouseEvent, on_link: Option<Callback<String>>) {
    let Some(on_link) = on_link else { return };
    let Some(target) = event.target() else { return };
    let Some(element) = target.dyn_ref::<web_sys::Element>() else {
        return;
    };
    let Ok(Some(anchor)) = element.closest("a[href]") else {
        return;
    };
    let Some(href) = anchor.get_attribute("href") else {
        return;
    };
    if href.starts_with('#') {
        return;
    }
    event.prevent_default();
    on_link.run(href);
}

/// One code block with a language label and a copy button.
///
/// `html` is the escaped, highlighted body that
/// [`highlight`](crate::render::highlight) produces; `source` is the raw
/// text the copy button puts on the clipboard.
#[component]
pub fn CodeBlock(
    /// The code as written.
    #[prop(into)]
    source: String,
    /// The language's display name, if known.
    #[prop(optional_no_strip)]
    language: Option<String>,
    /// The escaped, highlighted body of the `<code>` element.
    #[prop(into)]
    html: String,
) -> impl IntoView {
    let copied = RwSignal::new(false);
    let label = language.unwrap_or_else(|| "text".to_string());
    let copy = move |_| {
        copy_to_clipboard(&source);
        copied.set(true);
        set_timeout(move || copied.set(false), Duration::from_millis(1500));
    };
    view! {
        <div class="rc-codeblock">
            <div class="rc-codeblock-bar">
                <span class="rc-codeblock-language">{label}</span>
                <button
                    type="button"
                    class="rc-copy"
                    class:rc-copied=move || copied.get()
                    aria-label="Copy code"
                    on:click=copy
                >
                    {move || if copied.get() { "Copied" } else { "Copy" }}
                </button>
            </div>
            <pre class="rc-code">
                <code inner_html=html></code>
            </pre>
        </div>
    }
}

fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        // Awaited so that a denied clipboard is a resolved future, not an
        // unhandled rejection in the console.
        let promise = window.navigator().clipboard().write_text(text);
        leptos::task::spawn_local(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        });
    }
}

/// One message in the transcript, on its role's side of the window.
#[component]
pub fn MessageBubble(
    /// The message.
    message: Message,
    /// What to render; the default renders everything.
    #[prop(optional)]
    options: RenderOptions,
    /// Called with a link's destination instead of following it.
    #[prop(into, optional_no_strip)]
    on_link: Option<Callback<String>>,
) -> impl IntoView {
    let class = format!("rc-message rc-message-{}", message.role.as_str());
    view! {
        <div class=class data-message-id=message.id.clone()>
            <div class="rc-bubble">
                <RichText content=message.content draft=message.live options=options on_link=on_link />
            </div>
        </div>
    }
}

/// The input: a growing text box with a live preview above it.
///
/// Enter sends and Shift+Enter breaks a line; an IME composition in
/// progress is never sent. The preview renders the draft as it is typed,
/// with unfinished constructs closed for display, and disappears when
/// the box is empty.
#[component]
pub fn Composer(
    /// Receives the text of each message sent.
    #[prop(into)]
    on_send: Callback<String>,
    /// The text box's placeholder.
    #[prop(default = "Write a message…".to_string(), into)]
    placeholder: String,
    /// Blocks sending while true; the draft is kept.
    #[prop(optional, into)]
    disabled: Signal<bool>,
    /// Show the live preview.
    #[prop(default = true)]
    preview: bool,
    /// What the preview renders; the default renders everything.
    #[prop(optional)]
    options: RenderOptions,
    /// Called with a link's destination instead of following it.
    #[prop(into, optional_no_strip)]
    on_link: Option<Callback<String>>,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());
    let input = NodeRef::<html::Textarea>::new();

    let fit = move || {
        if let Some(element) = input.get_untracked() {
            let style = web_sys::HtmlElement::style(&element);
            let _ = style.set_property("height", "auto");
            let _ = style.set_property("height", &format!("{}px", element.scroll_height()));
        }
    };
    let submit = move || {
        if disabled.get_untracked() {
            return;
        }
        let text = draft.get_untracked();
        if text.trim().is_empty() {
            return;
        }
        on_send.run(text);
        draft.set(String::new());
        request_animation_frame(fit);
    };
    let has_draft = move || !draft.read().trim().is_empty();

    view! {
        <div class="rc-composer">
            <Show when=move || preview && has_draft()>
                <div class="rc-composer-preview" aria-live="polite">
                    <div class="rc-composer-preview-label">"Preview"</div>
                    <RichText content=draft draft=true options=options.clone() on_link=on_link />
                </div>
            </Show>
            <div class="rc-composer-row">
                <textarea
                    node_ref=input
                    class="rc-composer-input"
                    rows="1"
                    placeholder=placeholder
                    aria-label="Message"
                    prop:value=move || draft.get()
                    disabled=move || disabled.get()
                    on:input=move |event| {
                        draft.set(event_target_value(&event));
                        fit();
                    }
                    on:keydown=move |event: ev::KeyboardEvent| {
                        if event.key() == "Enter" && !event.shift_key() && !event.is_composing() {
                            event.prevent_default();
                            submit();
                        }
                    }
                ></textarea>
                <button
                    type="button"
                    class="rc-send"
                    aria-label="Send"
                    disabled=move || disabled.get() || !has_draft()
                    on:click=move |_| submit()
                >
                    "Send"
                </button>
            </div>
            <div class="rc-composer-hint">
                "Enter to send, Shift+Enter for a new line. Markdown, $math$ and ```code``` render as you type."
            </div>
        </div>
    }
}

/// A chat window: the transcript, then a [`Composer`].
///
/// The transcript follows new messages and growing ones, unless the
/// reader has scrolled up to look at something, in which case it stays
/// put until they return to the bottom.
#[component]
pub fn Chat(
    /// The transcript, oldest first.
    #[prop(into)]
    messages: Signal<Vec<Message>>,
    /// Receives the text of each message sent.
    #[prop(into)]
    on_send: Callback<String>,
    /// The text box's placeholder.
    #[prop(default = "Write a message…".to_string(), into)]
    placeholder: String,
    /// Blocks sending while true.
    #[prop(optional, into)]
    disabled: Signal<bool>,
    /// Show the live preview in the composer.
    #[prop(default = true)]
    preview: bool,
    /// What to render; the default renders everything.
    #[prop(optional)]
    options: RenderOptions,
    /// Called with a link's destination instead of following it.
    #[prop(into, optional_no_strip)]
    on_link: Option<Callback<String>>,
    /// Shown in the transcript while it is empty.
    #[prop(optional, into)]
    empty: Option<String>,
) -> impl IntoView {
    let pane = NodeRef::<html::Div>::new();
    let pinned = RwSignal::new(true);

    Effect::new(move |_| {
        for message in messages.read().iter() {
            message.content.track();
        }
        if pinned.get_untracked() {
            request_animation_frame(move || {
                if let Some(pane) = pane.get_untracked() {
                    pane.set_scroll_top(pane.scroll_height());
                }
            });
        }
    });

    // The grammar tables deserialize on first use; do that in a moment of
    // quiet rather than on the first keystroke into a code fence.
    #[cfg(target_arch = "wasm32")]
    set_timeout(render::preload, Duration::from_millis(250));

    let bubble_options = options.clone();
    view! {
        <div class="rc-chat">
            <div
                class="rc-messages"
                node_ref=pane
                on:scroll=move |_| {
                    if let Some(pane) = pane.get_untracked() {
                        let gap = pane.scroll_height() - pane.scroll_top() - pane.client_height();
                        pinned.set(gap <= 48);
                    }
                }
            >
                <Show when=move || messages.read().is_empty()>
                    <div class="rc-empty">{empty.clone().unwrap_or_default()}</div>
                </Show>
                <For
                    each=move || messages.get()
                    key=|message| (message.id.clone(), message.live)
                    children=move |message| {
                        view! { <MessageBubble message=message options=bubble_options.clone() on_link=on_link /> }
                    }
                />
            </div>
            <Composer
                on_send=on_send
                placeholder=placeholder
                disabled=disabled
                preview=preview
                options=options
                on_link=on_link
            />
        </div>
    }
}
