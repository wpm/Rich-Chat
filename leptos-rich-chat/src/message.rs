//! The chat's data model: the messages in the transcript.

use std::fmt;

use leptos::prelude::*;

/// What a message shows: Markdown the crate renders, or a view the host
/// draws.
///
/// [`Body::Text`] is speech, and the crate renders it in a bubble. A
/// transcript that carries anything else — a tool call, a failure, a
/// prompt the reader has to answer — gives that entry a [`Body::View`]
/// and draws it itself. The crate still places the entry by its
/// [`Names`](crate::Names) rules, keys it, scrolls to it and follows it
/// while it grows; only what is inside is the host's.
///
/// A view body is rendered *in place of* the bubble, so a body that is
/// deliberately not speech does not have to undo the bubble's width,
/// padding and background. A host that wants the bubble's look around a
/// custom body puts `class="rc-bubble"` on its own root element, and the
/// name's colors and tail land on it, because the generated rules are
/// written as `.rc-message[data-name="…"] > .rc-bubble`.
#[derive(Clone)]
pub enum Body {
    /// Markdown, rendered by the crate.
    Text(Signal<String>),
    /// Drawn by the host; the crate places, keys, scrolls and follows it.
    View(ViewFn),
}

/// A [`ViewFn`] is a function, with nothing inside it to print, so the
/// view arm says only that the body is one.
impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(content) => write!(f, "Body::Text({content:?})"),
            Self::View(_) => f.write_str("Body::View(..)"),
        }
    }
}

/// One message in the transcript.
///
/// Its `name` says who it is from: a name the host chooses, such as
/// `user` or `alice`. The crate attaches no meaning to it. The message
/// carries it as `data-name`, the host's [`Names`](crate::Names) table
/// says where messages with that name sit and what colors they have, and
/// with [`Chat`](crate::Chat)'s `show_names` it is written over each
/// message.
///
/// The body is a [`Body`]: Markdown, or a view of the host's. A Markdown
/// body is a signal so that a message can grow while its text is still
/// arriving: hold an `RwSignal<String>`, append to it, and the bubble
/// re-renders only the blocks that changed. A message whose text is final
/// is built with [`Message::new`]; a growing one with
/// [`Message::streaming`], which also renders it progressively (an open
/// `$$` shows as math before its closing delimiter has arrived); one the
/// host draws with [`Message::view`].
///
/// The transcript view keys on `id` and `live`, so changing a message's
/// text means writing to its body signal, not replacing the message with
/// another of the same id.
///
/// There is no `PartialEq`: a view body is a function, which has no
/// honest equality. A host that reaches for `Memo<Vec<Message>>` wants
/// [`Signal::derive`] instead, since the keyed diff downstream is what
/// decides what is rebuilt anyway.
#[derive(Clone, Debug)]
pub struct Message {
    /// Unique within the transcript.
    pub id: String,
    /// Who the message is from, as the host names them.
    pub name: String,
    /// What was posted: Markdown, or a view of the host's.
    pub body: Body,
    /// Whether the text is still arriving. Live messages are rendered as
    /// drafts: constructs left open at the end are closed for display.
    pub live: bool,
}

