//! The chat's data model: who said what.

use leptos::prelude::*;

/// Who a message is from. Decides which side of the window it sits on
/// and how its bubble is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The person typing.
    User,
    /// The model, or whoever answers.
    Assistant,
    /// A note from the application itself, centred and muted.
    System,
}

impl Role {
    /// The role's name as it appears in CSS classes: `user`, `assistant`,
    /// `system`.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
        }
    }
}

/// One message in the transcript.
///
/// The content is a signal so that a message can grow while a model
/// streams it: hold an `RwSignal<String>`, append to it, and the bubble
/// re-renders only the blocks that changed. A message whose text is
/// final is built with [`Message::new`]; a growing one with
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
    /// Who said it.
    pub role: Role,
    /// What they said, as Markdown.
    pub content: Signal<String>,
    /// Whether the text is still arriving. Live messages are rendered as
    /// drafts: constructs left open at the end are closed for display.
    pub live: bool,
}

impl Message {
    /// A message whose text is final.
    pub fn new(id: impl Into<String>, role: Role, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role,
            content: Signal::stored(content.into()),
            live: false,
        }
    }

    /// A message whose text is still arriving through `content`.
    pub fn streaming(
        id: impl Into<String>,
        role: Role,
        content: impl Into<Signal<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            role,
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
    fn role_names_match_the_css_classes() {
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
        assert_eq!(Role::System.as_str(), "system");
    }

    #[test]
    fn a_new_message_is_final_and_holds_its_text() {
        let message = Message::new("m1", Role::User, "hello");
        assert_eq!(message.id, "m1");
        assert_eq!(message.role, Role::User);
        assert!(!message.live);
        assert_eq!(message.content.get_untracked(), "hello");
    }

    #[test]
    fn a_streaming_message_follows_its_signal() {
        let text = RwSignal::new(String::from("par"));
        let message = Message::streaming(String::from("r1"), Role::Assistant, text);
        assert!(message.live);
        assert_eq!(message.content.get_untracked(), "par");
        text.update(|t| t.push_str("tial"));
        assert_eq!(message.content.get_untracked(), "partial");
    }

    #[test]
    fn finishing_keeps_everything_but_the_live_flag() {
        let text = RwSignal::new(String::from("done"));
        let live = Message::streaming("r1", Role::Assistant, text);
        let finished = live.clone().finished();
        assert!(!finished.live);
        assert_eq!(finished.id, live.id);
        assert_eq!(finished.role, live.role);
        assert_eq!(finished.content, live.content);
        assert_ne!(finished, live);
    }
}
