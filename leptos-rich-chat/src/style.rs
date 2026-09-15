//! The stylesheet and fonts.
//!
//! Everything the components need is in [`STYLESHEET`]: layout, the light
//! and dark palettes, code highlighting colours, and MathML spacing. The
//! [`RichChatStyle`](crate::RichChatStyle) component injects it, or a
//! consumer can serve it as a file. Colours are CSS custom properties
//! prefixed `--rc-`, set on `.rc-chat`, `.rc-rich`, and `.rc-composer`,
//! so a host can override any of them.
//!
//! Dark mode follows `prefers-color-scheme` unless the document sets
//! `data-theme="light"` or `data-theme="dark"` on its root element, in
//! which case that wins.
//!
//! Equations are set in Latin Modern Math, the TeX font, so they look the
//! same everywhere. With the `bundled-fonts` feature (the default) the
//! fonts are embedded and [`font_faces`] returns `@font-face` rules that
//! carry them as data URIs; without it, [`font_faces_from`] builds the
//! same rules for fonts served from a directory, and [`FONT_FILES`] has
//! the bytes to serve.

/// The layout, palette, highlighting, and math styles, without fonts.
pub const STYLESHEET: &str = concat!(
    include_str!("../assets/rich-chat.css"),
    "\n",
    include_str!("../assets/math.css"),
    "\n",
    include_str!("../assets/highlight.css"),
);

/// The math fonts by file name, for a consumer that serves them itself.
/// Latin Modern, under the GUST Font License.
pub const FONT_FILES: [(&str, &[u8]); 4] = [
    (
        "latinmodern-math.woff2",
        include_bytes!("../assets/fonts/latinmodern-math.woff2"),
    ),
    (
        "lmroman12-regular.woff2",
        include_bytes!("../assets/fonts/lmroman12-regular.woff2"),
    ),
    (
        "lmroman12-bold.woff2",
        include_bytes!("../assets/fonts/lmroman12-bold.woff2"),
    ),
    (
        "lmroman12-italic.woff2",
        include_bytes!("../assets/fonts/lmroman12-italic.woff2"),
    ),
];

/// `@font-face` rules that load the math fonts from `base_url`, a
/// directory URL with or without a trailing slash.
pub fn font_faces_from(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    font_face_rules(|file| format!("{base}/{file}"))
}

/// `@font-face` rules with the fonts embedded as data URIs. Built once,
/// on first use. Empty without the `bundled-fonts` feature.
pub fn font_faces() -> &'static str {
    #[cfg(feature = "bundled-fonts")]
    {
        static RULES: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            font_face_rules(|file| {
                let bytes = FONT_FILES
                    .iter()
                    .find(|(name, _)| *name == file)
                    .map(|(_, bytes)| *bytes)
                    .unwrap_or_default();
                format!("data:font/woff2;base64,{}", base64(bytes))
            })
        });
        &RULES
    }
    #[cfg(not(feature = "bundled-fonts"))]
    {
        ""
    }
}

fn font_face_rules(url: impl Fn(&str) -> String) -> String {
    let faces = [
        (
            "Latin Modern Math",
            "latinmodern-math.woff2",
            "normal",
            "normal",
        ),
        (
            "Latin Modern Roman",
            "lmroman12-regular.woff2",
            "normal",
            "normal",
        ),
        (
            "Latin Modern Roman",
            "lmroman12-bold.woff2",
            "bold",
            "normal",
        ),
        (
            "Latin Modern Roman",
            "lmroman12-italic.woff2",
            "normal",
            "italic",
        ),
    ];
    let mut css = String::new();
    for (family, file, weight, style) in faces {
        css.push_str(&format!(
            "@font-face{{font-family:\"{family}\";font-weight:{weight};font-style:{style};\
             font-display:swap;src:local(\"{family}\"),url(\"{}\") format(\"woff2\");}}\n",
            url(file)
        ));
    }
    css
}

#[cfg(feature = "bundled-fonts")]
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &b)| acc | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(TABLE[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn embedded_font_faces_cover_every_font() {
        let css = font_faces();
        assert_eq!(css.matches("@font-face").count(), 4);
        assert_eq!(css.matches("data:font/woff2;base64,").count(), 4);
        assert!(css.contains("\"Latin Modern Math\""));
    }

    #[test]
    fn served_font_faces_point_at_the_directory() {
        let css = font_faces_from("/fonts/");
        assert!(
            css.contains("url(\"/fonts/latinmodern-math.woff2\")"),
            "{css}"
        );
        assert_eq!(css.matches("@font-face").count(), 4);
    }

    #[test]
    fn the_stylesheet_has_all_three_parts() {
        assert!(STYLESHEET.contains(".rc-chat"));
        assert!(STYLESHEET.contains(".rc-rich math"));
        assert!(STYLESHEET.contains(".rc-keyword"));
    }
}
