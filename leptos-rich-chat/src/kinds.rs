//! The kinds of message a host shows, and how each looks.
//!
//! The crate has no idea who says what. A [`Message`](crate::Message)
//! carries a `kind`, a name the host chooses, and the bubble carries it
//! as a `data-kind` attribute. What a kind looks like is the host's too,
//! declared in a [`Kinds`] table: where the bubble sits and what colors
//! it has. [`RichChatStyle`](crate::RichChatStyle) turns the table into
//! one rule per kind, in the `rich-chat.theme` cascade layer, so a host's
//! own stylesheet still wins over it. A kind with no entry gets the plain
//! bubble: on the left, in the theme's `--rc-bubble-*` colors.
//!
//! The default table has two kinds, `user` on the right in the theme's
//! tint colors and `assistant` on the left, which suits a chat with a
//! model. Anything else, a group chat or a transcript with notices in
//! the middle, is a different table:
//!
//! ```
//! use leptos_rich_chat::{Kinds, Look, Position};
//!
//! let kinds = Kinds::none()
//!     .kind("me", Look::at(Position::Right).background("#ddf4ff").foreground("#1f2328"))
//!     .kind("alice", Look::at(Position::Left).background("var(--alice)"))
//!     .kind("notice", Look::at(Position::Center).background("transparent").foreground("var(--rc-muted)"));
//! ```
//!
//! Position and the two colors are the only properties here, because
//! they are what a stylesheet cannot express without knowing the kind's
//! name. Everything else about a kind (its font size, a border, an
//! avatar) is ordinary CSS against `.rc-message[data-kind="…"]`.

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

/// How the bubbles of one kind look.
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

/// The host's kinds of message, each with its [`Look`], in the order
/// they were added. [`Kinds::default`] is `user` and `assistant`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kinds {
    entries: Vec<(String, Look)>,
}

impl Default for Kinds {
    /// `user` on the right in the theme's tint colors (`--rc-tint-bg`
    /// and `--rc-tint-fg`), `assistant` on the left in its plain ones.
    fn default() -> Self {
        Self::none()
            .kind(
                "user",
                Look::at(Position::Right)
                    .background("var(--rc-tint-bg)")
                    .foreground("var(--rc-tint-fg)"),
            )
            .kind("assistant", Look::at(Position::Left))
    }
}

impl Kinds {
    /// No kinds at all: every bubble gets the plain look.
    pub fn none() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The table with `name` looking like `look`. A kind already in the
    /// table is replaced in place.
    pub fn kind(mut self, name: impl Into<String>, look: Look) -> Self {
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

    /// The kinds and their looks, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Look)> {
        self.entries
            .iter()
            .map(|(name, look)| (name.as_str(), look))
    }

    /// The stylesheet for the table, in the layer `rich-chat.theme`, the
    /// same form as the constants in [`style`](crate::style). One rule
    /// per kind places its bubbles; a second gives them their colors,
    /// and the same colors to a composer preview of that kind. The tail
    /// radius is the theme's `--rc-tail`.
    pub fn css(&self) -> String {
        let mut css = String::from(
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n",
        );
        for (name, look) in self.iter() {
            let name = css_string(name);
            let bubble = format!(".rc-message[data-kind=\"{name}\"] > .rc-bubble");
            let placement = match look.position {
                Position::Left => {
                    "margin-right: auto; border-bottom-left-radius: var(--rc-tail, 0);"
                }
                Position::Center => "margin-left: auto; margin-right: auto;",
                Position::Right => {
                    "margin-left: auto; border-bottom-right-radius: var(--rc-tail, 0);"
                }
            };
            let _ = writeln!(css, "{bubble} {{ {placement} }}");
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
                    ":is({bubble}, .rc-composer-preview[data-kind=\"{name}\"]) {{{colors} }}"
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
        let kinds = Kinds::default();
        let names: Vec<_> = kinds.iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["user", "assistant"]);
        assert_eq!(kinds.get("user").unwrap().position, Position::Right);
        assert_eq!(kinds.get("assistant").unwrap().position, Position::Left);
        assert_eq!(kinds.get("assistant").unwrap().background, None);
        assert_eq!(kinds.get("system"), None);
    }

    #[test]
    fn a_kind_added_twice_is_replaced_in_place() {
        let kinds = Kinds::none()
            .kind("a", Look::at(Position::Left))
            .kind("b", Look::at(Position::Right))
            .kind("a", Look::at(Position::Center));
        let names: Vec<_> = kinds.iter().map(|(name, _)| name).collect();
        assert_eq!(names, ["a", "b"]);
        assert_eq!(kinds.get("a").unwrap().position, Position::Center);
    }

    #[test]
    fn each_position_places_the_bubble_and_only_colors_given_are_set() {
        let css = Kinds::none()
            .kind("l", Look::at(Position::Left))
            .kind("c", Look::at(Position::Center).background("teal"))
            .kind(
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
        assert!(css.contains(
            ".rc-message[data-kind=\"l\"] > .rc-bubble { margin-right: auto; border-bottom-left-radius: var(--rc-tail, 0); }\n"
        ), "{css}");
        assert!(css.contains(
            ".rc-message[data-kind=\"c\"] > .rc-bubble { margin-left: auto; margin-right: auto; }\n"
        ), "{css}");
        assert!(css.contains(
            ".rc-message[data-kind=\"r\"] > .rc-bubble { margin-left: auto; border-bottom-right-radius: var(--rc-tail, 0); }\n"
        ), "{css}");
        assert!(
            !css.contains("data-kind=\"l\"]) {"),
            "l has no color rule: {css}"
        );
        assert!(css.contains(
            ":is(.rc-message[data-kind=\"c\"] > .rc-bubble, .rc-composer-preview[data-kind=\"c\"]) { background: teal; }\n"
        ), "{css}");
        assert!(css.contains(
            ":is(.rc-message[data-kind=\"r\"] > .rc-bubble, .rc-composer-preview[data-kind=\"r\"]) { background: #fff; color: #000; }\n"
        ), "{css}");
    }

    #[test]
    fn no_kinds_is_an_empty_layer() {
        let css = Kinds::none().css();
        assert_eq!(
            css,
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n}\n"
        );
    }

    #[test]
    fn kind_names_are_escaped_in_the_selector() {
        let css = Kinds::none()
            .kind("say \"hi\"\\\n", Look::at(Position::Left))
            .css();
        assert!(
            css.contains("[data-kind=\"say \\\"hi\\\"\\\\\\a \"]"),
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
        let css = Kinds::default().css();
        assert!(
            css.contains("background: var(--rc-tint-bg); color: var(--rc-tint-fg);"),
            "{css}"
        );
        assert!(!css.contains('#'), "{css}");
    }
}
