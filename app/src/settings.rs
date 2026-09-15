//! What the controls set, and where it is kept between runs.
//!
//! Settings live in the browser's local storage (the webview's, under
//! Tauri), one key each, so a value that was never chosen is simply
//! absent and the app falls back to the system's or the theme's own.

use leptos_rich_chat::Position;

const THEME_KEY: &str = "rich-chat.theme";
const SIDE_KEY: &str = "rich-chat.side";
const BUBBLE_KEY: &str = "rich-chat.bubble";
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

    /// The library's tint, the background its default kinds give the
    /// user's bubbles in this theme (`--rc-tint-bg`), which the colour
    /// control shows until a colour is chosen.
    pub fn bubble(self) -> &'static str {
        match self {
            Theme::Light => "#ddf4ff",
            Theme::Dark => "#172b45",
        }
    }
}

/// Where a sent message lands across the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Side {
    Left,
    Center,
    #[default]
    Right,
}

impl Side {
    /// The side's name, as stored and as the control's value.
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Center => "center",
            Side::Right => "right",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "left" => Some(Side::Left),
            "center" => Some(Side::Center),
            "right" => Some(Side::Right),
            _ => None,
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

/// Everything the controls set.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Settings {
    /// The chosen theme; `None` follows the system.
    pub theme: Option<Theme>,
    /// Which side sent messages land on.
    pub side: Side,
    /// The bubble background as `#rrggbb`; `None` is the theme's own.
    pub bubble: Option<String>,
    /// The text box's height in CSS pixels, once it has been dragged.
    pub input_height: Option<f64>,
}

impl Settings {
    /// What was stored last time, or the defaults.
    pub fn load() -> Self {
        let Some(storage) = storage() else {
            return Self::default();
        };
        let read = |key: &str| storage.get_item(key).ok().flatten();
        Settings {
            theme: read(THEME_KEY).as_deref().and_then(Theme::parse),
            side: read(SIDE_KEY)
                .as_deref()
                .and_then(Side::parse)
                .unwrap_or_default(),
            bubble: read(BUBBLE_KEY).filter(|value| parse_hex(value).is_some()),
            input_height: read(INPUT_HEIGHT_KEY)
                .and_then(|value| value.parse().ok())
                .filter(|height: &f64| height.is_finite() && *height > 0.0),
        }
    }

    /// Keeps these for next time. A setting that is unset is removed, so
    /// that it follows the system or the theme again.
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
        write(SIDE_KEY, Some(self.side.as_str().to_string()));
        write(BUBBLE_KEY, self.bubble.clone());
        write(
            INPUT_HEIGHT_KEY,
            self.input_height.map(|height| height.to_string()),
        );
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Whether the system prefers a dark colour scheme right now.
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

/// The library's text colour on light backgrounds (`--rc-fg` in the light
/// theme) and on dark ones (the same in the dark theme).
const DARK_TEXT: &str = "#1f2328";
const LIGHT_TEXT: &str = "#e6edf3";

/// The text colour that reads best on `background`: the library's own
/// ink for light backgrounds or its ink for dark ones, whichever has the
/// higher WCAG contrast. `None` when `background` is not `#rrggbb`.
pub fn text_on(background: &str) -> Option<&'static str> {
    let bg = luminance(background)?;
    let contrast = |ink: &str| {
        let ink = luminance(ink).expect("the inks are valid colours");
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

/// `#rrggbb`, which is what a colour input produces, to its channels.
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
    fn hex_colours_parse_and_nothing_else_does() {
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
        assert_eq!(text_on(Theme::Light.bubble()), Some(DARK_TEXT));
        assert_eq!(text_on(Theme::Dark.bubble()), Some(LIGHT_TEXT));
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
        assert_eq!(text_on("not a colour"), None);
    }

    #[test]
    fn luminance_is_wcag() {
        assert_eq!(luminance("#000000"), Some(0.0));
        assert_eq!(luminance("#ffffff"), Some(1.0));
        let red = luminance("#ff0000").unwrap();
        assert!((red - 0.2126).abs() < 1e-9, "{red}");
    }

    #[test]
    fn themes_and_sides_round_trip() {
        for theme in [Theme::Light, Theme::Dark] {
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
            assert_eq!(theme.toggled().toggled(), theme);
            assert_ne!(theme.toggled(), theme);
        }
        for side in [Side::Left, Side::Center, Side::Right] {
            assert_eq!(Side::parse(side.as_str()), Some(side));
        }
        assert_eq!(Side::Center.position(), Position::Center);
        assert_eq!(Theme::parse("blue"), None);
        assert_eq!(Side::parse("middle"), None);
        assert_eq!(Settings::default().side, Side::Right);
    }
}
