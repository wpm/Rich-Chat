//! The Leptos components.
//!
//! [`Chat`] is the whole thing: a transcript above a [`Composer`]. Each
//! piece is also usable alone: [`MessageBubble`] for one message,
//! [`RichText`] for any Markdown, [`CodeBlock`] for one highlighted block.
//! All of them expect at least the structure stylesheet, which
//! [`RichChatStyle`] injects; see [`style`](crate::style) for what a host
//! can replace.

use std::time::Duration;

use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::message::Message;
use crate::render::{self, Block, BlockKind, RenderOptions};

/// Injects the crate's stylesheets and, with the `bundled-fonts` feature,
/// the math fonts. Place it once, anywhere in the tree.
///
/// [`style::STRUCTURE`](crate::style::STRUCTURE) always goes in; it is
/// what the components need to work and has no opinion on looks. The
/// rest is switchable, for a host that writes its own theme, its own code
/// colours, or serves the fonts itself. Everything is in cascade layers,
/// so a host's own unlayered rules win over it regardless of specificity;
/// see [`style`](crate::style).
#[component]
pub fn RichChatStyle(
    /// The default look, [`style::THEME`](crate::style::THEME).
    #[prop(default = true)]
    theme: bool,
    /// The code colours, [`style::HIGHLIGHT`](crate::style::HIGHLIGHT).
    #[prop(default = true)]
    highlight: bool,
    /// The `@font-face` rules for the bundled math fonts,
    /// [`style::font_faces`](crate::style::font_faces).
    #[prop(default = true)]
    fonts: bool,
) -> impl IntoView {
    let mut css = String::from(crate::style::STRUCTURE);
    for (wanted, part) in [
        (theme, crate::style::THEME),
        (highlight, crate::style::HIGHLIGHT),
        (fonts, crate::style::font_faces()),
    ] {
        if wanted {
            css.push('\n');
            css.push_str(part);
        }
    }
    view! { <style inner_html=css></style> }
}

/// The copy button's labels.
///
/// [`CodeBlock`] takes them as a prop. The code blocks that [`RichText`]
/// renders inside Markdown read them from context instead, so provide
/// one above the tree to change them everywhere:
///
/// ```ignore
/// provide_context(CodeLabels { copy: "Copier".into(), copied: "Copié".into() });
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeLabels {
    /// The button at rest.
    pub copy: String,
    /// The button for a moment after a copy.
    pub copied: String,
}

impl Default for CodeLabels {
    fn default() -> Self {
        Self {
            copy: "Copy".into(),
            copied: "Copied".into(),
        }
    }
}

/// Markdown, rendered.
///
/// Re-renders as `content` changes, block by block: only the top-level
/// blocks whose rendered form changed are touched in the DOM. With
/// `draft`, text still being written renders as what it is becoming (see
/// [`complete_draft`](crate::render::complete_draft)).
///
/// Each block is wrapped in a `div.rc-block`, which the structure
/// stylesheet sets to `display: contents` so the wrapper is not a box. A
/// host that styles `.rc-block` itself must keep that, or paragraphs stop
/// collapsing margins and sibling selectors stop seeing each other.
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
    /// The copy button's labels; the default is a [`CodeLabels`] from
    /// context, or "Copy" and "Copied".
    #[prop(optional)]
    labels: Option<CodeLabels>,
) -> impl IntoView {
    let copied = RwSignal::new(false);
    let label = language.unwrap_or_else(|| "text".to_string());
    let labels = labels
        .or_else(use_context::<CodeLabels>)
        .unwrap_or_default();
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
                    {move || {
                        if copied.get() { labels.copied.clone() } else { labels.copy.clone() }
                    }}
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

/// The text under the composer unless a host gives its own.
const DEFAULT_HINT: &str = "Enter to send, Shift+Enter for a new line. Markdown, $math$ and ```code``` render as you type.";

/// The send button's content: the host's, or the word.
fn send_content(send: Option<ViewFn>) -> ViewFn {
    send.unwrap_or_else(|| ViewFn::from(|| "Send"))
}

/// The line under the text box: the host's, or the default.
fn hint_text(hint: Option<String>) -> String {
    hint.unwrap_or_else(|| DEFAULT_HINT.to_string())
}

/// The input: a growing text box with a live preview above it.
///
/// Enter sends and Shift+Enter breaks a line; an IME composition in
/// progress is never sent. The preview renders the draft as it is typed,
/// with unfinished constructs closed for display, and disappears when
/// the box is empty.
///
/// The text box grows with its draft: on every input its inline height
/// is set to its scroll height. The structure stylesheet gives it the box
/// model and the cap that make that measurement right.
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
    /// The heading over the preview.
    #[prop(default = "Preview".to_string(), into)]
    preview_label: String,
    /// The send button's content, in place of the word "Send": an icon,
    /// say. Pass a view function: `send=|| view! { <SendIcon /> }`.
    #[prop(optional, into)]
    send: Option<ViewFn>,
    /// The line under the text box. The default explains the keys; an
    /// empty string leaves the line out.
    #[prop(optional, into)]
    hint: Option<String>,
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
    let send = send_content(send);
    let hint = hint_text(hint);

    view! {
        <div class="rc-composer">
            <Show when=move || preview && has_draft()>
                <div class="rc-composer-preview" aria-live="polite">
                    <div class="rc-composer-preview-label">{preview_label.clone()}</div>
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
                    {send.run()}
                </button>
            </div>
            {(!hint.is_empty()).then(|| view! { <div class="rc-composer-hint">{hint}</div> })}
        </div>
    }
}

