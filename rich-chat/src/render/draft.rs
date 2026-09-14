//! Progressive completion of text that is still being written.
//!
//! A draft in the composer, or a message still streaming in from a model,
//! is usually cut off in the middle of something: an open `$$` block, an
//! unfinished `` `code span` ``. CommonMark renders such fragments as
//! literal text, which makes a live preview flicker between "math" and
//! "a paragraph with dollar signs in it" on every keystroke.
//! [`complete_draft`] appends the closing delimiters the text is missing
//! so the fragment renders as what it is about to become.
//!
//! Fenced code blocks need no help: CommonMark already runs an unclosed
//! fence to the end of the document.

use std::borrow::Cow;

/// Closes the inline constructs left open at the end of `text`.
///
/// Follows pulldown-cmark's own delimiter rules: a `$` opens inline math
/// only when the next character is not whitespace (nor, here, a digit,
/// so prices stay prose) and closes it only when the previous one is not; `$$` opens and closes display math anywhere;
/// a run of backticks is closed by a run of the same length; whichever
/// opener comes first wins, so a `$` inside a code span is text and a
/// backtick inside math is math. Inline constructs cannot cross a blank
/// line or enter a fenced code block, and neither does the scan.
///
/// Returns the input unchanged (without allocating) when nothing is open.
pub fn complete_draft(text: &str) -> Cow<'_, str> {
    let mut fence: Option<(u8, usize)> = None;
    let mut inline_code: Option<usize> = None;
    let mut inline_math = false;
    let mut display_math = false;
    // Unclosed `{` inside the open math span. pulldown-cmark only pairs
    // math delimiters at the same brace depth, so a `$$\frac{a}{b` has
    // to be closed as `}$$`.
    let mut braces = 0usize;

    for line in text.split_inclusive('\n') {
        if let Some((marker, len)) = fence {
            if closes_fence(line, marker, len) {
                fence = None;
            }
            continue;
        }
        if let Some(opened) = opens_fence(line) {
            fence = Some(opened);
            inline_code = None;
            inline_math = false;
            display_math = false;
            braces = 0;
            continue;
        }
        if line.trim().is_empty() {
            inline_code = None;
            inline_math = false;
            display_math = false;
            braces = 0;
            continue;
        }

        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'`' => {
                    let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                    match inline_code {
                        Some(open) if open == run => inline_code = None,
                        Some(_) => {}
                        None if !inline_math && !display_math => inline_code = Some(run),
                        None => {}
                    }
                    i += run;
                }
                b'{' if inline_math || display_math => {
                    braces += 1;
                    i += 1;
                }
                b'}' if inline_math || display_math => {
                    braces = braces.saturating_sub(1);
                    i += 1;
                }
                b'$' if inline_code.is_none() => {
                    let double = bytes.get(i + 1) == Some(&b'$');
                    if display_math {
                        if double && braces == 0 {
                            display_math = false;
                            i += 2;
                        } else {
                            i += 1;
                        }
                    } else if inline_math {
                        let can_close = i > 0 && !bytes[i - 1].is_ascii_whitespace();
                        if can_close && braces == 0 {
                            inline_math = false;
                        }
                        i += 1;
                    } else if double {
                        display_math = true;
                        braces = 0;
                        i += 2;
                    } else {
                        // Stricter than CommonMark's rule on purpose: a
                        // `$` before a digit is almost always a price, and
                        // closing it would turn "$5 and $10" into math on
                        // every keystroke until the paragraph is finished.
                        let can_open = bytes
                            .get(i + 1)
                            .is_some_and(|b| !b.is_ascii_whitespace() && !b.is_ascii_digit());
                        if can_open {
                            inline_math = true;
                            braces = 0;
                        }
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
    }

    let mut closer = String::new();
    if display_math || inline_math {
        closer.extend(std::iter::repeat_n('}', braces));
    }
    if display_math {
        closer.push_str("$$");
    } else if inline_math {
        closer.push('$');
    }
    if let Some(run) = inline_code {
        closer.extend(std::iter::repeat_n('`', run));
    }
    if closer.is_empty() {
        Cow::Borrowed(text)
    } else {
        let mut completed = String::with_capacity(text.len() + closer.len());
        completed.push_str(text);
        completed.push_str(&closer);
        Cow::Owned(completed)
    }
}

/// A fence opener: up to three spaces of indentation, then at least three
/// backticks or tildes. A backtick fence's info string may not contain a
/// backtick.
fn opens_fence(line: &str) -> Option<(u8, usize)> {
    let stripped = line.trim_start_matches(' ');
    if line.len() - stripped.len() > 3 {
        return None;
    }
    let marker = *stripped.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let len = stripped.bytes().take_while(|&b| b == marker).count();
    if len < 3 {
        return None;
    }
    if marker == b'`' && stripped[len..].contains('`') {
        return None;
    }
    Some((marker, len))
}

/// A fence closer: the same marker, at least as long, and nothing but
/// whitespace after it.
fn closes_fence(line: &str, marker: u8, len: usize) -> bool {
    let stripped = line.trim_start_matches(' ');
    if line.len() - stripped.len() > 3 {
        return false;
    }
    let run = stripped.bytes().take_while(|&b| b == marker).count();
    run >= len && stripped[run..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::complete_draft;

    #[test]
    fn complete_text_is_untouched() {
        for text in [
            "",
            "plain",
            "inline $x$ math",
            "display $$x$$ math",
            "a `code` span",
            "```rust\nlet x = 1;",
            "costs $5 and $10",
            "trailing $",
            "escaped \\$x",
        ] {
            assert!(
                matches!(complete_draft(text), std::borrow::Cow::Borrowed(_)),
                "{text:?} should be left alone"
            );
        }
    }

    #[test]
    fn open_inline_math_is_closed() {
        assert_eq!(complete_draft("so $x^2 + y"), "so $x^2 + y$");
    }

    #[test]
    fn open_display_math_is_closed() {
        assert_eq!(complete_draft("$$\n\\int_0^1 x"), "$$\n\\int_0^1 x$$");
        assert_eq!(complete_draft("see $$\\frac{a}{b}"), "see $$\\frac{a}{b}$$");
    }

    #[test]
    fn open_braces_are_closed_before_the_delimiter() {
        assert_eq!(
            complete_draft("$$\\frac{n(n+1)}{2"),
            "$$\\frac{n(n+1)}{2}$$"
        );
        assert_eq!(complete_draft("$\\sqrt{\\frac{a"), "$\\sqrt{\\frac{a}}$");
        assert_eq!(complete_draft("$\\{a"), "$\\{a$");
    }

    #[test]
    fn open_code_span_is_closed() {
        assert_eq!(complete_draft("run `cargo tes"), "run `cargo tes`");
        assert_eq!(complete_draft("run ``a`b"), "run ``a`b``");
    }

    #[test]
    fn dollars_inside_code_spans_are_text() {
        assert!(matches!(
            complete_draft("`$x`"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(complete_draft("`$x` and $y"), "`$x` and $y$");
    }

    #[test]
    fn backticks_inside_math_are_math() {
        assert_eq!(complete_draft("$a`b"), "$a`b$");
    }

    #[test]
    fn a_blank_line_ends_inline_constructs() {
        assert!(matches!(
            complete_draft("$open\n\nnew paragraph"),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn fenced_code_is_opaque() {
        assert!(matches!(
            complete_draft("```\n$x\n`y\n"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert!(matches!(
            complete_draft("~~~\n$$\n~~~\n"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(complete_draft("```\n$x\n```\n$y"), "```\n$x\n```\n$y$");
    }

    #[test]
    fn a_dollar_before_whitespace_or_a_digit_does_not_open() {
        assert!(matches!(
            complete_draft("$ x"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert!(matches!(
            complete_draft("$5 and then"),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    #[test]
    fn closing_needs_a_non_space_before() {
        assert_eq!(complete_draft("$x $ y"), "$x $ y$");
    }
}
