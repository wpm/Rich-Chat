//! The Leptos components.
//!
//! [`Chat`] is the whole thing: a transcript above a [`Composer`]. Each
//! piece is also usable alone: [`MessageView`] for one message,
//! [`RichText`] for any Markdown, [`CodeBlock`] for one highlighted block.
//! All of them expect at least the structure stylesheet, which
//! [`RichChatStyle`] injects; see [`style`](crate::style) for what a host
//! can replace.

use std::time::Duration;

use leptos::attribute_interceptor::AttributeInterceptor;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::tachys::html::attribute::custom::custom_attribute;
use wasm_bindgen::JsCast;

use crate::message::{Body, Message};
use crate::names::Names;
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
/// The rules for the names the host's messages are from come from
/// `names`, which may be a signal: only they are rewritten when it
/// changes, in a second `<style>` element, so the fonts and the theme
/// are injected once.
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
    /// Where each name's bubbles sit and their colors. The default is
    /// [`Names::default`]; [`Names::none`] leaves every bubble plain.
    #[prop(default = Signal::stored(Names::default()), into)]
    names: Signal<Names>,
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
        <style inner_html=move || names.read().css()></style>
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
/// message's name as `data-name`, which the host's [`Names`] rules place
/// and color. With `show_names`, the name is written over the body, in a
/// `div.rc-sender` on the body's side.
///
/// What follows the name depends on the message's [`Body`]. Markdown,
/// [`Body::Text`], is rendered by [`RichText`] inside a `div.rc-bubble`,
/// which is where the theme's width, padding, radius and colors live. A
/// view of the host's, [`Body::View`], is rendered in the bubble's place,
/// as a direct child of `div.rc-message`, so a body that is deliberately
/// not speech does not start by undoing the bubble; a host that wants
/// the bubble around its own body gives its root element
/// `class="rc-bubble"`, and the name's colors and tail land on it.
/// `options` and `on_link` apply to Markdown bodies only.
#[component]
pub fn MessageView(
    /// The message.
    message: Message,
    /// Write the message's name over its body. May be a signal.
    #[prop(optional, into)]
    show_names: Signal<bool>,
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
    let name = message.name.clone();
    let body = match message.body {
        Body::Text(content) => view! {
            <div class="rc-bubble">
                <RichText content=content draft=message.live options=options on_link=on_link />
            </div>
        }
        .into_any(),
        Body::View(view) => view.run(),
    };
    view! {
        <div
            class="rc-message"
            data-name=message.name.clone()
            data-message-id=message.id.clone()
            node_ref=node_ref
        >
            <Show when=move || show_names.get()>
                <div class="rc-sender">{name.clone()}</div>
            </Show>
            {body}
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

/// Whether nothing may be sent: the composer is off, `disabled`, or the
/// host is waiting on a reply, `busy`. The two differ in everything else:
/// off, the text box is disabled and there is no preview; busy, the
/// reader goes on writing the next message and only the send waits.
fn sending_blocked(disabled: bool, busy: bool) -> bool {
    disabled || busy
}

/// Whether a press is refused by the wait and nothing else: the composer
/// is not off, it is busy, and the draft is not blank. That press is the
/// one worth a word, since the reader did everything right and only the
/// wait stood in the way; a blank draft or a composer that is off is
/// refused quietly, as ever. The same condition decides when the send
/// button is named for the wait rather than disabled.
fn busy_refusal(disabled: bool, busy: bool, blank: bool) -> bool {
    !disabled && busy && !blank
}

/// What the composer's status line says: `busy_label` while a press has
/// been refused for the wait and the wait is still on, and nothing
/// otherwise. The two are read together so that the phrase leaves with
/// the wait, whether or not the flag has been cleared yet.
fn busy_status(refused: bool, busy: bool, busy_label: &str) -> &str {
    if refused && busy { busy_label } else { "" }
}

/// What Enter and the send button do: send the draft and leave the box
/// empty, with the caret back in it for the next message; not while
/// sending is blocked (see [`sending_blocked`]), when the draft is kept,
/// and never a blank one, which stays as it is. A press that only the
/// wait refused (see [`busy_refusal`]) sets `refused`, which is what
/// fills the status line. The signals are read when the key or the
/// button is pressed, not when the composer is built, and untracked,
/// since an event handler subscribes to nothing.
fn send_draft(
    disabled: Signal<bool>,
    busy: Signal<bool>,
    draft: RwSignal<String>,
    refused: RwSignal<bool>,
    on_send: Callback<String>,
    input: NodeRef<html::Textarea>,
) {
    let (disabled, busy) = (disabled.get_untracked(), busy.get_untracked());
    let blank = draft.read_untracked().trim().is_empty();
    if busy_refusal(disabled, busy, blank) {
        refused.set(true);
    }
    if sending_blocked(disabled, busy) || blank {
        return;
    }
    // Taken, not copied: the signal is left empty by the same write.
    on_send.run(std::mem::take(&mut *draft.write()));
    // The send came from the box, by Enter in it or the button beside
    // it, and a click on the button took the focus with it.
    focus(input);
}

/// Puts the caret in the text box.
#[cfg(target_arch = "wasm32")]
fn focus(input: NodeRef<html::Textarea>) {
    if let Some(element) = input.get_untracked() {
        let _ = web_sys::HtmlElement::focus(&element);
    }
}

/// Off the browser there is no box to put the caret in.
#[cfg(not(target_arch = "wasm32"))]
fn focus(_input: NodeRef<html::Textarea>) {}

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
/// Sending clears it either way, and puts the caret back in the box,
/// ready for the next message.
///
/// Two states keep a message from being sent, and they are not each
/// other. `disabled` is the composer off, for a host with nothing to send
/// with: the text box is disabled, there is no preview, and nothing can
/// be sent. `busy` is a wait, for a host whose last message is still
/// being answered: the reader goes on writing the next one, preview and
/// all, and only the sending waits, Enter and the button both. Enter
/// still does not break a line while busy; Shift+Enter does. Neither
/// state touches the draft, and nothing is sent on its own when either
/// ends: the draft waits for Enter or the button. `.rc-composer` has the
/// class `rc-busy` for the length of the wait, which the theme leaves
/// alone; it is there for a host that wants the wait to show, and it is
/// not announced. With both set the composer is off, and the class,
/// which follows `busy` alone, is still there.
///
/// The wait is spoken as well as shown, in the host's words,
/// `busy_label`. While the wait alone holds a draft back, the send button
/// is named `Send, {busy_label}` and carries `aria-disabled="true"` in
/// place of `disabled`, so it stays in the tab order and going to it says
/// why it cannot be pressed; a composer that is off, or a blank draft,
/// still disables it under the plain name. And a press that only the
/// wait refused, Enter or the button, puts `busy_label` in
/// `div.rc-composer-status`, a `role="status"` live region that is the
/// first child of `.rc-composer` and is hidden from sight by the
/// structure stylesheet; the phrase stays until the wait ends, repeated
/// presses within one wait announce it once, and the wait beginning
/// announces nothing.
///
/// Attributes passed to the component go on the text box, not on the
/// wrapper: `attr:id` for a label elsewhere on the page to point at,
/// `attr:maxlength`, `attr:data-*`, `class:mine=true` to add a class, and
/// so on. The text box has `spellcheck="true"`,
/// `autocapitalize="sentences"` and `autocorrect="on"` unless the host
/// passes its own, which win. (`attr:class` replaces the box's class
/// attribute, as it does on any element; `class:` adds to it.)
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
    /// The composer is off while true: the text box is disabled, nothing
    /// is previewed, and nothing can be sent. The draft signal is kept.
    #[prop(optional, into)]
    disabled: Signal<bool>,
    /// A reply is in flight while true: the text box stays open and its
    /// draft is previewed as ever, but neither Enter nor the button
    /// sends. When it falls back to false nothing is sent on its own.
    #[prop(optional, into)]
    busy: Signal<bool>,
    /// What a screen reader is told while `busy` holds a send back: the
    /// send button's name carries it, and a refused press puts it in the
    /// status line. A host talking to a model might say "Waiting for a
    /// reply".
    #[prop(default = "Sending is paused".to_string(), into)]
    busy_label: String,
    /// Show the live preview.
    #[prop(default = true)]
    preview: bool,
    /// The heading over the preview.
    #[prop(default = "Preview".to_string(), into)]
    preview_label: String,
    /// Whose message the preview shows, so that it takes that name's
    /// colors from the [`Names`] rules. Empty, the default, leaves it in
    /// the theme's tint colors. May be a signal, for a host whose sender
    /// changes.
    #[prop(optional, into)]
    preview_name: Signal<String>,
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
    // Held, not captured: `submit` is built once and used by both the
    // keydown handler and the click handler, so it has to be `Copy`.
    let busy_label = StoredValue::new(busy_label);
    // A press the wait alone refused, which is what the status line says.
    let refused = RwSignal::new(false);
    let submit = move || send_draft(disabled, busy, draft, refused, on_send, input);
    // The flag lasts one wait: cleared when the wait ends, so that the
    // next wait begins quiet and only a press during it speaks. Not run
    // in a server render, where the status line is empty anyway.
    Effect::new(move |_| {
        if !busy.get() {
            refused.set(false);
        }
    });
    // The height follows the text however it changes: sent, or put in
    // by the host, at the start or later. Browser only, as the frame is.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        draft.track();
        request_animation_frame(fit);
    });
    let has_draft = move || !draft.read().trim().is_empty();
    // Whether the wait alone holds the send back: then the button is
    // named for it and kept in the tab order rather than disabled.
    let held = move || busy_refusal(disabled.get(), busy.get(), !has_draft());
    let status = move || {
        busy_label.with_value(|label| busy_status(refused.get(), busy.get(), label).to_string())
    };
    let send = send_content(send);
    let hint = hint_text(hint);
    let preview_name = move || {
        let name = preview_name.get();
        (!name.is_empty()).then_some(name)
    };
    let expanded = RwSignal::new(true);
    let options = StoredValue::new(options);
    // Stored, since the view is built once per set of attributes the host
    // passes and the preview's children are rebuilt every time it shows.
    let preview_label = StoredValue::new(preview_label);

    // The attributes passed to the component go on the text box, not on
    // the wrapper Leptos would put them on: `id`, `maxlength`, `data-*`
    // and the rest are the input's to take. The three the text box has
    // by default are what prose in a chat wants; they come before the
    // host's, so in the browser, where the last value set wins, a host
    // that passes one of them has its way.
    view! {
        <AttributeInterceptor let:attrs>
            <div class="rc-composer" class:rc-busy=move || busy.get()>
                // In the tree from the start, empty: a live region is
                // announced only for what is put into it after it is
                // there. Hidden from sight by the structure stylesheet.
                <div class="rc-composer-status" role="status">{status}</div>
                // Off, the composer previews nothing, even a draft the
                // host put in the box; busy, it previews as ever.
                <Show when=move || preview && !disabled.get() && has_draft()>
                    <div
                        class="rc-composer-preview"
                        class:rc-collapsed=move || !expanded.get()
                        data-name=preview_name
                        aria-live="polite"
                    >
                        <div class="rc-composer-preview-label">
                            <span>{preview_label.get_value()}</span>
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
                            <RichText
                                content=draft
                                draft=true
                                options=options.get_value()
                                on_link=on_link
                            />
                        </Show>
                    </div>
                </Show>
                <div class="rc-composer-row">
                    <textarea
                        node_ref=input
                        class="rc-composer-input"
                        rows="1"
                        placeholder=placeholder.clone()
                        aria-label="Message"
                        spellcheck="true"
                        autocapitalize="sentences"
                        {..custom_attribute("autocorrect", "on")}
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
                        {..attrs}
                    ></textarea>
                    // Held by the wait alone, the button says so and stays
                    // in the tab order; the press still goes to `submit`,
                    // which refuses it. Off, or with nothing to send, it is
                    // disabled as any button is.
                    <button
                        type="button"
                        class="rc-send"
                        aria-label=move || {
                            if held() {
                                busy_label.with_value(|label| format!("Send, {label}"))
                            } else {
                                String::from("Send")
                            }
                        }
                        aria-disabled=move || held().then_some("true")
                        disabled=move || disabled.get() || !has_draft()
                        on:click=move |_| submit()
                    >
                        {send.run()}
                    </button>
                </div>
                {(!hint.is_empty()).then(|| view! { <div class="rc-composer-hint">{hint.clone()}</div> })}
            </div>
        </AttributeInterceptor>
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
    /// The composer is off while true: the text box is disabled, nothing
    /// is previewed, and nothing can be sent. The draft signal is kept.
    /// See [`Composer`].
    #[prop(optional, into)]
    disabled: Signal<bool>,
    /// A reply is in flight while true: the text box stays open and its
    /// draft is previewed as ever, but neither Enter nor the button
    /// sends; the send button and the composer's status line say so in
    /// the words of `busy_label`. See [`Composer`].
    #[prop(optional, into)]
    busy: Signal<bool>,
    /// What a screen reader is told while `busy` holds a send back. See
    /// [`Composer`].
    #[prop(default = "Sending is paused".to_string(), into)]
    busy_label: String,
    /// Show the live preview in the composer.
    #[prop(default = true)]
    preview: bool,
    /// The heading over the preview.
    #[prop(default = "Preview".to_string(), into)]
    preview_label: String,
    /// Whose message the preview shows, for its colors. See [`Composer`].
    #[prop(optional, into)]
    preview_name: Signal<String>,
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
    /// Write each message's name over its bubble. Off by default; may be
    /// a signal, for a host that lets the reader switch it.
    #[prop(optional, into)]
    show_names: Signal<bool>,
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

    let body_options = options.clone();
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
                            <MessageView
                                message=message
                                show_names=show_names
                                options=body_options.clone()
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
                busy=busy
                busy_label=busy_label
                preview=preview
                preview_label=preview_label
                preview_name=preview_name
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
    use crate::names::{Look, Position};

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
    fn bubbles_carry_name_and_id() {
        for name in ["user", "alice", "a name \"quoted\""] {
            let message = Message::new("m7", name, "hi");
            let out = html(|| view! { <MessageView message=message /> });
            assert!(out.starts_with("<div "), "{out}");
            assert!(out.contains(" class=\"rc-message\">"), "{out}");
            let escaped = name.replace('"', "&quot;");
            assert!(out.contains(&format!("data-name=\"{escaped}\"")), "{out}");
            assert!(out.contains("data-message-id=\"m7\""), "{out}");
            assert!(out.contains("<div class=\"rc-bubble\">"), "{out}");
            assert!(out.contains("<p>hi</p>"), "{out}");
            assert!(!out.contains("rc-sender"), "no name unless asked: {out}");
        }
    }

    #[test]
    fn a_bubble_shows_its_name_when_asked() {
        let message = Message::new("m8", "Alice <3", "hi");
        let out = html(|| view! { <MessageView message=message show_names=true /> });
        let name = out
            .find("<div class=\"rc-sender\">Alice &lt;3</div>")
            .unwrap_or_else(|| panic!("{out}"));
        let bubble = out
            .find("<div class=\"rc-bubble\">")
            .unwrap_or_else(|| panic!("{out}"));
        assert!(name < bubble, "the name is over the bubble: {out}");

        // A signal, so that the reader can switch it.
        let show = RwSignal::new(false);
        let message = Message::new("m8", "Alice", "hi");
        let off = html(|| view! { <MessageView message=message.clone() show_names=show /> });
        assert!(!off.contains("rc-sender"), "{off}");
        show.set(true);
        let on = html(|| view! { <MessageView message=message show_names=show /> });
        assert!(on.contains("<div class=\"rc-sender\">Alice</div>"), "{on}");
    }

    #[test]
    fn a_live_bubble_renders_as_a_draft() {
        let text = RwSignal::new(String::from("so $x^2"));
        let live = html(|| view! { <MessageView message=Message::streaming("s", "bob", text) /> });
        assert!(live.contains("<math"), "{live}");
        let done = html(
            || view! { <MessageView message=Message::streaming("s", "bob", text).finished() /> },
        );
        assert!(!done.contains("<math"), "{done}");
    }

    #[test]
    fn a_view_body_takes_the_bubbles_place() {
        let message = Message::view("t1", "tool", || {
            view! { <details class="tool-call"><summary>"ran a tool"</summary>"output"</details> }
        });
        let out = html(|| view! { <MessageView message=message show_names=true /> });
        assert!(out.starts_with("<div "), "{out}");
        assert!(out.contains(" class=\"rc-message\">"), "{out}");
        assert!(out.contains("data-name=\"tool\""), "{out}");
        assert!(out.contains("data-message-id=\"t1\""), "{out}");
        assert!(
            !out.contains("rc-bubble"),
            "no bubble unless the host asks for one: {out}"
        );
        assert!(
            !out.contains("rc-rich"),
            "nothing is rendered as Markdown: {out}"
        );
        let name = out
            .find("<div class=\"rc-sender\">tool</div>")
            .unwrap_or_else(|| panic!("{out}"));
        let body = out
            .find("<details class=\"tool-call\">")
            .unwrap_or_else(|| panic!("{out}"));
        assert!(name < body, "the name is over the body: {out}");
        assert!(
            out.ends_with("</details></div>"),
            "the body is the message's last child: {out}"
        );
    }

    #[test]
    fn a_view_body_may_opt_into_the_bubble() {
        let message = Message::view("t2", "tool", || {
            view! { <div class="rc-bubble">"looks like speech"</div> }
        });
        let out = html(|| view! { <MessageView message=message /> });
        assert!(
            out.contains("<div class=\"rc-bubble\">looks like speech</div>"),
            "{out}"
        );
        assert_eq!(
            out.matches("rc-bubble").count(),
            1,
            "one bubble, the host's: {out}"
        );
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
    fn composer_previews_the_hosts_draft_with_the_preview_name() {
        let draft = RwSignal::new(String::from("so $x^2"));
        let out = html(|| {
            view! {
                <Composer on_send=|_text: String| {} draft=draft preview_name="me" preview_label="Draft" />
            }
        });
        assert!(
            out.contains(
                "<div data-name=\"me\" aria-live=\"polite\" class=\"rc-composer-preview\">"
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

        // No name given leaves the attribute out; a blank draft has no
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

    /// The opening tag of the first element of that name, attributes and
    /// all.
    fn opening_tag<'a>(out: &'a str, element: &str) -> &'a str {
        let start = out
            .find(&format!("<{element}"))
            .unwrap_or_else(|| panic!("no {element}: {out}"));
        let end = out[start..].find('>').expect("an unclosed tag") + start;
        &out[start..=end]
    }

    /// The text of the status line, `div.rc-composer-status`. A server
    /// render puts one space where a dynamic text is empty, the
    /// placeholder that hydration finds the text node by.
    fn status_line(out: &str) -> &str {
        let open = "<div role=\"status\" class=\"rc-composer-status\">";
        let start = out
            .find(open)
            .unwrap_or_else(|| panic!("no status line: {out}"))
            + open.len();
        let end = out[start..]
            .find("</div>")
            .expect("an unclosed status line")
            + start;
        &out[start..end]
    }

    /// The opening tag of the send button.
    fn send_button(out: &str) -> &str {
        let class = out.find("class=\"rc-send\"").expect(out);
        opening_tag(&out[out[..class].rfind("<button").expect(out)..], "button")
    }

    #[test]
    fn a_busy_composer_keeps_the_box_and_the_preview_but_not_the_send() {
        let draft = RwSignal::new(String::from("so $x^2"));
        let out = html(|| view! { <Composer on_send=|_text: String| {} draft=draft busy=true /> });
        assert!(
            out.starts_with("<div class=\"rc-composer rc-busy\">"),
            "the wait is there to style: {out}"
        );
        let text_box = opening_tag(&out, "textarea");
        assert!(
            !text_box.contains("disabled"),
            "the box stays open: {text_box}"
        );
        assert!(out.contains("class=\"rc-composer-preview\""), "{out}");
        assert!(out.contains("class=\"rc-preview-toggle\""), "{out}");
        assert!(out.contains("<math"), "the draft renders as ever: {out}");
        // The button is held rather than disabled: it stays in the tab
        // order and its name says why it does nothing.
        let button = send_button(&out);
        assert!(
            button.contains("aria-disabled=\"true\""),
            "held, not off: {button}"
        );
        assert!(
            button.contains("aria-label=\"Send, Sending is paused\""),
            "named for the wait: {button}"
        );
        assert!(!button.contains(" disabled"), "in the tab order: {button}");
        assert!(
            status_line(&out).trim().is_empty(),
            "the status line is there and quiet: {out}"
        );

        // Not busy, the class is gone and the button is back, under its
        // plain name.
        let idle =
            html(|| view! { <Composer on_send=|_text: String| {} draft=draft busy=false /> });
        assert!(idle.starts_with("<div class=\"rc-composer\">"), "{idle}");
        assert!(!idle.contains("rc-busy"), "{idle}");
        let button = send_button(&idle);
        assert!(!button.contains("disabled"), "{button}");
        assert!(button.contains("aria-label=\"Send\""), "{button}");

        // The host's words reach the name.
        let named = html(|| {
            view! {
                <Composer on_send=|_text: String| {} draft=draft busy=true busy_label="Waiting for a reply" />
            }
        });
        assert!(
            send_button(&named).contains("aria-label=\"Send, Waiting for a reply\""),
            "{named}"
        );

        // A blank draft is disabled under the plain name, busy or not:
        // there is nothing the wait is holding back.
        draft.set(String::new());
        let blank =
            html(|| view! { <Composer on_send=|_text: String| {} draft=draft busy=true /> });
        let button = send_button(&blank);
        assert!(button.contains(" disabled"), "{button}");
        assert!(!button.contains("aria-disabled"), "{button}");
        assert!(button.contains("aria-label=\"Send\""), "{button}");
    }

    #[test]
    fn every_composer_has_an_empty_status_line() {
        // In the tree before it is filled, since a live region announces
        // only what arrives after it is there; and empty in a server
        // render, where no press has been refused.
        let bare = html(|| view! { <Composer on_send=|_text: String| {} /> });
        assert!(
            bare.starts_with(
                "<div class=\"rc-composer\"><div role=\"status\" class=\"rc-composer-status\">"
            ),
            "first child: {bare}"
        );
        assert!(status_line(&bare).trim().is_empty(), "{bare}");
        let draft = RwSignal::new(String::from("typed"));
        for (busy, disabled) in [(false, false), (true, false), (false, true), (true, true)] {
            let out = html(|| {
                view! { <Composer on_send=|_text: String| {} draft=draft busy=busy disabled=disabled /> }
            });
            assert!(
                status_line(&out).trim().is_empty(),
                "busy {busy}, disabled {disabled}: {out}"
            );
        }
        let none = Vec::<Message>::new();
        let chat = html(|| view! { <Chat messages=none on_send=|_: String| {} busy=true /> });
        assert!(status_line(&chat).trim().is_empty(), "{chat}");
    }

    #[test]
    fn a_disabled_composer_is_off() {
        // A draft the host holds, so that there is one to not preview.
        let draft = RwSignal::new(String::from("so $x^2"));
        let out =
            html(|| view! { <Composer on_send=|_text: String| {} draft=draft disabled=true /> });
        let text_box = opening_tag(&out, "textarea");
        assert!(text_box.contains(" disabled"), "the box is off: {text_box}");
        assert!(!out.contains("rc-composer-preview"), "no preview: {out}");
        assert!(send_button(&out).contains(" disabled"), "{out}");
        assert!(!out.contains("rc-busy"), "off is not waiting: {out}");
        assert_eq!(draft.get_untracked(), "so $x^2", "the draft is kept");

        // Off wins over waiting: the box is disabled all the same.
        let both = html(|| {
            view! { <Composer on_send=|_text: String| {} draft=draft disabled=true busy=true /> }
        });
        assert!(
            opening_tag(&both, "textarea").contains(" disabled"),
            "{both}"
        );
        assert!(!both.contains("rc-composer-preview"), "{both}");
        assert!(
            both.starts_with("<div class=\"rc-composer rc-busy\">"),
            "the class follows busy alone: {both}"
        );
        let button = send_button(&both);
        assert!(button.contains(" disabled"), "off wins: {button}");
        assert!(!button.contains("aria-disabled"), "{button}");
        assert!(button.contains("aria-label=\"Send\""), "{button}");
    }

    #[test]
    fn the_preview_goes_while_disabled_and_comes_back_after() {
        // Signals, as a host holds them. A render off the browser is
        // made once, so each state is rendered anew; the same composer
        // living through the change is the end-to-end check's to see.
        let draft = RwSignal::new(String::from("**typed** before"));
        let disabled = RwSignal::new(false);
        let render = || {
            html(|| view! { <Composer on_send=|_text: String| {} draft=draft disabled=disabled /> })
        };
        let on = render();
        assert!(on.contains("rc-composer-preview"), "{on}");
        assert!(!opening_tag(&on, "textarea").contains("disabled"), "{on}");

        disabled.set(true);
        let off = render();
        assert!(!off.contains("rc-composer-preview"), "{off}");
        assert!(opening_tag(&off, "textarea").contains(" disabled"), "{off}");
        assert_eq!(draft.get_untracked(), "**typed** before", "kept");

        disabled.set(false);
        let back = render();
        assert!(back.contains("<strong>typed</strong>"), "{back}");
        assert!(!send_button(&back).contains("disabled"), "{back}");
    }

    #[test]
    fn chat_hands_busy_and_disabled_to_the_composer() {
        let draft = RwSignal::new(String::from("**bold**"));
        let busy = RwSignal::new(true);
        let none = Vec::<Message>::new();
        let out =
            html(|| view! { <Chat messages=none on_send=|_: String| {} draft=draft busy=busy /> });
        assert!(out.contains("class=\"rc-composer rc-busy\""), "{out}");
        assert!(!opening_tag(&out, "textarea").contains("disabled"), "{out}");
        assert!(out.contains("<strong>bold</strong>"), "{out}");
        assert!(
            send_button(&out).contains("aria-disabled=\"true\""),
            "{out}"
        );

        // The words go down with the state.
        let none = Vec::<Message>::new();
        let named = html(|| {
            view! {
                <Chat messages=none on_send=|_: String| {} draft=draft busy=busy busy_label="Waiting for a reply" />
            }
        });
        assert!(
            send_button(&named).contains("aria-label=\"Send, Waiting for a reply\""),
            "{named}"
        );

        let none = Vec::<Message>::new();
        let off = html(
            || view! { <Chat messages=none on_send=|_: String| {} draft=draft disabled=true /> },
        );
        assert!(opening_tag(&off, "textarea").contains(" disabled"), "{off}");
        assert!(!off.contains("rc-busy"), "{off}");
    }

    #[test]
    fn composer_text_box_is_set_up_for_prose() {
        let out = html(|| view! { <Composer on_send=|_text: String| {} /> });
        assert!(
            out.contains(
                "<textarea rows=\"1\" placeholder=\"Write a message…\" aria-label=\"Message\" \
                 spellcheck=\"true\" autocapitalize=\"sentences\" autocorrect=\"on\" \
                 class=\"rc-composer-input\">"
            ),
            "{out}"
        );
    }

    #[test]
    fn attributes_on_composer_land_on_its_text_box() {
        let out = html(|| {
            view! {
                <Composer
                    on_send=|_text: String| {}
                    attr:id="prompt"
                    attr:maxlength="500"
                    attr:data-test="box"
                    class:mine=true
                />
            }
        });
        assert!(
            out.starts_with("<div class=\"rc-composer\">"),
            "the wrapper is untouched: {out}"
        );
        assert!(
            out.contains(
                "autocorrect=\"on\" id=\"prompt\" maxlength=\"500\" data-test=\"box\" \
                 class=\"rc-composer-input mine\">"
            ),
            "{out}"
        );
        assert_eq!(out.matches("id=\"prompt\"").count(), 1, "{out}");
    }

    #[test]
    fn attributes_on_composer_come_after_its_defaults() {
        // The host's value follows the default, which in the browser is
        // the one that wins: an attribute set twice keeps the last.
        let out = html(|| {
            view! { <Composer on_send=|_text: String| {} attr:spellcheck="false" /> }
        });
        let default = out.find("spellcheck=\"true\"").expect(&out);
        let host = out.find("spellcheck=\"false\"").expect(&out);
        assert!(default < host, "{out}");
    }

    #[test]
    fn attributes_on_chat_land_on_the_window() {
        let none = Vec::<Message>::new();
        let out = html(|| {
            view! { <Chat messages=none on_send=|_: String| {} attr:id="chat" attr:data-test="window" /> }
        });
        assert!(
            out.starts_with("<div id=\"chat\" data-test=\"window\" class=\"rc-chat\">"),
            "{out}"
        );
        assert!(
            out.contains("autocorrect=\"on\" class=\"rc-composer-input\">"),
            "the text box has none of it: {out}"
        );
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
        let me = full.find("data-name=\"me\"").unwrap();
        let them = full.find("data-name=\"them\"").unwrap();
        assert!(me < them, "messages keep their order");
        assert!(
            !full.contains("rc-sender"),
            "names are off by default: {full}"
        );

        let named = html(|| {
            view! { <Chat messages=messages on_send=|_: String| {} show_names=true /> }
        });
        assert!(
            named.contains("<div class=\"rc-sender\">me</div>"),
            "{named}"
        );
        assert!(
            named.contains("<div class=\"rc-sender\">them</div>"),
            "{named}"
        );
    }

    #[test]
    fn chat_places_a_view_message_among_the_text_ones() {
        let messages = vec![
            Message::new("1", "me", "Run it"),
            Message::view(
                "2",
                "tool",
                || view! { <pre class="tool-output">"ok"</pre> },
            ),
            Message::new("3", "them", "Done"),
        ];
        let out = html(|| view! { <Chat messages=messages on_send=|_: String| {} /> });
        let first = out.find("data-message-id=\"1\"").unwrap();
        let tool = out.find("data-message-id=\"2\"").unwrap();
        let last = out.find("data-message-id=\"3\"").unwrap();
        assert!(
            first < tool && tool < last,
            "messages keep their order: {out}"
        );
        // From the tool message's opening tag to the end of its body: the
        // host's element, and no bubble around it.
        let open = out[..tool].rfind("<div ").unwrap();
        let body = &out[open..out[tool..].find("</pre>").unwrap() + tool];
        assert!(body.contains("data-name=\"tool\""), "{body}");
        assert!(body.contains("<pre class=\"tool-output\">ok"), "{body}");
        assert!(
            !body.contains("rc-bubble"),
            "no bubble around the view: {body}"
        );
        assert_eq!(
            out.matches("rc-bubble").count(),
            2,
            "one bubble per text message: {out}"
        );
    }

    #[test]
    fn style_component_writes_the_names_rules_separately() {
        let out = html(|| view! { <RichChatStyle /> });
        assert_eq!(out.matches("<style>").count(), 2, "{}", &out[..80]);
        assert!(
            out.contains("[data-name=\"user\"]"),
            "default names missing"
        );
        assert!(
            out.contains("[data-name=\"assistant\"]"),
            "default names missing"
        );
        let names = Names::none().name("notice", Look::at(Position::Center));
        let out = html(|| view! { <RichChatStyle names=names /> });
        assert!(
            !out.contains("[data-name=\"user\"]"),
            "default names still there"
        );
        assert!(
            out.contains(".rc-message[data-name=\"notice\"] { align-items: center; }"),
            "{out}"
        );
        let out = html(|| view! { <RichChatStyle names=Names::none() /> });
        assert!(!out.contains("[data-name="), "no names, no rules");
    }

    #[test]
    fn the_preview_name_takes_a_literal_or_a_signal() {
        let none = Vec::<Message>::new();
        let literal =
            html(|| view! { <Chat messages=none on_send=|_: String| {} preview_name="me" /> });
        let none = Vec::<Message>::new();
        let sender = RwSignal::new(String::from("me"));
        let signal =
            html(|| view! { <Chat messages=none on_send=|_: String| {} preview_name=sender /> });
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
        assert!(out.contains(".hl-keyword"), "highlighting missing");
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
        assert!(!out.contains(".hl-keyword"), "highlighting leaked in");
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
            let refused = RwSignal::new(false);
            let input = NodeRef::new();
            let (on, off) = (Signal::stored(true), Signal::stored(false));
            send_draft(on, off, draft, refused, on_send, input);
            assert!(sent.get_untracked().is_empty(), "nothing while disabled");
            assert_eq!(draft.get_untracked(), "hi", "and the draft is kept");
            send_draft(off, off, draft, refused, on_send, input);
            assert_eq!(sent.get_untracked(), ["hi"]);
            assert_eq!(draft.get_untracked(), "", "sent, the box is empty");
            draft.set(String::from("  hi \n"));
            send_draft(off, off, draft, refused, on_send, input);
            assert_eq!(sent.get_untracked()[1], "  hi \n", "sent as written");
            for blank in ["", " \n\t"] {
                draft.set(blank.to_string());
                send_draft(off, off, draft, refused, on_send, input);
                assert_eq!(sent.get_untracked().len(), 2, "a blank draft is not sent");
                assert_eq!(draft.get_untracked(), blank, "and is left as it is");
            }
        });
    }

    #[test]
    fn sending_is_blocked_while_off_or_busy() {
        assert!(!sending_blocked(false, false));
        assert!(sending_blocked(true, false), "off");
        assert!(sending_blocked(false, true), "waiting");
        assert!(sending_blocked(true, true));
    }

    #[test]
    fn only_the_wait_alone_refuses_a_press_out_loud() {
        // (disabled, busy, blank): the one refusal worth a word is the
        // composer on, waiting, with something to send.
        assert!(busy_refusal(false, true, false));
        assert!(!busy_refusal(false, false, false), "nothing refuses it");
        assert!(!busy_refusal(false, false, true), "blank, and no wait");
        assert!(!busy_refusal(false, true, true), "blank: nothing held back");
        assert!(!busy_refusal(true, false, false), "off, quietly");
        assert!(!busy_refusal(true, false, true), "off and blank");
        assert!(!busy_refusal(true, true, false), "off wins over the wait");
        assert!(!busy_refusal(true, true, true));
    }

    #[test]
    fn the_status_speaks_only_for_a_refusal_during_the_wait() {
        assert_eq!(busy_status(true, true, "Waiting"), "Waiting");
        assert_eq!(busy_status(true, false, "Waiting"), "", "the wait is over");
        assert_eq!(
            busy_status(false, true, "Waiting"),
            "",
            "nothing was pressed"
        );
        assert_eq!(busy_status(false, false, "Waiting"), "");
    }

    #[test]
    fn busy_blocks_the_send_and_keeps_the_draft() {
        Owner::new().with(|| {
            let sent = RwSignal::new(Vec::<String>::new());
            let on_send = Callback::new(move |text| sent.update(|all| all.push(text)));
            let draft = RwSignal::new(String::from("written during the wait"));
            let input = NodeRef::new();
            // The host's signals, as the composer holds them.
            let waiting = RwSignal::new(true);
            let off = RwSignal::new(false);
            let (busy, disabled) = (Signal::from(waiting), Signal::from(off));
            let refused = RwSignal::new(false);
            let press = || send_draft(disabled, busy, draft, refused, on_send, input);

            press();
            assert!(sent.get_untracked().is_empty(), "nothing while busy");
            assert_eq!(draft.get_untracked(), "written during the wait");

            // The reply lands. That alone sends nothing; the next press does.
            waiting.set(false);
            assert!(
                sent.get_untracked().is_empty(),
                "the wait ending is no send"
            );
            assert_eq!(draft.get_untracked(), "written during the wait");
            press();
            assert_eq!(sent.get_untracked(), ["written during the wait"]);
            assert_eq!(draft.get_untracked(), "");

            // Off blocks a press as well, and lets it through once on again.
            draft.set(String::from("next"));
            off.set(true);
            press();
            assert_eq!(sent.get_untracked().len(), 1, "nothing while disabled");
            assert_eq!(draft.get_untracked(), "next", "and the draft is kept");
            off.set(false);
            press();
            assert_eq!(sent.get_untracked()[1], "next");
        });
    }

    #[test]
    fn a_press_refused_for_the_wait_fills_the_status_line() {
        Owner::new().with(|| {
            let on_send = Callback::new(|_text: String| {});
            let draft = RwSignal::new(String::from("written during the wait"));
            let input = NodeRef::new();
            let waiting = RwSignal::new(true);
            let off = RwSignal::new(false);
            let (busy, disabled) = (Signal::from(waiting), Signal::from(off));
            let refused = RwSignal::new(false);
            let press = || send_draft(disabled, busy, draft, refused, on_send, input);
            // What the status line reads, as the composer derives it.
            let status = || busy_status(refused.get_untracked(), busy.get_untracked(), "Paused");

            assert_eq!(status(), "", "the wait beginning says nothing");
            press();
            assert_eq!(status(), "Paused", "a press during it says why");
            press();
            assert_eq!(
                status(),
                "Paused",
                "and again says it once: the text is unchanged"
            );

            // The wait ends: the phrase leaves with it, before anything
            // clears the flag (the composer's effect does that).
            waiting.set(false);
            assert_eq!(status(), "", "the wait is over");
            refused.set(false);

            // A press refused for a blank draft is quiet, busy or not.
            waiting.set(true);
            draft.set(String::from(" \n"));
            press();
            assert_eq!(status(), "", "nothing was held back");

            // And one refused because the composer is off, even while busy.
            draft.set(String::from("next"));
            off.set(true);
            press();
            assert_eq!(status(), "", "off is refused quietly");
            assert_eq!(draft.get_untracked(), "next", "and the draft is kept");
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
