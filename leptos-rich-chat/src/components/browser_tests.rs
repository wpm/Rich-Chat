//! The components in a real browser, for what only a DOM can show: clicks
//! and keys, the focus, the clipboard, and the transcript following its
//! end. Run with `.github/scripts/browser-tests.sh`; the tests in
//! `super::tests` cover the rest natively.

use std::sync::{Arc, Mutex};

use leptos::mount::{UnmountHandle, mount_to};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{
    Event, EventInit, HtmlElement, HtmlTextAreaElement, KeyboardEvent, KeyboardEventInit,
    MouseEvent, MouseEventInit,
};

use super::*;

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

/// Mounts `view` in a fresh element of the page's body, returning the
/// element and the handle that keeps the view mounted.
fn mount<V: IntoView + 'static>(
    view: impl FnOnce() -> V + 'static,
) -> (HtmlElement, UnmountHandle<V::State>) {
    let host: HtmlElement = document().create_element("div").unwrap().unchecked_into();
    document().body().unwrap().append_child(&host).unwrap();
    let handle = mount_to(host.clone(), view);
    (host, handle)
}

/// The first element `selector` finds under `root`.
fn find(root: &HtmlElement, selector: &str) -> HtmlElement {
    root.query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("nothing matches {selector}"))
        .unchecked_into()
}

fn exists(root: &HtmlElement, selector: &str) -> bool {
    root.query_selector(selector).unwrap().is_some()
}

/// Clicks `target` as a reader would, and says whether the browser was
/// left to follow it: false when a handler prevented the default.
fn click(target: &HtmlElement) -> bool {
    let init = MouseEventInit::new();
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = MouseEvent::new_with_mouse_event_init_dict("click", &init).unwrap();
    target.dispatch_event(&event).unwrap()
}

/// A key pressed in `target`, with Shift and Alt as given; false when a
/// handler consumed it.
fn press(target: &HtmlElement, key: &str, code: &str, shift: bool, alt: bool) -> bool {
    let init = KeyboardEventInit::new();
    init.set_key(key);
    init.set_code(code);
    init.set_shift_key(shift);
    init.set_alt_key(alt);
    init.set_bubbles(true);
    init.set_cancelable(true);
    let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    target.dispatch_event(&event).unwrap()
}

/// Types `text` into the text box: its value set, then the input event
/// the browser fires.
fn type_into(input: &HtmlTextAreaElement, text: &str) {
    input.set_value(text);
    let init = EventInit::new();
    init.set_bubbles(true);
    let event = Event::new_with_event_init_dict("input", &init).unwrap();
    input.dispatch_event(&event).unwrap();
}

/// What has the focus.
fn active() -> Option<web_sys::Element> {
    document().active_element()
}

/// Resolves after `ms` milliseconds: long enough, at 0, for Leptos's
/// effects to have run, which it schedules as tasks.
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

/// Resolves on the next animation frame, after the effects that run
/// before it.
async fn frame() {
    sleep(0).await;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .request_animation_frame(&resolve)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    sleep(0).await;
}

/// The callback a host passes, and what it was called with.
fn recorder() -> (Callback<String>, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&seen);
    let callback = Callback::new(move |text: String| into.lock().unwrap().push(text));
    (callback, seen)
}

#[wasm_bindgen_test]
fn a_link_goes_to_the_host_instead_of_being_followed() {
    let (on_link, clicked) = recorder();
    let (root, _view) = mount(move || {
        view! { <RichText content="[a link](https://example.com)" on_link=on_link /> }
    });
    assert!(!click(&find(&root, "a")), "the browser does not follow it");
    assert_eq!(*clicked.lock().unwrap(), ["https://example.com"]);
}

#[wasm_bindgen_test]
fn an_in_page_link_is_left_to_the_browser() {
    let (on_link, clicked) = recorder();
    let (root, _view) = mount(move || {
        view! { <RichText content="A note.[^1]\n\n[^1]: The note." on_link=on_link /> }
    });
    assert!(
        click(&find(&root, "a[href^='#']")),
        "the browser follows it"
    );
    assert!(clicked.lock().unwrap().is_empty(), "the host is not told");
}

