//! The conversation: the messages in the window, and which users are
//! thinking.
//!
//! A thinking user is a live message with no text yet, which the library
//! draws as dancing dots in that user's bubble. Sending as a thinking
//! user writes into that bubble, which turns the dots into text in place,
//! rather than adding a new one. Not a setting: every run starts with no
//! one thinking.

use leptos::prelude::*;
use leptos_rich_chat::Message;

/// A thinking user's bubble: the id of its message and the text that
/// fills it. Sending as them hands it back, for the text to arrive
/// through `text` while the bubble is live, until [`Transcript::finish`]
/// with `id` makes it final.
#[derive(Clone)]
pub struct Reply {
    pub id: String,
    pub text: RwSignal<String>,
}

pub struct Transcript {
    messages: Vec<Message>,
    /// The thinking users by name, in the order they started.
    thinking: Vec<(String, Reply)>,
    /// The number in the next message's id. Ids are never reused, even
    /// once a bubble is removed, since the transcript keys on them.
    next: usize,
}

impl Transcript {
    /// A transcript that opens on `messages`. Ids for the messages after
    /// them count on from how many there are.
    pub fn new(messages: Vec<Message>) -> Self {
        let next = messages.len();
        Self {
            messages,
            thinking: Vec::new(),
            next,
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn is_thinking(&self, name: &str) -> bool {
        self.thinking.iter().any(|(thinking, _)| thinking == name)
    }

    /// Stops `name` thinking, handing back their bubble.
    fn take_thinker(&mut self, name: &str) -> Option<Reply> {
        let at = self
            .thinking
            .iter()
            .position(|(thinking, _)| thinking == name)?;
        Some(self.thinking.remove(at).1)
    }

    fn take_id(&mut self) -> String {
        let id = format!("m{}", self.next);
        self.next += 1;
        id
    }

    /// Starts or stops `name` thinking: a new empty live bubble of theirs
    /// at the end, or that bubble removed.
    pub fn toggle_thinking(&mut self, name: &str) {
        if self.is_thinking(name) {
            self.stop_thinking(name);
        } else {
            let id = self.take_id();
            let text = RwSignal::new(String::new());
            self.messages
                .push(Message::streaming(id.clone(), name, text));
            self.thinking.push((name.to_string(), Reply { id, text }));
        }
    }

    /// Removes `name`'s thinking bubble, if they have one.
    pub fn stop_thinking(&mut self, name: &str) {
        if let Some(thinker) = self.take_thinker(name) {
            self.messages.retain(|message| message.id != thinker.id);
        }
    }

    /// Sends `text` as `name`. A user who is not thinking gets a new
    /// final bubble at the end. A thinking user stops thinking and gets
    /// back their bubble, for the caller to write `text` into; it stays
    /// where it is, and live until [`finish`](Self::finish).
    pub fn send(&mut self, name: &str, text: String) -> Option<Reply> {
        let reply = self.take_thinker(name);
        if reply.is_none() {
            let id = self.take_id();
            self.messages.push(Message::new(id, name, text));
        }
        reply
    }

    /// Makes the bubble with `id` final.
    pub fn finish(&mut self, id: &str) {
        if let Some(message) = self.messages.iter_mut().find(|message| message.id == id) {
            message.live = false;
        }
    }
}

/// Where the next piece of a reply being dripped into its bubble ends:
/// `step` characters past `from`, or the end of `text`. `from` and the
/// result are byte offsets on character boundaries.
pub fn drip_end(text: &str, from: usize, step: usize) -> usize {
    text[from..]
        .char_indices()
        .nth(step)
        .map_or(text.len(), |(at, _)| from + at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos_rich_chat::Body;

    fn tour() -> Transcript {
        Transcript::new(vec![
            Message::new("welcome", "A", "hi"),
            Message::new("welcome-2", "U", "hello"),
        ])
    }

    fn ids(transcript: &Transcript) -> Vec<&str> {
        transcript
            .messages()
            .iter()
            .map(|message| message.id.as_str())
            .collect()
    }

    fn text(message: &Message) -> String {
        match &message.body {
            Body::Text(text) => text.get_untracked(),
            Body::View(_) => panic!("a view"),
        }
    }

    #[test]
    fn a_send_appends_a_final_message_after_the_tour() {
        let mut transcript = tour();
        assert!(transcript.send("U", "one".into()).is_none());
        let last = transcript.messages().last().unwrap();
        assert_eq!(last.id, "m2");
        assert_eq!(last.name, "U");
        assert!(!last.live);
        assert_eq!(text(last), "one");
    }

    #[test]
    fn thinking_is_an_empty_live_bubble_of_the_users() {
        let mut transcript = tour();
        assert!(!transcript.is_thinking("A"));
        transcript.toggle_thinking("A");
        assert!(transcript.is_thinking("A"));
        assert!(!transcript.is_thinking("U"));
        let last = transcript.messages().last().unwrap();
        assert_eq!((last.id.as_str(), last.name.as_str()), ("m2", "A"));
        assert!(last.live);
        assert_eq!(text(last), "");

        transcript.toggle_thinking("A");
        assert!(!transcript.is_thinking("A"));
        assert_eq!(ids(&transcript), ["welcome", "welcome-2"]);
    }

    #[test]
    fn ids_are_never_reused() {
        let mut transcript = tour();
        transcript.toggle_thinking("A");
        transcript.toggle_thinking("A");
        transcript.send("U", "after".into());
        assert_eq!(ids(&transcript), ["welcome", "welcome-2", "m3"]);
    }

    #[test]
    fn several_users_think_at_once() {
        let mut transcript = tour();
        transcript.toggle_thinking("A");
        transcript.toggle_thinking("U");
        assert!(transcript.is_thinking("A") && transcript.is_thinking("U"));
        assert_eq!(ids(&transcript)[2..], ["m2", "m3"]);
    }

    #[test]
    fn a_thinker_sends_into_their_own_bubble() {
        let mut transcript = tour();
        transcript.toggle_thinking("A");
        transcript.toggle_thinking("U");
        let reply = transcript.send("A", "done".into()).expect("A was thinking");
        assert!(!transcript.is_thinking("A"), "A is no longer thinking");
        assert!(transcript.is_thinking("U"), "U still is");
        assert_eq!(ids(&transcript)[2..], ["m2", "m3"], "nothing new appended");

        reply.text.set("done".into());
        let bubble = &transcript.messages()[2];
        assert_eq!(text(bubble), "done");
        assert!(bubble.live, "live until finished");
        transcript.finish(&reply.id);
        assert!(!transcript.messages()[2].live);
        assert_eq!(text(&transcript.messages()[2]), "done");
        assert!(transcript.messages()[3].live, "U's bubble is untouched");
    }

    #[test]
    fn stopping_a_user_who_is_not_thinking_changes_nothing() {
        let mut transcript = tour();
        transcript.stop_thinking("A");
        assert_eq!(ids(&transcript), ["welcome", "welcome-2"]);
        transcript.toggle_thinking("U");
        transcript.stop_thinking("U");
        assert!(!transcript.is_thinking("U"));
        assert_eq!(ids(&transcript), ["welcome", "welcome-2"]);
    }

    #[test]
    fn drips_end_on_character_boundaries() {
        assert_eq!(drip_end("abcdef", 0, 4), 4);
        assert_eq!(drip_end("abcdef", 4, 4), 6);
        assert_eq!(drip_end("", 0, 4), 0);
        let text = "αβγδε";
        let end = drip_end(text, 0, 2);
        assert_eq!(&text[..end], "αβ");
        assert_eq!(&text[end..drip_end(text, end, 2)], "γδ");
    }
}
