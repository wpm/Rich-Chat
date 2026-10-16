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
//! Position and the two colors are what a [`Look`] sets itself, because
//! they are what a stylesheet cannot express without knowing the name;
//! a value more than the bubble needs is a custom property on it (see
//! [`Look::property`]). Everything else about a name (its font size, an
//! avatar, or that a `notice` shows no name over its bubble) is
//! ordinary CSS against `.rc-message[data-name="…"]`.

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
    /// Custom properties, name and value, in order. See [`Look::property`].
    pub properties: Vec<(String, String)>,
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

    /// The same look with the custom property `name` set to `css` on the
    /// name's messages and its composer preview, where everything inside
    /// inherits it for the host's own rules to read. `css` is inserted as
    /// it is; a `name` that is not `--` and an identifier is left out.
    pub fn property(mut self, name: impl Into<String>, css: impl Into<String>) -> Self {
        self.properties.push((name.into(), css.into()));
        self
    }

    /// The look's custom properties that are custom properties' names,
    /// as declarations each led by a space: ` --side: blue;`.
    fn declarations(&self) -> String {
        self.properties
            .iter()
            .filter(|(name, _)| is_custom_property(name))
            .map(|(name, css)| format!(" {name}: {css};"))
            .collect()
    }
}

/// Whether `name` is a custom property's: `--` and at least one more
/// character, each a letter, a digit, `-`, `_`, or anything past ASCII.
fn is_custom_property(name: &str) -> bool {
    name.strip_prefix("--").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii())
    })
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
    /// name. Custom properties go on the first rule and on one of
    /// their own for the preview.
    pub fn css(&self) -> String {
        let mut css = String::from(
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n",
        );
        for (name, look) in self.iter() {
            let name = css_string(name);
            let message = format!(".rc-message[data-name=\"{name}\"]");
            let bubble = format!("{message} > .rc-bubble");
            let preview = format!(".rc-composer-preview[data-name=\"{name}\"]");
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
            let properties = look.declarations();
            let _ = writeln!(css, "{message} {{ align-items: {placement};{properties} }}");
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
                let _ = writeln!(css, ":is({bubble}, {preview}) {{{colors} }}");
            }
            if !properties.is_empty() {
                let _ = writeln!(css, "{preview} {{{properties} }}");
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
    fn properties_are_written_in_order_on_the_message_and_the_preview() {
        let css = Names::none()
            .name(
                "p6",
                Look::at(Position::Right)
                    .background("transparent")
                    .property("--side", "var(--sd-good)")
                    .property("--seat", "light-dark(#123, #abc)"),
            )
            .css();
        assert!(
            css.contains(
                ".rc-message[data-name=\"p6\"] { align-items: flex-end; \
                 --side: var(--sd-good); --seat: light-dark(#123, #abc); }\n"
            ),
            "{css}"
        );
        assert!(
            css.contains(
                ".rc-composer-preview[data-name=\"p6\"] { \
                 --side: var(--sd-good); --seat: light-dark(#123, #abc); }\n"
            ),
            "{css}"
        );
    }

    #[test]
    fn a_look_without_properties_writes_no_more_than_before() {
        let css = Names::none()
            .name("a", Look::at(Position::Center).background("teal"))
            .css();
        assert_eq!(
            css,
            "@layer rich-chat.structure, rich-chat.theme;\n@layer rich-chat.theme {\n\
             .rc-message[data-name=\"a\"] { align-items: center; }\n\
             :is(.rc-message[data-name=\"a\"] > .rc-bubble, .rc-composer-preview[data-name=\"a\"]) { background: teal; }\n\
             }\n"
        );
    }

    #[test]
    fn only_custom_property_names_are_written() {
        let look = Look::at(Position::Left)
            .property("color", "red")
            .property("--", "red")
            .property("-x", "red")
            .property("--a b", "red")
            .property("--a;color", "red")
            .property("--a}", "red")
            .property("--ok_1-é", "blue");
        assert_eq!(look.properties.len(), 7, "the look keeps what it is given");
        let css = Names::none().name("n", look).css();
        assert!(!css.contains("red"), "{css}");
        assert!(
            css.contains(
                ".rc-message[data-name=\"n\"] { align-items: flex-start; --ok_1-é: blue; }\n"
            ),
            "{css}"
        );
        assert!(
            css.contains(".rc-composer-preview[data-name=\"n\"] { --ok_1-é: blue; }\n"),
            "{css}"
        );

        let none = Names::none()
            .name("n", Look::at(Position::Left).property("color", "red"))
            .css();
        assert!(!none.contains("rc-composer-preview"), "{none}");
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