#[wasm_bindgen_test]
async fn the_copy_button_says_copied_for_a_moment() {
    let (root, _view) = mount(|| view! { <CodeBlock source="let x = 1;" html="let x = 1;" /> });
    let button = find(&root, ".rc-copy");
    assert_eq!(button.text_content().unwrap(), "Copy");
    click(&button);
    sleep(0).await;
    assert_eq!(button.text_content().unwrap(), "Copied");
    assert!(button.class_list().contains("rc-copied"));
    sleep(1600).await;
    assert_eq!(button.text_content().unwrap(), "Copy", "and then not");
}

/// A mounted composer: the element it is in, the text box, the draft,
/// and what was sent. Mounted while this lives.
struct Composed<M: Mountable> {
    root: HtmlElement,
    _view: UnmountHandle<M>,
    input: HtmlTextAreaElement,
    draft: RwSignal<String>,
    sent: Arc<Mutex<Vec<String>>>,
}

/// A composer holding `text`, on or off as `disabled` says.
fn composer(text: &str, disabled: RwSignal<bool>) -> Composed<impl Mountable> {
    let draft = RwSignal::new(text.to_string());
    let (on_send, sent) = recorder();
    let (root, view) = mount(move || {
        view! { <Composer on_send=on_send draft=draft disabled=disabled /> }
    });
    let input = find(&root, "textarea").unchecked_into();
    Composed {
        root,
        _view: view,
        input,
        draft,
        sent,
    }
}

#[wasm_bindgen_test]
async fn enter_sends_and_leaves_the_caret_in_the_box() {
    let Composed {
        input,
        draft,
        sent,
        _view,
        ..
    } = composer("hello", RwSignal::new(false));
    assert!(!press(&input, "Enter", "Enter", false, false), "consumed");
    sleep(0).await;
    assert_eq!(*sent.lock().unwrap(), ["hello"]);
    assert_eq!(draft.get_untracked(), "");
    assert_eq!(active().as_deref(), Some(input.as_ref()));
}

#[wasm_bindgen_test]
fn shift_enter_does_not_send() {
    let Composed {
        input, sent, _view, ..
    } = composer("hello", RwSignal::new(false));
    assert!(
        press(&input, "Enter", "Enter", true, false),
        "left to break the line"
    );
    assert!(sent.lock().unwrap().is_empty());
}

#[wasm_bindgen_test]
async fn the_send_button_sends() {
    let Composed {
        root, sent, _view, ..
    } = composer("hello", RwSignal::new(false));
    click(&find(&root, ".rc-send"));
    sleep(0).await;
    assert_eq!(*sent.lock().unwrap(), ["hello"]);
}

#[wasm_bindgen_test]
async fn typing_previews_the_draft_and_fits_the_box_to_it() {
    let Composed {
        root,
        input,
        draft,
        _view,
        ..
    } = composer("", RwSignal::new(false));
    assert!(!exists(&root, ".rc-composer-preview"), "nothing to preview");
    type_into(&input, "*hi*\n\nand\n\nmore");
    frame().await;
    assert_eq!(draft.get_untracked(), "*hi*\n\nand\n\nmore");
    assert!(exists(&root, ".rc-composer-preview em"));
    let height = HtmlElement::style(&input)
        .get_property_value("height")
        .unwrap();
    assert!(height.ends_with("px"), "{height:?}");
}

#[wasm_bindgen_test]
async fn the_preview_collapses_and_expands() {
    let Composed { root, _view, .. } = composer("*hi*", RwSignal::new(false));
    let toggle = find(&root, ".rc-preview-toggle");
    assert_eq!(toggle.get_attribute("aria-expanded").unwrap(), "true");
    click(&toggle);
    sleep(0).await;
    assert_eq!(toggle.get_attribute("aria-expanded").unwrap(), "false");
    assert!(!exists(&root, ".rc-composer-preview em"), "the body goes");
    click(&toggle);
    sleep(0).await;
    assert!(exists(&root, ".rc-composer-preview em"), "and comes back");
}

#[wasm_bindgen_test]
async fn alt_shift_p_reads_the_preview_and_escape_returns() {
    let Composed {
        root, input, _view, ..
    } = composer("*hi*", RwSignal::new(false));
    click(&find(&root, ".rc-preview-toggle"));
    sleep(0).await;
    input.focus().unwrap();
    // macOS reports the character Option+Shift+P makes, not P.
    assert!(!press(&input, "∏", "KeyP", true, true), "consumed");
    sleep(0).await;
    let preview = find(&root, ".rc-composer-preview");
    assert_eq!(active().as_deref(), Some(preview.as_ref()));
    assert!(exists(&root, ".rc-composer-preview em"), "expanded first");
    assert!(
        !press(&preview, "Escape", "Escape", false, false),
        "consumed"
    );
    assert_eq!(active().as_deref(), Some(input.as_ref()));
}

