//! Prints the highlighting rules for one of two-face's embedded themes,
//! in the form `assets/highlight.css` uses.
//!
//! ```sh
//! cargo run -p rich-chat --example theme_css -- OneHalfLight light
//! cargo run -p rich-chat --example theme_css -- OneHalfDark dark
//! ```
//!
//! The first argument is the theme, the second which side of the light
//! and dark split the rules go on. Light rules apply when the system
//! prefers light and the document does not force dark, or when the
//! document forces light; dark rules, the reverse. Both sides get the
//! same selector shape so neither wins on specificity alone.

use syntect::html::{ClassStyle, css_for_theme_with_class_style};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(name) = args.first() else {
        eprintln!("usage: theme_css <ThemeName> <light|dark>");
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
            prefix: rich_chat::render::code::CLASS_PREFIX,
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
    let rules: Vec<(String, String)> = css
        .split("}\n")
        .filter_map(|rule| {
            let (selectors, body) = rule.split_once('{')?;
            let selectors = selectors.trim();
            // Skip the comment header and the theme's root block, whose
            // background is the stylesheet's business.
            if selectors.is_empty() || selectors.starts_with("/*") || selectors == ".rc-code" {
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
        "/* {} of two-face's \"{theme}\", via examples/theme_css.rs */\n",
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
