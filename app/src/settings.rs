//! What the controls set, and where it is kept between runs.
//!
//! Settings live in the browser's local storage (the webview's, under
//! Tauri), one key each, so a value that was never chosen is simply
//! absent and the app falls back to the system's or the defaults.

use leptos_rich_chat::{Kinds, Look, Position};
use serde::{Deserialize, Serialize};

const THEME_KEY: &str = "rich-chat.theme";
const USERS_KEY: &str = "rich-chat.users";
const INPUT_HEIGHT_KEY: &str = "rich-chat.input-height";

/// Light or dark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    /// The value of the root element's `data-theme` attribute, which the
    /// library's stylesheet keys its palette on.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }

    /// The other one.
    pub fn toggled(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

/// Where a user's bubbles land across the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Left,
    Center,
    Right,
}

impl Side {
    /// The side's name, the value of its button.
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Center => "center",
            Side::Right => "right",
        }
    }

    /// The library's position for bubbles on this side.
    pub fn position(self) -> Position {
        match self {
            Side::Left => Position::Left,
            Side::Center => Position::Center,
            Side::Right => Position::Right,
        }
    }
}

/// Someone in the chat. The name is the kind of their messages, and the
/// side and color are how their bubbles look.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub side: Side,
    /// The bubble background as `#rrggbb`.
    pub color: String,
}

impl User {
    fn new(name: &str, side: Side, color: &str) -> Self {
        Self {
            name: name.to_string(),
            side,
            color: color.to_string(),
        }
    }

    /// How this user's bubbles look: on their side, in their color, with
    /// the text color that reads best on it.
    pub fn look(&self) -> Look {
        let look = Look::at(self.side.position()).background(self.color.clone());
        match text_on(&self.color) {
            Some(text) => look.foreground(text),
            None => look,
        }
    }
}

/// The user the welcome is from, and who is there at the start.
pub const ASSISTANT: &str = "Assistant";
/// The other user there at the start, and the one selected.
pub const USER: &str = "User";

/// The colors new users get, in turn.
const PALETTE: [&str; 6] = [
    "#7c3aed", "#c2410c", "#0e7490", "#be185d", "#4d7c0f", "#6b7280",
];

/// Everything the controls set.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// The chosen theme; `None` follows the system.
    pub theme: Option<Theme>,
    /// The users in the list, in the order they were added.
    pub users: Vec<User>,
    /// Users deleted from the list. Their bubbles keep the look they
    /// had, so the look is kept.
    pub retired: Vec<User>,
    /// The user the controls edit and messages are sent as.
    pub selected: Option<String>,
    /// The text box's height in CSS pixels, once it has been dragged.
    pub input_height: Option<f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: None,
            users: vec![
                User::new(ASSISTANT, Side::Left, "#2a64c8"),
                User::new(USER, Side::Right, "#2f855a"),
            ],
            retired: Vec::new(),
            selected: Some(USER.to_string()),
            input_height: None,
        }
    }
}

/// The users as they are stored: one JSON value under one key.
#[derive(Serialize, Deserialize)]
struct StoredUsers {
    users: Vec<User>,
    retired: Vec<User>,
    selected: Option<String>,
}

impl Settings {
    /// The selected user, if the selection names one in the list.
    pub fn selected_user(&self) -> Option<&User> {
        let name = self.selected.as_deref()?;
        self.users.iter().find(|user| user.name == name)
    }

    /// The selected user, to change.
    pub fn selected_user_mut(&mut self) -> Option<&mut User> {
        let name = self.selected.clone()?;
        self.users.iter_mut().find(|user| user.name == name)
    }

    /// Selects `name`, or nobody for a name not in the list.
    pub fn select(&mut self, name: &str) {
        self.selected = self
            .users
            .iter()
            .find(|user| user.name == name)
            .map(|user| user.name.clone());
    }

    /// Whether `name`, trimmed, could be added: not empty and not already
    /// in the list.
    pub fn can_add(&self, name: &str) -> bool {
        let name = name.trim();
        !name.is_empty() && !self.users.iter().any(|user| user.name == name)
    }

    /// Adds and selects a user called `name`, on the left in the next
    /// color of the palette. Nothing happens for a name that cannot be
    /// added; a deleted user of that name is forgotten.
    pub fn add_user(&mut self, name: &str) -> bool {
        if !self.can_add(name) {
            return false;
        }
        let name = name.trim();
        self.retired.retain(|user| user.name != name);
        let color = PALETTE[(self.users.len() + self.retired.len()) % PALETTE.len()];
        self.users.push(User::new(name, Side::Left, color));
        self.selected = Some(name.to_string());
        true
    }

