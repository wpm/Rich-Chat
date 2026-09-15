//! LaTeX math, rendered to MathML Core by pulldown-latex.
//!
//! Every modern engine (Blink, WebKit, Gecko) renders MathML natively, so
//! no JavaScript or CSS framework is needed. The crate's stylesheet
//! supplies TeX-like fonts and spacing; see the [`style`](crate::style)
//! module.

use pulldown_cmark_escape::escape_html;
use pulldown_latex::config::DisplayMode;
use pulldown_latex::{Parser, RenderConfig, Storage, push_mathml};

/// Renders one LaTeX expression to a `<math>` element.
///
/// Errors never fail the render: pulldown-latex marks the offending part
/// of the expression with a `<merror>` element carrying the message, and
/// the rest still renders, which is what a live preview wants to see. The
/// original source rides along in an `<annotation>` for assistive
/// technology and copy-paste.
pub fn latex_to_mathml(latex: &str, display: bool) -> String {
    let storage = Storage::new();
    let parser = Parser::new(latex, &storage);
    // pulldown-latex copies the annotation into the output verbatim, so it
    // is escaped here; otherwise `\text{</math><img onerror=…>}` would
    // break out of the equation.
    let mut annotation = String::with_capacity(latex.len());
    escape_html(&mut annotation, latex).expect("a String never refuses a write");
    let config = RenderConfig {
        display_mode: if display {
            DisplayMode::Block
        } else {
            DisplayMode::Inline
        },
        annotation: Some(&annotation),
        ..RenderConfig::default()
    };
    let mut mathml = String::new();
    // Writing into a String cannot fail; the io::Result is an artifact of
    // the writer being generic over io::Write.
    push_mathml(&mut mathml, parser, config).expect("a String never refuses a write");
    mathml
}

#[cfg(test)]
mod tests {
    use super::latex_to_mathml;

    #[test]
    fn inline_and_display_modes() {
        let inline = latex_to_mathml("x^2", false);
        assert!(inline.starts_with("<math"), "{inline}");
        assert!(!inline.contains("display=\"block\""), "{inline}");
        let display = latex_to_mathml("x^2", true);
        assert!(display.contains("display=\"block\""), "{display}");
    }

    #[test]
    fn source_is_escaped_in_text_and_annotation() {
        let mathml = latex_to_mathml(r"\text{<b>&}", false);
        assert!(!mathml.contains("<b>"), "{mathml}");
        assert!(mathml.contains("<mtext>&lt;b&gt;&amp;</mtext>"), "{mathml}");
        assert!(
            mathml.contains(
                "<annotation encoding=\"application/x-tex\">\\text{&lt;b&gt;&amp;}</annotation>"
            ),
            "{mathml}"
        );
        let escape = latex_to_mathml(
            r"\text{</annotation></math><img src=x onerror=alert(1)>}",
            false,
        );
        assert!(!escape.contains("<img"), "{escape}");
    }

    #[test]
    fn errors_render_inline_instead_of_failing() {
        let mathml = latex_to_mathml(r"\frac{a}{", true);
        assert!(mathml.contains("<merror"), "{mathml}");
    }
}