impl Message {
    /// A message from `name` whose text is final.
    pub fn new(id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            body: Body::Text(Signal::stored(content.into())),
            live: false,
        }
    }

    /// A message from `name` whose text is still arriving through
    /// `content`.
    pub fn streaming(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<Signal<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            body: Body::Text(content.into()),
            live: true,
        }
    }

    /// A message from `name` whose body the host draws, in place of the
    /// bubble. Pass a closure returning a view, or a [`ViewFn`].
    ///
    /// The closure is called once per `(id, live)` key: when the
    /// transcript builds the entry, and again only when that key
    /// changes. It is `Fn` rather than `FnOnce` because [`ViewFn`] is,
    /// not because it runs often, so a `clone()` inside it is paid once
    /// per entry and not on every render. Where what it captures is more
    /// than a couple of small fields, capture an `Arc` of it, so that
    /// the clone is of a pointer rather than of the data:
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use leptos::prelude::*;
    /// use leptos_rich_chat::Message;
    ///
    /// struct Question {
    ///     ask: String,
    ///     choices: Vec<String>,
    /// }
    ///
    /// #[component]
    /// fn QuestionCard(question: Arc<Question>) -> impl IntoView {
    ///     view! {
    ///         <div class="question">
    ///             <p>{question.ask.clone()}</p>
    ///             {question.choices.iter().map(|choice| view! { <button>{choice.clone()}</button> }).collect_view()}
    ///         </div>
    ///     }
    /// }
    ///
    /// enum Entry {
    ///     Said(String),
    ///     Asked(Arc<Question>),
    /// }
    ///
    /// fn message(id: String, entry: &Entry) -> Message {
    ///     match entry {
    ///         Entry::Said(text) => Message::new(id, "user", text.clone()),
    ///         Entry::Asked(question) => {
    ///             let question = Arc::clone(question);
    ///             Message::view(id, "question", move || {
    ///                 view! { <QuestionCard question=question.clone() /> }
    ///             })
    ///         }
    ///     }
    /// }
    ///
    /// let asked = Entry::Asked(Arc::new(Question {
    ///     ask: "Run the tests?".into(),
    ///     choices: vec!["Yes".into(), "No".into()],
    /// }));
    /// let message = message("q1".into(), &asked);
    /// assert_eq!(message.name, "question");
    /// ```
    pub fn view(id: impl Into<String>, name: impl Into<String>, view: impl Into<ViewFn>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            body: Body::View(view.into()),
            live: false,
        }
    }

    /// The same message, marked final. The view rebuilds the bubble once
    /// on this transition.
    pub fn finished(mut self) -> Self {
        self.live = false;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Markdown a text body holds, for a test that expects one.
    fn text_of(body: &Body) -> String {
        match body {
            Body::Text(content) => content.get_untracked(),
            Body::View(_) => panic!("the body is a view, not text"),
        }
    }

    #[test]
    fn a_new_message_is_final_and_holds_its_text() {
        let message = Message::new("m1", "alice", "hello");
        assert_eq!(message.id, "m1");
        assert_eq!(message.name, "alice");
        assert!(!message.live);
        assert_eq!(text_of(&message.body), "hello");
    }

    #[test]
    fn a_streaming_message_follows_its_signal() {
        let text = RwSignal::new(String::from("par"));
        let message = Message::streaming(String::from("r1"), "bob", text);
        assert!(message.live);
        assert_eq!(text_of(&message.body), "par");
        text.update(|t| t.push_str("tial"));
        assert_eq!(text_of(&message.body), "partial");
    }

    #[test]
    fn finishing_keeps_everything_but_the_live_flag() {
        let text = RwSignal::new(String::from("done"));
        let live = Message::streaming("r1", "bob", text);
        let finished = live.clone().finished();
        assert!(live.live);
        assert!(!finished.live);
        assert_eq!(finished.id, live.id);
        assert_eq!(finished.name, live.name);
        assert_eq!(text_of(&finished.body), text_of(&live.body));
    }

    #[test]
    fn a_view_message_carries_the_hosts_view() {
        let message = Message::view("t1", "tool", || "drawn by the host");
        assert_eq!(message.id, "t1");
        assert_eq!(message.name, "tool");
        assert!(!message.live);
        assert!(matches!(message.body, Body::View(_)));
    }

    #[test]
    fn debug_says_which_body_a_message_has() {
        let said = Message::new("m1", "alice", "hello");
        let printed = format!("{said:?}");
        assert!(printed.contains("Body::Text("), "{printed}");
        assert!(printed.contains("\"alice\""), "{printed}");

        let drawn = Message::view("t1", "tool", || "hello");
        let printed = format!("{drawn:?}");
        assert!(printed.contains("Body::View(..)"), "{printed}");
        assert!(printed.contains("\"tool\""), "{printed}");
    }
}