#[wasm_bindgen_test]
async fn alt_shift_p_does_nothing_without_a_preview() {
    let Composed { input, _view, .. } = composer("", RwSignal::new(false));
    input.focus().unwrap();
    press(&input, "P", "KeyP", true, true);
    sleep(0).await;
    assert_eq!(active().as_deref(), Some(input.as_ref()));
}

#[wasm_bindgen_test]
async fn the_focus_goes_to_the_box_when_the_preview_goes_from_under_it() {
    let Composed {
        root,
        input,
        draft,
        _view,
        ..
    } = composer("*hi*", RwSignal::new(false));
    find(&root, ".rc-composer-preview").focus().unwrap();
    draft.set(String::new());
    frame().await;
    assert!(!exists(&root, ".rc-composer-preview"));
    assert_eq!(active().as_deref(), Some(input.as_ref()));
}

#[wasm_bindgen_test]
async fn the_caret_goes_back_when_the_composer_comes_back_on() {
    let disabled = RwSignal::new(false);
    let Composed { input, _view, .. } = composer("hello", disabled);
    input.focus().unwrap();
    disabled.set(true);
    frame().await;
    assert!(input.disabled());
    // The browser blurs a control as it becomes disabled; Chrome does so
    // a task later, or when asked to, so ask.
    input.blur().unwrap();
    disabled.set(false);
    frame().await;
    frame().await;
    assert_eq!(active().as_deref(), Some(input.as_ref()));
}

#[wasm_bindgen_test]
async fn a_reader_who_went_elsewhere_stays_there() {
    let disabled = RwSignal::new(false);
    let Composed {
        root, input, _view, ..
    } = composer("hello", disabled);
    input.focus().unwrap();
    disabled.set(true);
    frame().await;
    // The reader goes to something outside the composer.
    let elsewhere: HtmlElement = document()
        .create_element("button")
        .unwrap()
        .unchecked_into();
    root.append_child(&elsewhere).unwrap();
    elsewhere.focus().unwrap();
    disabled.set(false);
    frame().await;
    frame().await;
    assert_eq!(active().as_deref(), Some(elsewhere.as_ref()));
}

/// A transcript in a pane 200 pixels tall, styled, with the scrolling
/// element and the messages to add to.
fn transcript() -> (
    HtmlElement,
    UnmountHandle<impl Mountable>,
    HtmlElement,
    RwSignal<Vec<Message>>,
) {
    let messages = RwSignal::new(Vec::new());
    let (root, view) = mount(move || {
        view! {
            <RichChatStyle />
            <div style="height: 200px">
                <Transcript messages=messages warm_up=false />
            </div>
        }
    });
    let pane = find(&root, ".rc-messages");
    (root, view, pane, messages)
}

fn add(messages: RwSignal<Vec<Message>>, count: usize) {
    messages.update(|all| {
        for _ in 0..count {
            let n = all.len();
            all.push(Message::new(n.to_string(), "User", format!("Message {n}")));
        }
    });
}

fn gap(pane: &HtmlElement) -> i32 {
    pane.scroll_height() - pane.scroll_top() - pane.client_height()
}

/// Lets the observers report and the scroll events they cause arrive.
async fn settle() {
    for _ in 0..3 {
        frame().await;
    }
}

#[wasm_bindgen_test]
async fn the_transcript_follows_its_end_until_the_reader_scrolls_up() {
    let (_root, _view, pane, messages) = transcript();
    add(messages, 20);
    settle().await;
    assert!(pane.scroll_height() > pane.client_height(), "it scrolls");
    assert!(gap(&pane) <= END_SLACK, "at the end: {}", gap(&pane));

    // The reader scrolls up; new messages leave them where they are.
    pane.set_scroll_top(0);
    settle().await;
    add(messages, 5);
    settle().await;
    assert_eq!(pane.scroll_top(), 0, "left alone");

    // Back at the end, it follows again.
    pane.set_scroll_top(pane.scroll_height());
    settle().await;
    add(messages, 5);
    settle().await;
    assert!(gap(&pane) <= END_SLACK, "following again: {}", gap(&pane));
}
