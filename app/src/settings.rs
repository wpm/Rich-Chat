//! What the controls set, and where it is kept between runs.
//!
//! Settings live in the browser's local storage (the webview's, under
//! Tauri), one key each, so a value that was never chosen is simply
//! absent and the app falls back to the system's or the defaults.

use leptos_rich_chat::{Look, Names, Position};
use serde::{Deserialize, Serialize};

const THEME_KEY: &str = "rich-chat.theme";
const USERS_KEY: &str = "rich-chat.users";
const INPUT_HEIGHT_KEY: &str = "rich-chat.input-height";
const SHOW_NAMES_KEY: &str = "rich-chat.show-names";

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

/// Someone in the chat. The name is the one their messages carry, and
/// the side and color are how their bubbles look.
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
    users: Vec<User>,
    /// Users deleted from the list. Their bubbles keep the look they
    /// had, so the look is kept.
    pub retired: Vec<User>,
    /// The user the controls edit and messages are sent as. Always one
    /// in the list, which is never empty. Only the methods below write
    /// the two, which is what keeps that so.
    selected: String,
    /// The text box's height in CSS pixels, once it has been dragged.
    pub input_height: Option<f64>,
    /// Whether every bubble has its user's name written over it.
    pub show_names: bool,
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
            selected: USER.to_string(),
            input_height: None,
            show_names: false,
        }
    }
}

/// The users as they are stored: one JSON value under one key. Generic
/// so that storing borrows the lists and loading owns them.
#[derive(Serialize, Deserialize)]
struct StoredUsers<U = Vec<User>, S = Option<String>> {
    users: U,
    retired: U,
    /// Older builds stored no selection for nobody.
    selected: S,
}

impl Settings {
    /// The users in the list, in the order they were added.
    pub fn users(&self) -> &[User] {
        &self.users
    }

    /// The selected user's name.
    pub fn selected(&self) -> &str {
        &self.selected
    }

    /// Where in the list the selected user is.
    fn selected_at(&self) -> usize {
        self.users
            .iter()
            .position(|user| user.name == self.selected)
            .expect("the selection names a user in the list")
    }

    /// The selected user.
    pub fn selected_user(&self) -> &User {
        &self.users[self.selected_at()]
    }

    /// The selected user, to change.
    pub fn selected_user_mut(&mut self) -> &mut User {
        let at = self.selected_at();
        &mut self.users[at]
    }

    /// Selects `name`. A name not in the list changes nothing.
    pub fn select(&mut self, name: &str) {
        if let Some(user) = self.users.iter().find(|user| user.name == name) {
            self.selected = user.name.clone();
        }
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
        self.selected = name.to_string();
        true
    }

    /// Whether the selected user could be deleted: only while there is
    /// someone else to select, since someone is always selected.
    pub fn can_delete(&self) -> bool {
        self.users.len() > 1
    }

    /// Removes the selected user from the list and selects the one
    /// before them, or the first if they were first. The user's bubbles
    /// keep their look. Nothing happens to the last user.
    pub fn delete_selected(&mut self) {
        if !self.can_delete() {
            return;
        }
        let at = self.selected_at();
        let user = self.users.remove(at);
        self.retired.retain(|retired| retired.name != user.name);
        self.retired.push(user);
        self.selected = self.users[at.saturating_sub(1)].name.clone();
    }

    /// The library's table of names: every user there has been, with the
    /// current list last so that it wins over a deleted namesake.
    pub fn names(&self) -> Names {
        self.retired
            .iter()
            .chain(&self.users)
            .fold(Names::none(), |names, user| {
                names.name(user.name.clone(), user.look())
            })
    }

    /// What was stored last time, or the defaults. A stored list with no
    /// one in it, or a selection naming no one in it, which older builds
    /// allowed, is corrected: the defaults, or the first user. A user
    /// with no name, which nothing here makes, is left out.
    pub fn load() -> Self {
        match storage() {
            Some(storage) => Self::from_stored(|key| storage.get_item(key).ok().flatten()),
            None => Self::default(),
        }
    }