    /// Removes the selected user from the list, leaving nobody selected.
    /// The user's bubbles keep their look.
    pub fn delete_selected(&mut self) {
        let Some(name) = self.selected.take() else {
            return;
        };
        if let Some(at) = self.users.iter().position(|user| user.name == name) {
            let user = self.users.remove(at);
            self.retired.retain(|retired| retired.name != user.name);
            self.retired.push(user);
        }
    }

    /// The library's table of kinds: every user there has been, with the
    /// current list last so that it wins over a deleted namesake.
    pub fn kinds(&self) -> Kinds {
        self.retired
            .iter()
            .chain(&self.users)
            .fold(Kinds::none(), |kinds, user| {
                kinds.kind(user.name.clone(), user.look())
            })
    }

    /// What was stored last time, or the defaults.
    pub fn load() -> Self {
        let Some(storage) = storage() else {
            return Self::default();
        };
        let read = |key: &str| storage.get_item(key).ok().flatten();
        let defaults = Self::default();
        let stored =
            read(USERS_KEY).and_then(|json| serde_json::from_str::<StoredUsers>(&json).ok());
        let (users, retired, selected) = match stored {
            Some(stored) => (stored.users, stored.retired, stored.selected),
            None => (defaults.users, defaults.retired, defaults.selected),
        };
        let mut settings = Settings {
            theme: read(THEME_KEY).as_deref().and_then(Theme::parse),
            users,
            retired,
            selected: None,
            input_height: read(INPUT_HEIGHT_KEY)
                .and_then(|value| value.parse().ok())
                .filter(|height: &f64| height.is_finite() && *height > 0.0),
        };
        if let Some(name) = selected {
            settings.select(&name);
        }
        settings
    }

    /// Keeps these for next time. A setting that is unset is removed, so
    /// that it follows the system again.
    pub fn store(&self) {
        let Some(storage) = storage() else { return };
        let write = |key: &str, value: Option<String>| {
            let _ = match value {
                Some(value) => storage.set_item(key, &value),
                None => storage.remove_item(key),
            };
        };
        write(
            THEME_KEY,
            self.theme.map(|theme| theme.as_str().to_string()),
        );
        let users = StoredUsers {
            users: self.users.clone(),
            retired: self.retired.clone(),
            selected: self.selected.clone(),
        };
        write(USERS_KEY, serde_json::to_string(&users).ok());
        write(
            INPUT_HEIGHT_KEY,
            self.input_height.map(|height| height.to_string()),
        );
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Whether the system prefers a dark color scheme right now.
pub fn system_prefers_dark() -> bool {
    web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-color-scheme: dark)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}

/// The library's text color on light backgrounds (`--rc-fg` in the light
/// theme) and on dark ones (the same in the dark theme).
const DARK_TEXT: &str = "#1f2328";
const LIGHT_TEXT: &str = "#e6edf3";

/// The text color that reads best on `background`: the library's own
/// ink for light backgrounds or its ink for dark ones, whichever has the
/// higher WCAG contrast. `None` when `background` is not `#rrggbb`.
pub fn text_on(background: &str) -> Option<&'static str> {
    let bg = luminance(background)?;
    let contrast = |ink: &str| {
        let ink = luminance(ink).expect("the inks are valid colors");
        (bg.max(ink) + 0.05) / (bg.min(ink) + 0.05)
    };
    Some(if contrast(DARK_TEXT) >= contrast(LIGHT_TEXT) {
        DARK_TEXT
    } else {
        LIGHT_TEXT
    })
}

/// The relative luminance of `#rrggbb`, as WCAG defines it.
fn luminance(color: &str) -> Option<f64> {
    let (r, g, b) = parse_hex(color)?;
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b))
}

