//! The generator behind `assets/highlight.css`, the crate's code colors.
//!
//! Highlighting is class-based (`hl-keyword`, `hl-string`, ...) so the
//! colors are a stylesheet's business, and that stylesheet is not
//! written by hand. This tool takes one of two-face's embedded themes,
//! has syntect print its rules against the crate's class prefix, and
//! scopes each rule to one side of the light and dark switch, so that
//! two themes can share one file. The shipped file is two runs of it
//! under a short header:
//!
//! ```sh
//! cargo run -p leptos-rich-chat --example highlight_css -- OneHalfLight light
//! cargo run -p leptos-rich-chat --example highlight_css -- OneHalfDark dark
//! ```
//!
//! The first argument is the theme (no arguments lists them), the second
//! which side of the light and dark split the rules go on. Light rules
//! apply when the system prefers light and the document does not force
//! dark, or when the document forces light; dark rules, the reverse.
//! Both sides get the same selector shape so neither wins on
//! specificity alone. Run it for any other theme to change the code
//! colors: turn `RichChatStyle`'s `highlight` off and serve the output
//! instead. See "Code colors" in the README.

use leptos_rich_chat::render::code::CLASS_PREFIX;
use syntect::html::{ClassStyle, css_for_theme_with_class_style};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(name) = args.first() else {
        eprintln!("usage: highlight_css <ThemeName> <light|dark>");
        list();
        std::process::exit(2);
    };
    let dark = args.get(1).is_some_and(|side| side == "dark");
    let Some(theme_name) =
        EmbeddedLazyThemeSet::theme_names()
            .iter()
            .copied()
            .find(|theme: &EmbeddedThemeName| {
                theme.as_name().eq_ignore_ascii_case(name)
                    || format!("{theme:?}").eq_ignore_ascii_case(name)
            })
    else {
        eprintln!("no theme named {name}");
        list();
        std::process::exit(2);
    };
    let themes = two_face::theme::extra();
    let theme = themes.get(theme_name);
    let css = css_for_theme_with_class_style(
        theme,
        ClassStyle::SpacedPrefixed {
            prefix: CLASS_PREFIX,
        },
    )
    .expect("theme renders to CSS");
    print!("{}", scoped(&css, theme_name.as_name(), dark));
}

fn list() {
    eprintln!("themes:");
    for theme in EmbeddedLazyThemeSet::theme_names() {
        eprintln!("  {theme:?}");
    }
}

/// Rewrites syntect's flat rules into the two scoped copies.
fn scoped(css: &str, theme: &str, dark: bool) -> String {
    // Syntect's root block, the theme's own foreground and background,
    // which it puts on a `code` class under the prefix.
    let root = format!(".{CLASS_PREFIX}code");
    let rules: Vec<(String, String)> = css
        .split("}\n")
        .filter_map(|rule| {
            let (selectors, body) = rule.split_once('{')?;
            let selectors = selectors.trim();
            // Skip the comment header and the theme's root block, whose
            // background is the stylesheet's business.
            if selectors.is_empty() || selectors.starts_with("/*") || selectors == root {
                return None;
            }
            let body: Vec<&str> = body
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect();
            Some((selectors.to_string(), body.join("")))
        })
        .collect();

    let (forced, media, system) = if dark {
        (
            "[data-theme=\"dark\"]",
            "dark",
            ":root:not([data-theme=\"light\"])",
        )
    } else {
        (
            "[data-theme=\"light\"]",
            "light",
            ":root:not([data-theme=\"dark\"])",
        )
    };
    let mut out = format!(
        "/* {} of two-face's \"{theme}\", via examples/highlight_css.rs */\n",
        if dark { "dark side" } else { "light side" }
    );
    out.push_str(&block(&rules, forced, ""));
    out.push_str(&format!("@media (prefers-color-scheme: {media}) {{\n"));
    out.push_str(&block(&rules, system, "  "));
    out.push_str("}\n");
    out
}

fn block(rules: &[(String, String)], prefix: &str, indent: &str) -> String {
    let mut out = String::new();
    for (selectors, body) in rules {
        let scoped: Vec<String> = selectors
            .split(',')
            .map(|s| format!("{prefix} {}", s.trim()))
            .collect();
        out.push_str(&format!("{indent}{} {{{body}}}\n", scoped.join(", ")));
    }
    out
}