    /// The settings `read` finds under the keys, with the defaults for
    /// what it does not find or what does not parse. See [`load`](Self::load).
    fn from_stored(read: impl Fn(&str) -> Option<String>) -> Self {
        let mut settings = Self::default();
        if let Some(mut stored) =
            read(USERS_KEY).and_then(|json| serde_json::from_str::<StoredUsers>(&json).ok())
        {
            stored.users.retain(|user| !user.name.trim().is_empty());
            if stored.users.is_empty() {
                // The defaults stand in, so a deleted namesake of theirs
                // would be in both lists.
                let defaults = &settings.users;
                stored
                    .retired
                    .retain(|user| !defaults.iter().any(|default| default.name == user.name));
            } else {
                settings.users = stored.users;
                settings.selected = settings.users[0].name.clone();
                settings.select(stored.selected.as_deref().unwrap_or(USER));
            }
            settings.retired = stored.retired;
        }
        settings.theme = read(THEME_KEY).as_deref().and_then(Theme::parse);
        settings.input_height = read(INPUT_HEIGHT_KEY)
            .and_then(|value| value.parse().ok())
            .filter(|height: &f64| height.is_finite() && *height > 0.0);
        settings.show_names = read(SHOW_NAMES_KEY).is_some_and(|value| value == "true");
        settings
    }

    /// Keeps these for next time. A setting that is unset is removed, so
    /// that it follows the system again.
    pub fn store(&self) {
        let Some(storage) = storage() else { return };
        self.store_with(|key, value| {
            let _ = match value {
                Some(value) => storage.set_item(key, &value),
                None => storage.remove_item(key),
            };
        });
    }

    /// Hands `write` each key with its value, or `None` for a setting
    /// that is unset. See [`store`](Self::store).
    fn store_with(&self, mut write: impl FnMut(&str, Option<String>)) {
        write(
            THEME_KEY,
            self.theme.map(|theme| theme.as_str().to_string()),
        );
        let users = StoredUsers {
            users: self.users.as_slice(),
            retired: self.retired.as_slice(),
            selected: Some(self.selected.as_str()),
        };
        write(USERS_KEY, serde_json::to_string(&users).ok());
        write(
            INPUT_HEIGHT_KEY,
            self.input_height.map(|height| height.to_string()),
        );
        write(SHOW_NAMES_KEY, Some(self.show_names.to_string()));
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
        assert_eq!(settings.selected_user().name, USER);
        assert!(
            !settings.show_names,
            "the bubbles are unnamed to begin with"
        );
        let names = settings.names();
        assert_eq!(names.get(ASSISTANT).unwrap().position, Position::Left);
        assert_eq!(names.get(USER).unwrap().position, Position::Right);
        assert_eq!(
            names.get(USER).unwrap().background.as_deref(),
            Some("#2f855a")
        );
        assert_eq!(
            names.get(USER).unwrap().foreground.as_deref(),
            Some(LIGHT_TEXT)
        );
        assert_eq!(names.get("user"), None, "the names are as spelled");
    }

    #[test]
    fn a_user_is_added_selected_and_edited() {
        let mut settings = Settings::default();
        assert!(!settings.can_add("  "));
        assert!(!settings.can_add("User"));
        assert!(settings.can_add("Alice"));
        assert!(settings.add_user(" Alice "));
        assert!(!settings.add_user("Alice"), "no two Alices");
        let alice = settings.selected_user();
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.side, Side::Left);
        assert_eq!(
            alice.color, PALETTE[2],
            "the third user gets the third color"
        );

