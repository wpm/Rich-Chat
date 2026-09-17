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

use crate::kinds::Kinds;
use crate::message::Message;
use crate::render::{self, Block, BlockKind, RenderOptions};

/// Injects the crate's stylesheets and, with the `bundled-fonts` feature,
/// the math fonts. Place it once, anywhere in the tree.
///
/// [`style::STRUCTURE`](crate::style::STRUCTURE) always goes in; it is
/// what the components need to work and has no opinion on looks. The
/// rest is switchable, for a host that writes its own theme, its own code
/// colors, or serves the fonts itself. Everything is in cascade layers,
/// so a host's own unlayered rules win over it regardless of specificity;
/// see [`style`](crate::style).
///
/// The rules for the host's kinds of message come from `kinds`, which
/// may be a signal: only they are rewritten when it changes, in a second
/// `<style>` element, so the fonts and the theme are injected once.
#[component]
pub fn RichChatStyle(
    /// The default look, [`style::THEME`](crate::style::THEME).
    #[prop(default = true)]
    theme: bool,
    /// The code colors, [`style::HIGHLIGHT`](crate::style::HIGHLIGHT).
    #[prop(default = true)]
    highlight: bool,
    /// The `@font-face` rules for the bundled math fonts,
    /// [`style::font_faces`](crate::style::font_faces).
    #[prop(default = true)]
    fonts: bool,
    /// Where each kind of message sits and its colors. The default is
    /// [`Kinds::default`]; [`Kinds::none`] leaves every bubble plain.
    #[prop(default = Signal::stored(Kinds::default()), into)]
    kinds: Signal<Kinds>,
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
    view! {
        <style inner_html=css></style>
        <style inner_html=move || kinds.read().css()></style>
    }
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

/// One message in the transcript. The outer `div.rc-message` carries the
/// message's kind as `data-kind`, which the host's [`Kinds`] rules place
/// and color.
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
    /// A reference to the outer `div.rc-message`.
    #[prop(optional)]
    node_ref: NodeRef<html::Div>,
) -> impl IntoView {
    view! {
        <div
            class="rc-message"
            data-kind=message.kind.clone()
            data-message-id=message.id.clone()
            node_ref=node_ref
        >
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

/// Sends the draft and leaves the box empty; not while sending is
/// disabled, and never a blank one, which stays as it is.
fn send_draft(disabled: bool, draft: RwSignal<String>, on_send: Callback<String>) {
    if disabled || draft.read_untracked().trim().is_empty() {
        return;
    }
    // Taken, not copied: the signal is left empty by the same write.
    on_send.run(std::mem::take(&mut *draft.write()));
}

/// Whether a key press in the text box sends: Enter on its own, not
/// Shift+Enter, and never in the middle of an IME composition.
fn enter_sends(key: &str, shift: bool, composing: bool) -> bool {
    key == "Enter" && !shift && !composing
}

/// The input: a growing text box with a live preview above it.
///
/// Enter sends and Shift+Enter breaks a line; an IME composition in
/// progress is never sent. The preview renders the draft as it is typed,
/// with unfinished constructs closed for display, and disappears when
/// the box is empty. A button in its heading collapses it to that
/// heading, for a draft whose preview (a long run of equations, say)
/// would crowd the window; it opens expanded, and the choice holds until
/// the composer is unmounted.
///
/// The text box grows with its draft: on every input its inline height
/// is set to its scroll height. The structure stylesheet gives it the box
/// model and the cap that make that measurement right.
///
/// The draft is the composer's own unless the host gives one as `draft`,
/// a signal it holds: to prefill the box (a quoted reply), to read what
/// is being typed, or to keep a draft across the composer's unmounting.
/// Sending clears it either way.
#[component]
pub fn Composer(
    /// Receives the text of each message sent.
    #[prop(into)]
    on_send: Callback<String>,
    /// The text in the box, held by the host. The default is a signal of
    /// the composer's own, starting empty.
    #[prop(optional)]
    draft: RwSignal<String>,
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
    /// The kind of message the preview shows, so that it takes that
    /// kind's colors from the [`Kinds`] rules. Empty, the default,
    /// leaves it in the theme's tint colors. May be a signal, for a
    /// host whose sender changes.
    #[prop(optional, into)]
    preview_kind: Signal<String>,
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
    let input = NodeRef::<html::Textarea>::new();

    let fit = move || {
        if let Some(element) = input.get_untracked() {
            let style = web_sys::HtmlElement::style(&element);
            let _ = style.set_property("height", "auto");
            let _ = style.set_property("height", &format!("{}px", element.scroll_height()));
        }
    };
    let submit = move || send_draft(disabled.get_untracked(), draft, on_send);
    // The height follows the text however it changes: sent, or put in
    // by the host, at the start or later. Browser only, as the frame is.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        draft.track();
        request_animation_frame(fit);
    });
    let has_draft = move || !draft.read().trim().is_empty();
    let send = send_content(send);
    let hint = hint_text(hint);
    let preview_kind = move || {
        let kind = preview_kind.get();
        (!kind.is_empty()).then_some(kind)
    };
    let expanded = RwSignal::new(true);
    let options = StoredValue::new(options);

    view! {
        <div class="rc-composer">
            <Show when=move || preview && has_draft()>
                <div
                    class="rc-composer-preview"
                    class:rc-collapsed=move || !expanded.get()
                    data-kind=preview_kind
                    aria-live="polite"
                >
                    <div class="rc-composer-preview-label">
                        <span>{preview_label.clone()}</span>
                        <button
                            type="button"
                            class="rc-preview-toggle"
                            aria-expanded=move || expanded.get().to_string()
                            aria-label=move || {
                                if expanded.get() { "Collapse preview" } else { "Expand preview" }
                            }
                            on:click=move |_| expanded.update(|open| *open = !*open)
                        >
                            {move || if expanded.get() { "\u{25BE}" } else { "\u{25B8}" }}
                        </button>
                    </div>
                    <Show when=move || expanded.get()>
                        <RichText content=draft draft=true options=options.get_value() on_link=on_link />
                    </Show>
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
                        if enter_sends(&event.key(), event.shift_key(), event.is_composing()) {
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
/// The transcript keeps its end in view: new messages, ones still
/// growing, images and fonts that arrive late, and a composer or window
/// that changes size all leave the last bubble showing. It stops
/// following once the reader scrolls up to look at something, and
/// resumes when they come back to the end.
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
    /// The text in the box, held by the host. See [`Composer`].
    #[prop(optional)]
    draft: RwSignal<String>,
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
    /// The kind of message the preview shows, for its colors. See
    /// [`Composer`].
    #[prop(optional, into)]
    preview_kind: Signal<String>,
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
    let follow = Follow::new(pane);
    follow.watch(pane);

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
            <div class="rc-messages" node_ref=pane on:scroll=move |_| follow.scrolled()>
                <Show when=move || messages.read().is_empty()>
                    <div class="rc-empty">{empty.clone().unwrap_or_default()}</div>
                </Show>
                <For
                    each=move || messages.get()
                    key=|message| (message.id.clone(), message.live)
                    children=move |message| {
                        let wrapper = NodeRef::<html::Div>::new();
                        follow.watch(wrapper);
                        view! {
                            <MessageBubble
                                message=message
                                options=bubble_options.clone()
                                on_link=on_link
                                node_ref=wrapper
                            />
                        }
                    }
                />
            </div>
            <Composer
                on_send=on_send
                draft=draft
                placeholder=placeholder
                disabled=disabled
                preview=preview
                preview_label=preview_label
                preview_kind=preview_kind
                send=send_content(send)
                hint=hint_text(hint)
                options=options
                on_link=on_link
            />
        </div>
    }
}

/// How far from the end of the transcript, in CSS pixels, still counts
/// as at it.
const END_SLACK: i32 = 48;

/// Keeps the transcript's end in view while the reader is at it.
///
/// The reader is "at the end" until they scroll up, and again once they
/// scroll back to within [`END_SLACK`] of it. A scroll event is judged
/// by its direction, not by where it leaves the transcript: the event
/// arrives a frame after the scroll it reports, and by then a bubble may
/// have arrived and pushed the end further away. That is growth, not the
/// reader leaving, and reading it as leaving is how a transcript loses a
/// burst of messages.
///
/// The scrolling itself is driven by a `ResizeObserver` on the pane and
/// on every bubble: a bubble arriving, growing (a stream of content, an
/// image or a font loading late) or going away, or the pane changing
/// size under a growing composer or a resized window, each scroll the
/// pane to its end while the reader is there. The observer reports after
/// layout, so the pane is at its end before the frame paints.
#[derive(Clone, Copy)]
struct Follow {
    pane: NodeRef<html::Div>,
    /// The reader is at the end.
    at_end: StoredValue<bool>,
    /// Where the last scroll event left `scrollTop`.
    last_top: StoredValue<i32>,
    /// Made on the first element watched, in the browser only.
    observer: StoredValue<Option<Observer>, LocalStorage>,
}

/// A `ResizeObserver` with the closure it calls, which has to outlive
/// every report.
struct Observer {
    inner: web_sys::ResizeObserver,
    _callback: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

impl Follow {
    fn new(pane: NodeRef<html::Div>) -> Self {
        let follow = Self {
            pane,
            at_end: StoredValue::new(true),
            last_top: StoredValue::new(0),
            observer: StoredValue::new_local(None),
        };
        on_cleanup(move || {
            follow.observer.update_value(|observer| {
                if let Some(observer) = observer.take() {
                    observer.inner.disconnect();
                }
            });
        });
        follow
    }

    /// Scrolls to the end on every change of the element's size, for as
    /// long as the current reactive owner lives.
    fn watch(self, node: NodeRef<html::Div>) {
        node.on_load(move |element| {
            self.observer.update_value(|observer| {
                let observer = match observer {
                    Some(observer) => observer,
                    None => match self.make_observer() {
                        Some(made) => observer.insert(made),
                        None => return,
                    },
                };
                observer.inner.observe(&element);
            });
        });
        on_cleanup(move || {
            if let Some(element) = node.get_untracked() {
                self.observer.with_value(|observer| {
                    if let Some(observer) = observer {
                        observer.inner.unobserve(&element);
                    }
                });
            }
        });
    }

    fn make_observer(self) -> Option<Observer> {
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            if self.at_end.get_value() {
                self.to_end();
            }
        });
        let inner = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()).ok()?;
        Some(Observer {
            inner,
            _callback: callback,
        })
    }

    /// The pane scrolled: by the reader, or by [`to_end`](Self::to_end).
    fn scrolled(self) {
        let Some(pane) = self.pane.get_untracked() else {
            return;
        };
        let top = pane.scroll_top();
        let gap = pane.scroll_height() - top - pane.client_height();
        if gap <= END_SLACK {
            self.at_end.set_value(true);
        } else if top < self.last_top.get_value() {
            self.at_end.set_value(false);
        }
        self.last_top.set_value(top);
    }

    fn to_end(self) {
        if let Some(pane) = self.pane.get_untracked() {
            pane.set_scroll_top(pane.scroll_height());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::{Look, Position};

    /// Renders a view to its HTML string, the way a server would, under a
    /// reactive owner so that `For` and `Show` have somewhere to live.
    ///
    /// Building the view creates effects (`NodeRef::on_load` is one) that
    /// are inert in a plain test build but spawn a task when Leptos's
    /// `effects` feature is on, which a workspace-wide build turns on by
    /// unifying the app's `csr` feature into this crate. An executor to
    /// spawn on keeps the render from panicking either way; the tasks are
    /// never polled, which is what a server render wants.
    fn html<V: IntoView>(build: impl FnOnce() -> V) -> String {
        // Err means one is already set, by an earlier test in this process.
        let _ = any_spawner::Executor::init_futures_executor();
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
    fn bubbles_carry_kind_and_id() {
        for kind in ["user", "alice", "a kind \"quoted\""] {
            let message = Message::new("m7", kind, "hi");
            let out = html(|| view! { <MessageBubble message=message /> });
            assert!(out.starts_with("<div "), "{out}");
            assert!(out.contains(" class=\"rc-message\">"), "{out}");
            let escaped = kind.replace('"', "&quot;");
            assert!(out.contains(&format!("data-kind=\"{escaped}\"")), "{out}");
            assert!(out.contains("data-message-id=\"m7\""), "{out}");
            assert!(out.contains("<div class=\"rc-bubble\">"), "{out}");
            assert!(out.contains("<p>hi</p>"), "{out}");
        }
    }

    #[test]
    fn a_live_bubble_renders_as_a_draft() {
        let text = RwSignal::new(String::from("so $x^2"));
        let live =
            html(|| view! { <MessageBubble message=Message::streaming("s", "bob", text) /> });
        assert!(live.contains("<math"), "{live}");
        let done = html(
            || view! { <MessageBubble message=Message::streaming("s", "bob", text).finished() /> },
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
    fn composer_previews_the_hosts_draft_with_the_preview_kind() {
        let draft = RwSignal::new(String::from("so $x^2"));
        let out = html(|| {
            view! {
                <Composer on_send=|_text: String| {} draft=draft preview_kind="me" preview_label="Draft" />
            }
        });
        assert!(
            out.contains(
                "<div data-kind=\"me\" aria-live=\"polite\" class=\"rc-composer-preview\">"
            ),
            "{out}"
        );
        assert!(out.contains("<span>Draft</span>"), "{out}");
        assert!(
            out.contains(
                "<button type=\"button\" aria-expanded=\"true\" aria-label=\"Collapse preview\" \
                 class=\"rc-preview-toggle\">\u{25BE}</button>"
            ),
            "{out}"
        );
        assert!(out.contains("<math"), "the draft renders as one: {out}");
        assert!(!out.contains("rc-collapsed"), "{out}");
        assert!(
            out.contains("aria-label=\"Send\" class=\"rc-send\">"),
            "send is enabled: {out}"
        );

        // No kind named leaves the attribute out; a blank draft has no
        // preview and nothing to send.
        let plain = html(|| view! { <Composer on_send=|_text: String| {} draft=draft /> });
        assert!(
            plain.contains("<div aria-live=\"polite\" class=\"rc-composer-preview\">"),
            "{plain}"
        );
        draft.set(String::from("  \n"));
        let blank = html(|| view! { <Composer on_send=|_text: String| {} draft=draft /> });
        assert!(!blank.contains("rc-composer-preview"), "{blank}");
        assert!(
            blank.contains("aria-label=\"Send\" disabled class=\"rc-send\">"),
            "{blank}"
        );

        // Switched off, the preview stays out whatever the draft.
        draft.set(String::from("text"));
        let off =
            html(|| view! { <Composer on_send=|_text: String| {} draft=draft preview=false /> });
        assert!(!off.contains("rc-composer-preview"), "{off}");
    }

    #[test]
    fn chat_hands_its_draft_to_the_composer() {
        let draft = RwSignal::new(String::from("**bold**"));
        let none = Vec::<Message>::new();
        let out = html(|| view! { <Chat messages=none on_send=|_: String| {} draft=draft /> });
        assert!(out.contains("rc-composer-preview"), "{out}");
        assert!(out.contains("<strong>bold</strong>"), "{out}");
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
            Message::new("1", "me", "Hello"),
            Message::new("2", "them", "Hi *there*"),
        ]);
        let full = html(
            || view! { <Chat messages=messages on_send=|_: String| {} empty="Nothing yet" /> },
        );
        assert!(!full.contains("Nothing yet"), "{full}");
        assert!(full.contains("<p>Hi <em>there</em></p>"), "{full}");
        let me = full.find("data-kind=\"me\"").unwrap();
        let them = full.find("data-kind=\"them\"").unwrap();
        assert!(me < them, "messages keep their order");
    }

    #[test]
    fn style_component_writes_the_kinds_rules_separately() {
        let out = html(|| view! { <RichChatStyle /> });
        assert_eq!(out.matches("<style>").count(), 2, "{}", &out[..80]);
        assert!(
            out.contains("[data-kind=\"user\"]"),
            "default kinds missing"
        );
        assert!(
            out.contains("[data-kind=\"assistant\"]"),
            "default kinds missing"
        );
        let kinds = Kinds::none().kind("notice", Look::at(Position::Center));
        let out = html(|| view! { <RichChatStyle kinds=kinds /> });
        assert!(
            !out.contains("[data-kind=\"user\"]"),
            "default kinds still there"
        );
        assert!(
            out.contains(
                "[data-kind=\"notice\"] > .rc-bubble { margin-left: auto; margin-right: auto; }"
            ),
            "{out}"
        );
        let out = html(|| view! { <RichChatStyle kinds=Kinds::none() /> });
        assert!(!out.contains("[data-kind="), "no kinds, no rules");
    }

    #[test]
    fn the_preview_kind_takes_a_literal_or_a_signal() {
        let none = Vec::<Message>::new();
        let literal =
            html(|| view! { <Chat messages=none on_send=|_: String| {} preview_kind="me" /> });
        let none = Vec::<Message>::new();
        let sender = RwSignal::new(String::from("me"));
        let signal =
            html(|| view! { <Chat messages=none on_send=|_: String| {} preview_kind=sender /> });
        assert!(literal.contains("rc-composer"), "{literal}");
        assert!(signal.contains("rc-composer"), "{signal}");
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

    #[test]
    fn a_draft_is_sent_unless_blank_or_disabled() {
        Owner::new().with(|| {
            let sent = RwSignal::new(Vec::<String>::new());
            let on_send = Callback::new(move |text| sent.update(|all| all.push(text)));
            let draft = RwSignal::new(String::from("hi"));
            send_draft(true, draft, on_send);
            assert!(sent.get_untracked().is_empty(), "nothing while disabled");
            assert_eq!(draft.get_untracked(), "hi", "and the draft is kept");
            send_draft(false, draft, on_send);
            assert_eq!(sent.get_untracked(), ["hi"]);
            assert_eq!(draft.get_untracked(), "", "sent, the box is empty");
            draft.set(String::from("  hi \n"));
            send_draft(false, draft, on_send);
            assert_eq!(sent.get_untracked()[1], "  hi \n", "sent as written");
            for blank in ["", " \n\t"] {
                draft.set(blank.to_string());
                send_draft(false, draft, on_send);
                assert_eq!(sent.get_untracked().len(), 2, "a blank draft is not sent");
                assert_eq!(draft.get_untracked(), blank, "and is left as it is");
            }
        });
    }

    #[test]
    fn enter_sends_but_not_with_shift_or_mid_composition() {
        assert!(enter_sends("Enter", false, false));
        assert!(!enter_sends("Enter", true, false));
        assert!(!enter_sends("Enter", false, true));
        assert!(!enter_sends("a", false, false));
        assert!(!enter_sends("NumpadEnter", false, false));
    }

    #[test]
    fn follow_does_nothing_without_a_pane() {
        let _ = any_spawner::Executor::init_futures_executor();
        Owner::new().with(|| {
            let follow = Follow::new(NodeRef::new());
            follow.scrolled();
            follow.to_end();
            assert!(follow.at_end.get_value());
            assert_eq!(follow.last_top.get_value(), 0);
        });
    }
}