/// `#rrggbb`, which is what a color input produces, to its channels.
fn parse_hex(color: &str) -> Option<(u8, u8, u8)> {
    let digits = color.strip_prefix('#')?;
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_parse_and_nothing_else_does() {
        assert_eq!(parse_hex("#ff8800"), Some((255, 136, 0)));
        assert_eq!(parse_hex("#FF8800"), Some((255, 136, 0)));
        for bad in ["ff8800", "#ff880", "#ff88000", "#gg8800", "#ffé80", ""] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn text_follows_the_background() {
        assert_eq!(text_on("#ffffff"), Some(DARK_TEXT));
        assert_eq!(text_on("#000000"), Some(LIGHT_TEXT));
        assert_eq!(
            text_on("#ff8800"),
            Some(DARK_TEXT),
            "orange reads better in dark text"
        );
        assert_eq!(
            text_on("#0969da"),
            Some(LIGHT_TEXT),
            "the accent blue in light text"
        );
        assert_eq!(text_on("not a color"), None);
    }

    #[test]
    fn luminance_is_wcag() {
        assert_eq!(luminance("#000000"), Some(0.0));
        assert_eq!(luminance("#ffffff"), Some(1.0));
        let red = luminance("#ff0000").unwrap();
        assert!((red - 0.2126).abs() < 1e-9, "{red}");
    }

    #[test]
    fn themes_round_trip() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
            assert_eq!(theme.toggled().toggled(), theme);
            assert_ne!(theme.toggled(), theme);
        }
        assert_eq!(Theme::parse("blue"), None);
    }

    #[test]
    fn the_defaults_are_an_assistant_and_a_user() {
        let settings = Settings::default();
        let names: Vec<_> = settings
            .users
            .iter()
            .map(|user| user.name.as_str())
            .collect();
        assert_eq!(names, [ASSISTANT, USER]);
        assert_eq!(settings.selected_user().unwrap().name, USER);
        let kinds = settings.kinds();
        assert_eq!(kinds.get(ASSISTANT).unwrap().position, Position::Left);
        assert_eq!(kinds.get(USER).unwrap().position, Position::Right);
        assert_eq!(
            kinds.get(USER).unwrap().background.as_deref(),
            Some("#2f855a")
        );
        assert_eq!(
            kinds.get(USER).unwrap().foreground.as_deref(),
            Some(LIGHT_TEXT)
        );
        assert_eq!(
            kinds.get("user"),
            None,
            "the names are the kinds, as spelled"
        );
    }

    #[test]
    fn a_user_is_added_selected_and_edited() {
        let mut settings = Settings::default();
        assert!(!settings.can_add("  "));
        assert!(!settings.can_add("User"));
        assert!(settings.can_add("Alice"));
        assert!(settings.add_user(" Alice "));
        assert!(!settings.add_user("Alice"), "no two Alices");
        let alice = settings.selected_user().unwrap();
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.side, Side::Left);
        assert_eq!(
            alice.color, PALETTE[2],
            "the third user gets the third color"
        );

        settings.selected_user_mut().unwrap().side = Side::Center;
        settings.selected_user_mut().unwrap().color = "#ff8800".to_string();
        let look = settings.kinds().get("Alice").cloned().unwrap();
        assert_eq!(look.position, Position::Center);
        assert_eq!(look.background.as_deref(), Some("#ff8800"));
        assert_eq!(look.foreground.as_deref(), Some(DARK_TEXT));
    }

    #[test]
    fn a_deleted_user_keeps_their_look_and_leaves_nobody_selected() {
        let mut settings = Settings::default();
        settings.add_user("Alice");
        let before = settings.kinds().get("Alice").cloned().unwrap();
        settings.delete_selected();
        assert_eq!(settings.selected, None);
        assert_eq!(settings.selected_user(), None);
        assert_eq!(settings.users.len(), 2);
        assert_eq!(settings.kinds().get("Alice"), Some(&before));
        assert!(settings.can_add("Alice"), "and can come back");
        settings.delete_selected();
        assert_eq!(settings.users.len(), 2, "deleting nobody does nothing");

        // Back with a new look, the old one is forgotten.
        settings.add_user("Alice");
        settings.selected_user_mut().unwrap().color = "#000000".to_string();
        assert_eq!(settings.retired.len(), 0);
        assert_eq!(
            settings.kinds().get("Alice").unwrap().background.as_deref(),
            Some("#000000")
        );
    }

    #[test]
    fn selecting_names_a_user_in_the_list_or_nobody() {
        let mut settings = Settings::default();
        settings.select(ASSISTANT);
        assert_eq!(settings.selected.as_deref(), Some(ASSISTANT));
        settings.select("nobody");
        assert_eq!(settings.selected, None);
    }

    #[test]
    fn the_users_round_trip_through_json() {
        let mut settings = Settings::default();
        settings.add_user("Alice");
        settings.delete_selected();
        settings.select(ASSISTANT);
        let stored = StoredUsers {
            users: settings.users.clone(),
            retired: settings.retired.clone(),
            selected: settings.selected.clone(),
        };
        let json = serde_json::to_string(&stored).unwrap();
        assert!(json.contains("\"side\":\"right\""), "{json}");
        let back: StoredUsers = serde_json::from_str(&json).unwrap();
        assert_eq!(back.users, settings.users);
        assert_eq!(back.retired, settings.retired);
        assert_eq!(back.selected, settings.selected);
    }
}