        settings.selected_user_mut().side = Side::Center;
        settings.selected_user_mut().color = "#ff8800".to_string();
        let look = settings.names().get("Alice").cloned().unwrap();
        assert_eq!(look.position, Position::Center);
        assert_eq!(look.background.as_deref(), Some("#ff8800"));
        assert_eq!(look.foreground.as_deref(), Some(DARK_TEXT));
    }

    #[test]
    fn a_deleted_user_keeps_their_look_and_hands_over_to_a_neighbor() {
        let mut settings = Settings::default();
        settings.add_user("Alice");
        let before = settings.names().get("Alice").cloned().unwrap();
        assert!(settings.can_delete());
        settings.delete_selected();
        assert_eq!(settings.selected, USER, "the user before Alice");
        assert_eq!(settings.selected_user().name, USER);
        assert_eq!(settings.users.len(), 2);
        assert_eq!(settings.names().get("Alice"), Some(&before));
        assert!(settings.can_add("Alice"), "and can come back");

        // Back with a new look, the old one is forgotten.
        settings.add_user("Alice");
        settings.selected_user_mut().color = "#000000".to_string();
        assert_eq!(settings.retired.len(), 0);
        assert_eq!(
            settings.names().get("Alice").unwrap().background.as_deref(),
            Some("#000000")
        );
    }

    #[test]
    fn deleting_the_first_user_selects_the_next_and_the_last_stays() {
        let mut settings = Settings::default();
        settings.select(ASSISTANT);
        settings.delete_selected();
        assert_eq!(
            settings.selected, USER,
            "the one after, there being none before"
        );
        assert_eq!(settings.users.len(), 1);
        assert!(!settings.can_delete(), "someone has to be left");
        settings.delete_selected();
        assert_eq!(
            settings.users.len(),
            1,
            "deleting the last user does nothing"
        );
        assert_eq!(settings.selected_user().name, USER);
    }

    #[test]
    fn selecting_names_a_user_in_the_list() {
        let mut settings = Settings::default();
        settings.select(ASSISTANT);
        assert_eq!(settings.selected, ASSISTANT);
        settings.select("nobody");
        assert_eq!(
            settings.selected, ASSISTANT,
            "an unknown name changes nothing"
        );
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
            selected: Some(settings.selected.clone()),
        };
        let json = serde_json::to_string(&stored).unwrap();
        assert!(json.contains("\"side\":\"right\""), "{json}");
        let back: StoredUsers = serde_json::from_str(&json).unwrap();
        assert_eq!(back.users, settings.users);
        assert_eq!(back.retired, settings.retired);
        assert_eq!(back.selected.as_deref(), Some(settings.selected.as_str()));

        // What an older build stored, with nobody selected, still reads.
        let older: StoredUsers =
            serde_json::from_str(r#"{"users":[],"retired":[],"selected":null}"#).unwrap();
        assert_eq!(older.selected, None);
    }

    #[test]
    fn sides_are_named_and_placed() {
        for (side, name, position) in [
            (Side::Left, "left", Position::Left),
            (Side::Center, "center", Position::Center),
            (Side::Right, "right", Position::Right),
        ] {
            assert_eq!(side.as_str(), name);
            assert_eq!(side.position(), position);
        }
    }

    #[test]
    fn a_color_that_is_not_hex_gets_no_text_color() {
        let user = User::new("Alice", Side::Left, "var(--alice)");
        let look = user.look();
        assert_eq!(look.background.as_deref(), Some("var(--alice)"));
        assert_eq!(look.foreground, None);
    }

    /// `from_stored` over a fixed set of keys.
    fn stored(entries: &[(&str, &str)]) -> Settings {
        Settings::from_stored(|key| {
            entries
                .iter()
                .find(|(stored, _)| *stored == key)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn nothing_stored_is_the_defaults() {
        assert_eq!(stored(&[]), Settings::default());
    }

    #[test]
    fn the_theme_is_read_when_it_parses() {
        assert_eq!(stored(&[(THEME_KEY, "dark")]).theme, Some(Theme::Dark));
        assert_eq!(stored(&[(THEME_KEY, "light")]).theme, Some(Theme::Light));
        assert_eq!(stored(&[(THEME_KEY, "blue")]).theme, None);
    }

    #[test]
    fn the_input_height_is_read_when_it_is_a_positive_number() {
        assert_eq!(
            stored(&[(INPUT_HEIGHT_KEY, "120.5")]).input_height,
            Some(120.5)
        );
        for bad in ["0", "-3", "NaN", "inf", "tall", ""] {
            assert_eq!(
                stored(&[(INPUT_HEIGHT_KEY, bad)]).input_height,
                None,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn names_are_shown_only_when_stored_as_true() {
        assert!(stored(&[(SHOW_NAMES_KEY, "true")]).show_names);
        for off in ["false", "yes", "1", ""] {
            assert!(!stored(&[(SHOW_NAMES_KEY, off)]).show_names, "{off:?}");
        }
    }

    #[test]
    fn the_stored_users_and_selection_are_read() {
        let json = r##"{"users":[{"name":"Alice","side":"center","color":"#ff8800"},{"name":"Bob","side":"right","color":"#000000"}],"retired":[{"name":"Carol","side":"left","color":"#123456"}],"selected":"Bob"}"##;
        let settings = stored(&[(USERS_KEY, json)]);
        let names: Vec<_> = settings.users.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["Alice", "Bob"]);
        assert_eq!(settings.selected, "Bob");
        assert_eq!(settings.users[0].side, Side::Center);
        assert_eq!(
            settings.retired,
            [User::new("Carol", Side::Left, "#123456")]
        );
    }

    #[test]
    fn a_selection_naming_nobody_or_nobody_at_all_selects_the_first() {
        let json = r##"{"users":[{"name":"Alice","side":"left","color":"#ff8800"},{"name":"Bob","side":"right","color":"#000000"}],"retired":[],"selected":"Zed"}"##;
        assert_eq!(stored(&[(USERS_KEY, json)]).selected, "Alice");
        let older = r##"{"users":[{"name":"Alice","side":"left","color":"#ff8800"},{"name":"Bob","side":"right","color":"#000000"}],"retired":[]}"##;
        assert_eq!(stored(&[(USERS_KEY, older)]).selected, "Alice");
        // Unless the default selection is in the list, which older builds
        // that stored no selection always had.
        let with_user = r##"{"users":[{"name":"Alice","side":"left","color":"#ff8800"},{"name":"User","side":"right","color":"#000000"}],"retired":[],"selected":null}"##;
        assert_eq!(stored(&[(USERS_KEY, with_user)]).selected, USER);
    }

    #[test]
    fn an_empty_or_unreadable_list_is_the_default_list() {
        let empty = r##"{"users":[],"retired":[{"name":"Carol","side":"left","color":"#123456"}],"selected":"Carol"}"##;
        let settings = stored(&[(USERS_KEY, empty)]);
        assert_eq!(settings.users, Settings::default().users);
        assert_eq!(settings.selected, USER);
        assert_eq!(settings.retired.len(), 1, "the retired are kept");
        let unreadable = stored(&[(USERS_KEY, "not json")]);
        assert_eq!(unreadable, Settings::default());
    }

    #[test]
    fn the_defaults_are_taken_out_of_the_retired_and_the_nameless_left_out() {
        // An older build let everyone be deleted. The defaults stand in
        // for the empty list, and are not counted among the retired too.
        let everyone_deleted = r##"{"users":[],"retired":[{"name":"Assistant","side":"left","color":"#2a64c8"},{"name":"User","side":"right","color":"#2f855a"},{"name":"Carol","side":"left","color":"#123456"}],"selected":null}"##;
        let mut settings = stored(&[(USERS_KEY, everyone_deleted)]);
        assert_eq!(settings.users(), Settings::default().users);
        assert_eq!(settings.selected(), USER);
        assert_eq!(
            settings.retired,
            [User::new("Carol", Side::Left, "#123456")]
        );
        settings.add_user("Dave");
        assert_eq!(settings.selected_user().color, PALETTE[3], "the next color");

        // A user with no name could not be sent as.
        let nameless = r##"{"users":[{"name":"","side":"left","color":"#000000"},{"name":"Alice","side":"left","color":"#ff8800"}],"retired":[],"selected":""}"##;
        let settings = stored(&[(USERS_KEY, nameless)]);
        assert_eq!(
            settings.users(),
            [User::new("Alice", Side::Left, "#ff8800")]
        );
        assert_eq!(settings.selected(), "Alice");
        let only_nameless = r##"{"users":[{"name":" ","side":"left","color":"#000000"}],"retired":[],"selected":" "}"##;
        assert_eq!(
            stored(&[(USERS_KEY, only_nameless)]).users,
            Settings::default().users
        );
    }

    #[test]
    fn storing_writes_every_key_and_removes_what_is_unset() {
        let mut written = Vec::new();
        Settings::default().store_with(|key, value| written.push((key.to_string(), value)));
        let keys: Vec<_> = written.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [THEME_KEY, USERS_KEY, INPUT_HEIGHT_KEY, SHOW_NAMES_KEY]
        );
        assert_eq!(written[0].1, None, "no theme chosen");
        assert!(
            written[1]
                .1
                .as_deref()
                .unwrap()
                .contains(r#""selected":"User""#),
            "{:?}",
            written[1]
        );
        assert_eq!(written[2].1, None, "no height dragged");
        assert_eq!(written[3].1.as_deref(), Some("false"), "names off");

        let mut written = Vec::new();
        let settings = Settings {
            theme: Some(Theme::Dark),
            input_height: Some(120.0),
            show_names: true,
            ..Settings::default()
        };
        settings.store_with(|key, value| written.push((key.to_string(), value)));
        assert_eq!(written[0].1.as_deref(), Some("dark"));
        assert_eq!(written[2].1.as_deref(), Some("120"));
        assert_eq!(written[3].1.as_deref(), Some("true"));
    }

    #[test]
    fn settings_survive_a_round_trip_through_storage() {
        let mut settings = Settings {
            theme: Some(Theme::Light),
            input_height: Some(96.5),
            show_names: true,
            ..Settings::default()
        };
        settings.add_user("Alice");
        settings.selected_user_mut().side = Side::Center;
        settings.add_user("Bob");
        settings.delete_selected();
        settings.select(ASSISTANT);

        let mut storage = std::collections::HashMap::new();
        settings.store_with(|key, value| {
            storage.insert(key.to_string(), value.expect("everything is set"));
        });
        let back = Settings::from_stored(|key| storage.get(key).cloned());
        assert_eq!(back, settings);
    }
}
