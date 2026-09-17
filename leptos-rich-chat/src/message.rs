//! The chat's data model: the messages in the transcript.

use leptos::prelude::*;

/// One message in the transcript.
///
/// Its `name` says who it is from: a name the host chooses, such as
/// `user` or `alice`. The crate attaches no meaning to it. The bubble
/// carries it as `data-name`, the host's [`Names`](crate::Names) table
/// says where bubbles with that name sit and what colors they have, and
/// with [`Chat`](crate::Chat)'s `show_names` it is written over each
/// bubble.
///
/// The content is a signal so that a message can grow while its text is
/// still arriving: hold an `RwSignal<String>`, append to it, and the
/// bubble re-renders only the blocks that changed. A message whose text
/// is final is built with [`Message::new`]; a growing one with
/// [`Message::streaming`], which also renders it progressively (an open
/// `$$` shows as math before its closing delimiter has arrived).
///
/// The transcript view keys on `id` and `live`, so changing a message's
/// text means writing to its content signal, not replacing the message
/// with another of the same id.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    /// Unique within the transcript.
    pub id: String,
    /// Who the message is from, as the host names them.
    pub name: String,
    /// What they said, as Markdown.
    pub content: Signal<String>,
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
            content: Signal::stored(content.into()),
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
            content: content.into(),
            live: true,
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

    #[test]
    fn a_new_message_is_final_and_holds_its_text() {
        let message = Message::new("m1", "alice", "hello");
        assert_eq!(message.id, "m1");
        assert_eq!(message.name, "alice");
        assert!(!message.live);
        assert_eq!(message.content.get_untracked(), "hello");
    }

    #[test]
    fn a_streaming_message_follows_its_signal() {
        let text = RwSignal::new(String::from("par"));
        let message = Message::streaming(String::from("r1"), "bob", text);
        assert!(message.live);
        assert_eq!(message.content.get_untracked(), "par");
        text.update(|t| t.push_str("tial"));
        assert_eq!(message.content.get_untracked(), "partial");
    }

    #[test]
    fn finishing_keeps_everything_but_the_live_flag() {
        let text = RwSignal::new(String::from("done"));
        let live = Message::streaming("r1", "bob", text);
        let finished = live.clone().finished();
        assert!(!finished.live);
        assert_eq!(finished.id, live.id);
        assert_eq!(finished.name, live.name);
        assert_eq!(finished.content, live.content);
        assert_ne!(finished, live);
    }
}
