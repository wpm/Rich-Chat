//! The names a host's messages are from, and how each one's bubbles look.
//!
//! The crate has no idea who says what. A [`Message`](crate::Message)
//! carries a `name`, which the host chooses, and the bubble carries it
//! as a `data-name` attribute. What a name looks like is the host's too,
//! declared in a [`Names`] table: where the bubbles sit and what colors
//! they have. [`RichChatStyle`](crate::RichChatStyle) turns the table
//! into rules per name, in the `rich-chat.theme` cascade layer, so a
//! host's own stylesheet still wins over them. A name with no entry gets
//! the plain bubble: on the left, in the theme's `--rc-bubble-*` colors.
//!
//! The default table has two names, `user` on the right in the theme's
//! tint colors and `assistant` on the left, which suits a chat with a
//! model. Anything else, a group chat or a transcript with notices in
//! the middle, is a different table:
//!
//! ```
//! use leptos_rich_chat::{Look, Names, Position};
//!
//! let names = Names::none()
//!     .name("me", Look::at(Position::Right).background("#ddf4ff").foreground("#1f2328"))
//!     .name("alice", Look::at(Position::Left).background("var(--alice)"))
//!     .name("notice", Look::at(Position::Center).background("transparent").foreground("var(--rc-muted)"));
//! ```
//!
//! Position and the two colors are the only properties here, because
//! they are what a stylesheet cannot express without knowing the name.
//! Everything else about a name (its font size, a border, an avatar, or
//! that a `notice` shows no name over its bubble) is ordinary CSS
//! against `.rc-message[data-name="…"]`.

use std::fmt::Write;

/// Where a bubble sits across the transcript. Physical, not logical: a
/// host that wants the sides to follow the text direction swaps them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Position {
    /// Against the left edge, with the bubble's tail at bottom left.
    #[default]
    Left,
    /// In the middle, with no tail.
    Center,
    /// Against the right edge, with the tail at bottom right.
    Right,
}

impl Position {
    /// The position's name: `left`, `center`, `right`.
    pub fn as_str(self) -> &'static str {
        match self {
            Position::Left => "left",
            Position::Center => "center",
            Position::Right => "right",
        }
    }
}

/// How the bubbles from one name look.
///
/// The colors are CSS values, inserted into the generated stylesheet as
/// they are: a color, a `var(--…)`, a `light-dark(…, …)`. A color
/// left out is the theme's plain bubble color.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Look {
    /// Where the bubble sits.
    pub position: Position,
    /// The bubble's background, or the theme's if `None`.
    pub background: Option<String>,
    /// The bubble's text color, or the theme's if `None`.
    pub foreground: Option<String>,
}

impl Look {
    /// A look at `position` in the theme's colors.
    pub fn at(position: Position) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    /// The same look with this background.
    pub fn background(mut self, css: impl Into<String>) -> Self {
        self.background = Some(css.into());
        self
    }

    /// The same look with this text color.
    pub fn foreground(mut self, css: impl Into<String>) -> Self {
        self.foreground = Some(css.into());
        self
    }
}

/// The names the host's messages are from, each with its [`Look`], in
/// the order they were added. [`Names::default`] is `user` and
/// `assistant`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Names {
    entries: Vec<(String, Look)>,
}

impl Default for Names {
    /// `user` on the right in the theme's tint colors (`--rc-tint-bg`
    /// and `--rc-tint-fg`), `assistant` on the left in its plain ones.
    fn default() -> Self {
        Self::none()
            .name(
                "user",
                Look::at(Position::Right)
                    .background("var(--rc-tint-bg)")
                    .foreground("var(--rc-tint-fg)"),
            )
            .name("assistant", Look::at(Position::Left))
    }
}

impl Names {
    /// No names at all: every bubble gets the plain look.
    pub fn none() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The table with `name` looking like `look`. A name already in the
    /// table is replaced in place.
    pub fn name(mut self, name: impl Into<String>, look: Look) -> Self {
        let name = name.into();
        match self
            .entries
            .iter_mut()
            .find(|(existing, _)| *existing == name)
        {
            Some(entry) => entry.1 = look,
            None => self.entries.push((name, look)),
        }
        self
    }