/// Compiles the grammars of the languages in
/// [`WARM_LANGUAGES`](crate::render::WARM_LANGUAGES), one per idle
/// callback, so the first code block in any of them renders without the
/// pause that compiling a grammar on demand costs (up to half a second for
/// TypeScript). [`Chat`] calls this after mounting; a host that uses
/// [`RichText`] on its own can call it once at startup. Where the browser
/// has no `requestIdleCallback`, the steps run on short timeouts instead.
/// A no-op outside the browser or without the `highlight` feature.
pub fn warm_up() {
    #[cfg(target_arch = "wasm32")]
    warm_from(0);
}

#[cfg(target_arch = "wasm32")]
fn warm_from(index: usize) {
    let Some(token) = render::WARM_LANGUAGES.get(index) else {
        return;
    };
    let step = move || {
        render::warm(token);
        warm_from(index + 1);
    };
    if request_idle_callback_with_handle(step).is_err() {
        set_timeout(step, Duration::from_millis(200));
    }
}

/// A chat window: the transcript, then a [`Composer`].
///
/// The transcript follows new messages and growing ones, unless the
/// reader has scrolled up to look at something, in which case it stays
/// put until they return to the bottom.
///
/// That works by scrolling the transcript, `div.rc-messages`, which the
/// structure stylesheet makes the scroll container inside a `.rc-chat`
/// that fills its parent. A host that lays the window out itself must
/// keep `.rc-messages` the element that scrolls; if the page scrolls
/// instead, nothing follows.
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
    /// The heading over the preview.
    #[prop(default = "Preview".to_string(), into)]
    preview_label: String,
    /// The send button's content, in place of the word "Send". See
    /// [`Composer`].
    #[prop(optional, into)]
    send: Option<ViewFn>,
    /// The line under the text box; empty leaves it out. See [`Composer`].
    #[prop(optional, into)]
    hint: Option<String>,
    /// What to render; the default renders everything.
    #[prop(optional)]
    options: RenderOptions,
    /// Called with a link's destination instead of following it.
    #[prop(into, optional_no_strip)]
    on_link: Option<Callback<String>>,
    /// Shown in the transcript while it is empty.
    #[prop(optional, into)]
    empty: Option<String>,
    /// Compile the common languages' grammars in idle time after mount,
    /// so the first code block renders without a pause. See [`warm_up`].
    #[prop(default = true)]
    warm_up: bool,
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

    // Browser only: the timer does not exist off it, and a server render
    // has no grammars to warm.
    #[cfg(target_arch = "wasm32")]
    if warm_up {
        set_timeout(self::warm_up, Duration::from_millis(250));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = warm_up;

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
                preview_label=preview_label
                send=send_content(send)
                hint=hint_text(hint)
                options=options
                on_link=on_link
            />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Role;

    /// Renders a view to its HTML string, the way a server would, under a
    /// reactive owner so that `For` and `Show` have somewhere to live.
    fn html<V: IntoView>(build: impl FnOnce() -> V) -> String {
        Owner::new().with(|| build().into_view().to_html())
    }

    #[test]
    fn rich_text_renders_keyed_blocks() {
        let out = html(|| view! { <RichText content="# Title\n\nBody with $x$.".to_string() /> });
        assert!(out.starts_with("<div class=\"rc-rich\""), "{out}");
        assert_eq!(out.matches("<div class=\"rc-block\"").count(), 2, "{out}");
        assert!(out.contains("<h1>Title</h1>"), "{out}");
        assert!(out.contains("<math"), "{out}");
    }

    #[test]
    fn rich_text_extra_class_and_draft() {
        let out =
            html(|| view! { <RichText content="so $x".to_string() draft=true class="mine" /> });
        assert!(out.contains("class=\"rc-rich mine\""), "{out}");
        assert!(out.contains("<math"), "{out}");
        let plain = html(|| view! { <RichText content="so $x".to_string() /> });
        assert!(!plain.contains("<math"), "{plain}");
    }

    #[test]
    fn code_blocks_become_the_component_with_a_copy_button() {
        let out = html(|| view! { <RichText content="```rust\nfn main() {}\n```".to_string() /> });
        assert!(out.contains("class=\"rc-codeblock\""), "{out}");
        assert!(out.contains("class=\"rc-copy\""), "{out}");
        assert!(out.contains("aria-label=\"Copy code\""), "{out}");
        assert!(out.contains("<pre class=\"rc-code\"><code>"), "{out}");
        if cfg!(feature = "highlight") {
            assert!(
                out.contains("<span class=\"rc-codeblock-language\">Rust</span>"),
                "{out}"
            );
        }
    }

    #[test]
    fn code_block_without_a_language_says_text() {
        let out = html(|| view! { <CodeBlock source="x" html="x" /> });
        assert!(out.contains(">text</span>"), "{out}");
        assert!(out.contains("<code>x</code>"), "{out}");
    }

    #[test]
    fn display_math_becomes_a_math_block() {
        let out = html(|| view! { <RichText content="$$\\int x$$".to_string() /> });
        assert!(
            out.contains("<div class=\"rc-math-block\"><math display=\"block\""),
            "{out}"
        );
    }

    #[test]
    fn bubbles_carry_role_and_id() {
        for (role, class) in [
            (Role::User, "rc-message-user"),
            (Role::Assistant, "rc-message-assistant"),
            (Role::System, "rc-message-system"),
        ] {
            let message = Message::new("m7", role, "hi");
            let out = html(|| view! { <MessageBubble message=message /> });
            assert!(
                out.contains(&format!("class=\"rc-message {class}\"")),
                "{out}"
            );
            assert!(out.contains("data-message-id=\"m7\""), "{out}");
            assert!(out.contains("<div class=\"rc-bubble\">"), "{out}");
            assert!(out.contains("<p>hi</p>"), "{out}");
        }
    }

    #[test]
    fn a_live_bubble_renders_as_a_draft() {
        let text = RwSignal::new(String::from("so $x^2"));
        let live = html(
            || view! { <MessageBubble message=Message::streaming("s", Role::Assistant, text) /> },
        );
        assert!(live.contains("<math"), "{live}");
        let done = html(
            || view! { <MessageBubble message=Message::streaming("s", Role::Assistant, text).finished() /> },
        );
        assert!(!done.contains("<math"), "{done}");
    }

    #[test]
    fn composer_has_the_controls_and_no_preview_when_empty() {
        let out = html(|| view! { <Composer on_send=|_text: String| {} placeholder="Say it" /> });
        assert!(out.contains("<textarea"), "{out}");
        assert!(out.contains("placeholder=\"Say it\""), "{out}");
        assert!(out.contains("class=\"rc-send\""), "{out}");
        assert!(out.contains(">Send</button>"), "{out}");
        assert!(out.contains("rc-composer-hint"), "{out}");
        assert!(out.contains("Enter to send"), "{out}");
        assert!(!out.contains("rc-composer-preview"), "{out}");
    }

    #[test]
    fn composer_takes_its_own_send_content_and_hint() {
        let out = html(|| {
            view! {
                <Composer
                    on_send=|_text: String| {}
                    send=|| view! { <span class="icon">"→"</span> }
                    hint="Ctrl+Enter sends"
                />
            }
        });
        assert!(
            out.contains("<span class=\"icon\">→</span></button>"),
            "{out}"
        );
        assert!(!out.contains(">Send<"), "{out}");
        assert!(
            out.contains("<div class=\"rc-composer-hint\">Ctrl+Enter sends</div>"),
            "{out}"
        );
        let bare = html(|| view! { <Composer on_send=|_text: String| {} hint="" /> });
        assert!(!bare.contains("rc-composer-hint"), "{bare}");
    }

    #[test]
    fn copy_labels_come_from_the_prop_or_the_context() {
        let plain = html(|| view! { <CodeBlock source="x" html="x" /> });
        assert!(plain.contains(">Copy</button>"), "{plain}");

        let labels = CodeLabels {
            copy: "Copier".into(),
            copied: "Copié".into(),
        };
        let given = labels.clone();
        let prop = html(|| view! { <CodeBlock source="x" html="x" labels=given /> });
        assert!(prop.contains(">Copier</button>"), "{prop}");

        let context = html(|| {
            provide_context(labels.clone());
            view! { <RichText content="```\nx\n```".to_string() /> }
        });
        assert!(context.contains(">Copier</button>"), "{context}");
    }

    #[test]
    fn chat_shows_the_empty_text_then_the_messages() {
        let messages = RwSignal::new(Vec::<Message>::new());
        let empty = html(
            || view! { <Chat messages=messages on_send=|_: String| {} empty="Nothing yet" /> },
        );
        assert!(
            empty.contains("<div class=\"rc-empty\">Nothing yet</div>"),
            "{empty}"
        );
        assert!(empty.contains("<div class=\"rc-composer\">"), "{empty}");

        messages.set(vec![
            Message::new("1", Role::User, "Hello"),
            Message::new("2", Role::Assistant, "Hi *there*"),
        ]);
        let full = html(
            || view! { <Chat messages=messages on_send=|_: String| {} empty="Nothing yet" /> },
        );
        assert!(!full.contains("Nothing yet"), "{full}");
        assert!(full.contains("rc-message-user"), "{full}");
        assert!(full.contains("<p>Hi <em>there</em></p>"), "{full}");
        let user = full.find("rc-message-user").unwrap();
        let assistant = full.find("rc-message-assistant").unwrap();
        assert!(user < assistant, "messages keep their order");
    }

    #[test]
    fn style_component_carries_the_stylesheet_and_fonts() {
        let out = html(|| view! { <RichChatStyle /> });
        assert!(out.starts_with("<style>"), "{}", &out[..40]);
        assert!(
            out.contains("@layer rich-chat.structure {"),
            "structure missing"
        );
        assert!(out.contains("@layer rich-chat.theme {"), "theme missing");
        assert!(out.contains("--rc-accent:"), "palette missing");
        assert!(out.contains(".rc-rich mtable"), "math rules missing");
        assert!(out.contains(".rc-keyword"), "highlighting missing");
        if cfg!(feature = "bundled-fonts") {
            assert!(out.contains("data:font/woff2;base64,"), "fonts missing");
        }
        // Raw CSS, not entity-escaped: selectors with `>` must survive.
        assert!(out.contains(" > "), "CSS was escaped");
    }

    #[test]
    fn style_component_leaves_out_what_is_switched_off() {
        let out = html(|| view! { <RichChatStyle theme=false highlight=false fonts=false /> });
        assert!(
            out.contains("@layer rich-chat.structure {"),
            "structure missing"
        );
        assert!(out.contains(".rc-block {"), "{out}");
        assert!(!out.contains("--rc-accent:"), "theme leaked in");
        assert!(!out.contains(".rc-keyword"), "highlighting leaked in");
        assert!(!out.contains("@font-face"), "fonts leaked in");
    }

    #[test]
    fn warm_up_is_a_no_op_off_the_browser() {
        warm_up();
    }
}