    /// The look of `name`, if the table has it.
    pub fn get(&self, name: &str) -> Option<&Look> {
        self.entries
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, look)| look)
    }

    /// The names and their looks, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Look)> {
        self.entries
            .iter()
            .map(|(name, look)| (name.as_str(), look))
    }

    /// The stylesheet for the table, in the layer `rich-chat.theme`, the
    /// same form as the constants in [`style`](crate::style). For each
    /// name, one rule places its messages: `align-items` on the
    /// `.rc-message` column, which puts the bubble and the name written
    /// over it on the same side. On a side, a second rule gives the
    /// bubble its tail there, the theme's `--rc-tail`. Colors given are
    /// a last rule, on the bubble and on a composer preview of that
    /// name.
    pub fn css(&self) -> String {
        let mut css = String::from(
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n",
        );
        for (name, look) in self.iter() {
            let name = css_string(name);
            let message = format!(".rc-message[data-name=\"{name}\"]");
            let bubble = format!("{message} > .rc-bubble");
            let (placement, tail) = match look.position {
                Position::Left => (
                    "flex-start",
                    Some("border-bottom-left-radius: var(--rc-tail, 0);"),
                ),
                Position::Center => ("center", None),
                Position::Right => (
                    "flex-end",
                    Some("border-bottom-right-radius: var(--rc-tail, 0);"),
                ),
            };
            let _ = writeln!(css, "{message} {{ align-items: {placement}; }}");
            if let Some(tail) = tail {
                let _ = writeln!(css, "{bubble} {{ {tail} }}");
            }
            let mut colors = String::new();
            if let Some(background) = &look.background {
                let _ = write!(colors, " background: {background};");
            }
            if let Some(foreground) = &look.foreground {
                let _ = write!(colors, " color: {foreground};");
            }
            if !colors.is_empty() {
                let _ = writeln!(
                    css,
                    ":is({bubble}, .rc-composer-preview[data-name=\"{name}\"]) {{{colors} }}"
                );
            }
        }
        css.push_str("}\n");
        css
    }
}

/// `text` as the inside of a double-quoted CSS string.
fn css_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => {
                let _ = write!(out, "\\{:x} ", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_chat_with_a_model() {
        let names = Names::default();
        let listed: Vec<_> = names.iter().map(|(name, _)| name).collect();
        assert_eq!(listed, ["user", "assistant"]);
        assert_eq!(names.get("user").unwrap().position, Position::Right);
        assert_eq!(names.get("assistant").unwrap().position, Position::Left);
        assert_eq!(names.get("assistant").unwrap().background, None);
        assert_eq!(names.get("system"), None);
    }

    #[test]
    fn a_name_added_twice_is_replaced_in_place() {
        let names = Names::none()
            .name("a", Look::at(Position::Left))
            .name("b", Look::at(Position::Right))
            .name("a", Look::at(Position::Center));
        let listed: Vec<_> = names.iter().map(|(name, _)| name).collect();
        assert_eq!(listed, ["a", "b"]);
        assert_eq!(names.get("a").unwrap().position, Position::Center);
    }

    #[test]
    fn each_position_places_the_message_and_only_colors_given_are_set() {
        let css = Names::none()
            .name("l", Look::at(Position::Left))
            .name("c", Look::at(Position::Center).background("teal"))
            .name(
                "r",
                Look::at(Position::Right)
                    .background("#fff")
                    .foreground("#000"),
            )
            .css();
        assert!(css.starts_with(
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n"
        ));
        assert!(css.trim_end().ends_with('}'));
        assert!(
            css.contains(".rc-message[data-name=\"l\"] { align-items: flex-start; }\n"),
            "{css}"
        );
        assert!(css.contains(
            ".rc-message[data-name=\"l\"] > .rc-bubble { border-bottom-left-radius: var(--rc-tail, 0); }\n"
        ), "{css}");
        assert!(
            css.contains(".rc-message[data-name=\"c\"] { align-items: center; }\n"),
            "{css}"
        );
        assert!(
            !css.contains("data-name=\"c\"] > .rc-bubble {"),
            "a centered bubble has no tail: {css}"
        );
        assert!(
            css.contains(".rc-message[data-name=\"r\"] { align-items: flex-end; }\n"),
            "{css}"
        );
        assert!(css.contains(
            ".rc-message[data-name=\"r\"] > .rc-bubble { border-bottom-right-radius: var(--rc-tail, 0); }\n"
        ), "{css}");
        assert!(
            !css.contains("data-name=\"l\"]) {"),
            "l has no color rule: {css}"
        );
        assert!(css.contains(
            ":is(.rc-message[data-name=\"c\"] > .rc-bubble, .rc-composer-preview[data-name=\"c\"]) { background: teal; }\n"
        ), "{css}");
        assert!(css.contains(
            ":is(.rc-message[data-name=\"r\"] > .rc-bubble, .rc-composer-preview[data-name=\"r\"]) { background: #fff; color: #000; }\n"
        ), "{css}");
    }

    #[test]
    fn no_names_is_an_empty_layer() {
        let css = Names::none().css();
        assert_eq!(
            css,
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n}\n"
        );
    }

    #[test]
    fn names_are_escaped_in_the_selector() {
        let css = Names::none()
            .name("say \"hi\"\\\n", Look::at(Position::Left))
            .css();
        assert!(
            css.contains("[data-name=\"say \\\"hi\\\"\\\\\\a \"]"),
            "{css}"
        );
    }

    #[test]
    fn positions_have_names() {
        assert_eq!(Position::Left.as_str(), "left");
        assert_eq!(Position::Center.as_str(), "center");
        assert_eq!(Position::Right.as_str(), "right");
        assert_eq!(Position::default(), Position::Left);
    }

    #[test]
    fn the_default_reads_only_theme_properties() {
        let css = Names::default().css();
        assert!(
            css.contains("background: var(--rc-tint-bg); color: var(--rc-tint-fg);"),
            "{css}"
        );
        assert!(!css.contains('#'), "{css}");
    }
}
