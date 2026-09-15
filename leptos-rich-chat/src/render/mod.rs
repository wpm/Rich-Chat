//! Markdown to HTML, as a pure function.
//!
//! [`render_blocks`] parses a message with pulldown-cmark and returns its
//! top-level blocks, each rendered to HTML and tagged with a key that is
//! stable across re-renders. A block whose content did not change keeps
//! its key, so a view keyed on it (the [`RichText`](crate::RichText)
//! component) leaves that block's DOM alone while a draft is typed or a
//! streamed message grows. [`render_html`] is the same pipeline flattened
//! into one string.
//!
//! Along the way the event stream is rewritten:
//!
//! - `$…$` and `$$…$$` become MathML through [`math`].
//! - Fenced and indented code blocks become escaped, highlighted HTML
//!   through [`code`].
//! - Raw HTML, inline or block, becomes text. Links keep only `http`,
//!   `https`, and `mailto` destinations and open in a new context;
//!   images keep only `http`, `https`, and `data:image` sources. A
//!   message therefore cannot inject markup or scripts.
//! - Footnotes are numbered across the whole message and collected into a
//!   list at its end, GitHub style.
//!
//! Everything here is plain Rust with no DOM or reactive dependency, so it
//! runs and is tested natively.

pub mod code;
pub mod draft;
pub mod math;

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, LinkType, Options, Parser, Tag, TagEnd, TextMergeStream,
};
use pulldown_cmark_escape::{escape_href, escape_html};

pub use code::{Highlighted, WARM_LANGUAGES, highlight, preload, warm};
pub use draft::complete_draft;
pub use math::latex_to_mathml;

/// What to render and how. The default is the whole feature set with
/// nothing speculative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderOptions {
    /// Render `$…$` and `$$…$$` as math. Off, dollar signs are text.
    pub math: bool,
    /// Syntax-highlight fenced code blocks. Off, code is plain. Has no
    /// effect without the crate's `highlight` feature.
    pub highlight: bool,
    /// Render `![alt](src)` as an image. Off, only the alt text shows.
    /// Loading a remote image reveals the reader's address to its host,
    /// which a privacy-minded consumer may prefer to avoid.
    pub images: bool,
    /// Turn straight quotes, `--`, and `...` into typographic characters.
    /// Off by default because it also rewrites prose that quotes code.
    pub smart_punctuation: bool,
    /// Treat the text as unfinished: close constructs left open at its
    /// end so a live preview or a streaming message renders as what it is
    /// becoming. See [`complete_draft`].
    pub draft: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            math: true,
            highlight: true,
            images: true,
            smart_punctuation: false,
            draft: false,
        }
    }
}

impl RenderOptions {
    /// The defaults with [`draft`](Self::draft) set.
    pub fn draft() -> Self {
        Self {
            draft: true,
            ..Self::default()
        }
    }
}

/// Identifies a block across re-renders of the same message: the hash of
/// its rendered content, and which occurrence of that content it is, so
/// two identical paragraphs still get distinct keys.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockKey {
    /// Hash of the block's rendered form.
    pub hash: u64,
    /// Zero-based count of earlier blocks in the message with the same hash.
    pub nth: u32,
}

/// One top-level block of a rendered message.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// Any block-level construct: a paragraph, list, table, quote,
    /// heading, rule, or the footnote list. Sanitized HTML.
    Html(String),
    /// A fenced or indented code block.
    Code {
        /// The language's display name, when the fence's token resolved.
        language: Option<String>,
        /// The code as written, for copying.
        source: String,
        /// The code as escaped, highlighted HTML — the contents of a
        /// `<code>` element.
        html: String,
    },
    /// A paragraph consisting solely of one display equation.
    Math {
        /// The LaTeX as written.
        source: String,
        /// The rendered `<math display="block">` element.
        mathml: String,
    },
}

/// A keyed block. See [`render_blocks`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Stable identity for keyed rendering.
    pub key: BlockKey,
    /// The rendered content.
    pub kind: BlockKind,
}

impl Block {
    /// The block as a self-contained HTML fragment. Code blocks become a
    /// `<pre class="rc-code">`, equations a `<div class="rc-math-block">`.
    pub fn to_html(&self) -> String {
        match &self.kind {
            BlockKind::Html(html) => html.clone(),
            BlockKind::Code { language, html, .. } => {
                let mut out = String::from("<pre class=\"rc-code\"");
                if let Some(language) = language {
                    out.push_str(" data-language=\"");
                    escape_html(&mut out, language).expect("String write");
                    out.push('"');
                }
                out.push_str("><code>");
                out.push_str(html);
                out.push_str("</code></pre>\n");
                out
            }
            BlockKind::Math { mathml, .. } => {
                format!("<div class=\"rc-math-block\">{mathml}</div>\n")
            }
        }
    }
}

/// Renders `markdown` to its top-level blocks.
pub fn render_blocks(markdown: &str, options: &RenderOptions) -> Vec<Block> {
    let text = if options.draft {
        complete_draft(markdown)
    } else {
        Cow::Borrowed(markdown)
    };
    let mut renderer = Renderer::new(&text, options);
    let kinds = renderer.render();
    let mut seen: HashMap<u64, u32> = HashMap::new();
    kinds
        .into_iter()
        .map(|kind| {
            let hash = hash_of(&kind);
            let nth = seen.entry(hash).or_insert(0);
            let key = BlockKey { hash, nth: *nth };
            *nth += 1;
            Block { key, kind }
        })
        .collect()
}

/// Renders `markdown` to one HTML string.
pub fn render_html(markdown: &str, options: &RenderOptions) -> String {
    render_blocks(markdown, options)
        .iter()
        .map(Block::to_html)
        .collect()
}

fn hash_of(kind: &BlockKind) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    hasher.finish()
}

fn parser_options(options: &RenderOptions) -> Options {
    let mut flags = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    if options.math {
        flags |= Options::ENABLE_MATH;
    }
    if options.smart_punctuation {
        flags |= Options::ENABLE_SMART_PUNCTUATION;
    }
    flags
}

/// Rendering state for one message.
struct Renderer<'a, 'o> {
    options: &'o RenderOptions,
    /// Every top-level event group in source order.
    groups: Vec<Vec<Event<'a>>>,
    /// Footnote label → number, in order of first reference.
    footnotes: HashMap<CowStr<'a>, usize>,
    /// How many references each footnote has had, for unique anchor ids.
    references: HashMap<usize, usize>,
    /// Distinguishes this message's footnote anchors from another's on the
    /// same page.
    prefix: String,
}

impl<'a, 'o> Renderer<'a, 'o> {
    fn new(text: &'a str, options: &'o RenderOptions) -> Self {
        // Merged, so a URL that pulldown-cmark split around a `_` or `*`
        // reaches the autolinker whole.
        let parser = TextMergeStream::new(Parser::new_ext(text, parser_options(options)));
        let mut groups = Vec::new();
        let mut current = Vec::new();
        let mut depth = 0usize;
        for event in parser {
            match &event {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth = depth.saturating_sub(1),
                _ => {}
            }
            current.push(event);
            if depth == 0 {
                groups.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            groups.push(current);
        }

        let mut footnotes = HashMap::new();
        for event in groups.iter().flatten() {
            if let Event::FootnoteReference(label) = event {
                let next = footnotes.len() + 1;
                footnotes.entry(label.clone()).or_insert(next);
            }
        }
        for event in groups.iter().flatten() {
            if let Event::Start(Tag::FootnoteDefinition(label)) = event {
                let next = footnotes.len() + 1;
                footnotes.entry(label.clone()).or_insert(next);
            }
        }

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let prefix = format!("rc-{:08x}", hasher.finish() as u32);

        Self {
            options,
            groups,
            footnotes,
            references: HashMap::new(),
            prefix,
        }
    }

    fn render(&mut self) -> Vec<BlockKind> {
        let groups = std::mem::take(&mut self.groups);
        let mut blocks = Vec::with_capacity(groups.len());
        let mut definitions: Vec<(usize, String)> = Vec::new();
        for group in groups {
            match group.first() {
                Some(Event::Start(Tag::CodeBlock(kind))) => {
                    let token = fence_token(kind);
                    let source = code_text(&group[1..group.len() - 1]);
                    let Highlighted { language, html } =
                        self.highlight_code(token.as_deref(), &source);
                    blocks.push(BlockKind::Code {
                        language,
                        source,
                        html,
                    });
                }
                Some(Event::Start(Tag::FootnoteDefinition(label))) => {
                    let number = self.footnotes[label];
                    let inner = self.html_of(&group[1..group.len() - 1]);
                    definitions.push((number, inner));
                }
                Some(Event::Start(Tag::Paragraph)) if self.options.math => {
                    if let Some(source) = lone_display_math(&group) {
                        let mathml = latex_to_mathml(&source, true);
                        blocks.push(BlockKind::Math { source, mathml });
                    } else {
                        blocks.push(BlockKind::Html(self.html_of(&group)));
                    }
                }
                Some(Event::Start(Tag::Table(_))) => {
                    let inner = self.html_of(&group);
                    blocks.push(BlockKind::Html(format!(
                        "<div class=\"rc-table-wrap\">{inner}</div>\n"
                    )));
                }
                Some(_) => blocks.push(BlockKind::Html(self.html_of(&group))),
                None => {}
            }
        }
        if !definitions.is_empty() {
            definitions.sort_by_key(|(number, _)| *number);
            blocks.push(BlockKind::Html(self.footnote_list(definitions)));
        }
        blocks
    }

    fn highlight_code(&self, token: Option<&str>, source: &str) -> Highlighted {
        if self.options.highlight {
            highlight(token, source)
        } else {
            Highlighted {
                language: None,
                html: escaped(source),
            }
        }
    }

    /// Renders a slice of events to HTML after rewriting them.
    fn html_of(&mut self, events: &[Event<'a>]) -> String {
        let rewritten = self.rewrite(events);
        let mut html = String::new();
        pulldown_cmark::html::push_html(&mut html, rewritten.into_iter());
        html
    }

    fn rewrite(&mut self, events: &[Event<'a>]) -> Vec<Event<'a>> {
        let mut out = Vec::with_capacity(events.len());
        // For each open link or image: whether it produced an element
        // whose closing tag still has to be written.
        let mut links: Vec<bool> = Vec::new();
        let mut images: Vec<bool> = Vec::new();
        let mut i = 0;
        while i < events.len() {
            match &events[i] {
                Event::Start(Tag::CodeBlock(kind)) => {
                    let end = events[i..]
                        .iter()
                        .position(|e| matches!(e, Event::End(TagEnd::CodeBlock)))
                        .map_or(events.len(), |offset| i + offset);
                    let token = fence_token(kind);
                    let source = code_text(&events[i + 1..end]);
                    let Highlighted { language, html } =
                        self.highlight_code(token.as_deref(), &source);
                    let mut pre = String::from("<pre class=\"rc-code\"");
                    if let Some(language) = &language {
                        pre.push_str(" data-language=\"");
                        escape_html(&mut pre, language).expect("String write");
                        pre.push('"');
                    }
                    pre.push_str("><code>");
                    pre.push_str(&html);
                    pre.push_str("</code></pre>\n");
                    out.push(Event::Html(pre.into()));
                    i = end + 1;
                    continue;
                }
                Event::Start(Tag::HtmlBlock) => out.push(Event::Start(Tag::Paragraph)),
                Event::End(TagEnd::HtmlBlock) => out.push(Event::End(TagEnd::Paragraph)),
                Event::Html(raw) | Event::InlineHtml(raw) => out.push(Event::Text(raw.clone())),
                Event::InlineMath(latex) => {
                    out.push(Event::Html(latex_to_mathml(latex, false).into()));
                }
                Event::DisplayMath(latex) => {
                    let mathml = latex_to_mathml(latex, true);
                    out.push(Event::Html(
                        format!("<span class=\"rc-math-display\">{mathml}</span>").into(),
                    ));
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    // An `<addr@host>` autolink carries the bare address;
                    // the scheme is the writer's job, as in pulldown-cmark's
                    // own HTML output.
                    let href: Cow<str> = if *link_type == LinkType::Email {
                        format!("mailto:{dest_url}").into()
                    } else {
                        Cow::Borrowed(dest_url)
                    };
                    if is_safe_link(&href) {
                        let mut anchor = String::from("<a href=\"");
                        escape_href(&mut anchor, &href).expect("String write");
                        anchor.push('"');
                        if !title.is_empty() {
                            anchor.push_str(" title=\"");
                            escape_html(&mut anchor, title).expect("String write");
                            anchor.push('"');
                        }
                        anchor.push_str(" target=\"_blank\" rel=\"noopener noreferrer\">");
                        out.push(Event::Html(anchor.into()));
                        links.push(true);
                    } else {
                        links.push(false);
                    }
                }
                Event::End(TagEnd::Link) => {
                    if links.pop().unwrap_or(false) {
                        out.push(Event::Html("</a>".into()));
                    }
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    let allowed = self.options.images && is_safe_image(dest_url);
                    images.push(allowed);
                    if allowed {
                        out.push(events[i].clone());
                    } else {
                        out.push(Event::Html("<span class=\"rc-image-alt\">".into()));
                    }
                }
                Event::End(TagEnd::Image) => {
                    if images.pop().unwrap_or(true) {
                        out.push(events[i].clone());
                    } else {
                        out.push(Event::Html("</span>".into()));
                    }
                }
                Event::Text(text) if links.is_empty() && images.is_empty() => {
                    autolink(text, &mut out);
                }
                Event::FootnoteReference(label) => {
                    let number = self.footnotes[label];
                    let seen = self.references.entry(number).or_insert(0);
                    let id = if *seen == 0 {
                        format!("{}-fnref-{number}", self.prefix)
                    } else {
                        format!("{}-fnref-{number}-{seen}", self.prefix)
                    };
                    *seen += 1;
                    out.push(Event::Html(
                        format!(
                            "<sup class=\"rc-footnote-ref\"><a href=\"#{p}-fn-{number}\" \
                             id=\"{id}\">{number}</a></sup>",
                            p = self.prefix
                        )
                        .into(),
                    ));
                }
                other => out.push(other.clone()),
            }
            i += 1;
        }
        out
    }

    fn footnote_list(&self, definitions: Vec<(usize, String)>) -> String {
        let mut html = String::from("<section class=\"rc-footnotes\"><ol>\n");
        for (number, body) in definitions {
            let backref = format!(
                "<a href=\"#{p}-fnref-{number}\" class=\"rc-footnote-backref\" \
                 aria-label=\"Back to reference {number}\">\u{21a9}</a>",
                p = self.prefix
            );
            html.push_str(&format!("<li id=\"{p}-fn-{number}\">", p = self.prefix));
            let trimmed = body.trim_end();
            if let Some(paragraph) = trimmed.strip_suffix("</p>") {
                html.push_str(paragraph);
                html.push(' ');
                html.push_str(&backref);
                html.push_str("</p>\n");
            } else {
                html.push_str(trimmed);
                html.push_str(&backref);
                html.push('\n');
            }
            html.push_str("</li>\n");
        }
        html.push_str("</ol></section>\n");
        html
    }
}

/// Splits `text` around bare `http://` and `https://` URLs, emitting each
/// as an anchor, in the manner of GitHub's autolink extension: a URL runs
/// to the next whitespace or `<`, and trailing punctuation, or a `)` with
/// no matching `(` inside the URL, is left to the prose.
fn autolink<'a>(text: &CowStr<'a>, out: &mut Vec<Event<'a>>) {
    let mut rest: &str = text;
    let mut consumed = 0usize;
    let mut plain_start = 0usize;
    while let Some(found) = find_url_start(rest) {
        let start = consumed + found;
        let after = &text[start..];
        let mut end = after
            .find(|c: char| c.is_whitespace() || c == '<')
            .unwrap_or(after.len());
        // Trailing punctuation belongs to the sentence, and a closing
        // parenthesis only to the URL if it opened one.
        loop {
            let url = &after[..end];
            let last = url.chars().last();
            let trim = match last {
                Some('.' | ',' | ':' | ';' | '!' | '?' | '\'' | '"' | '*' | '_' | '~') => true,
                Some(')') => url.matches('(').count() < url.matches(')').count(),
                _ => false,
            };
            if trim {
                end -= last.map_or(0, char::len_utf8);
            } else {
                break;
            }
        }
        let url = &after[..end];
        if url.len() <= "https://".len() {
            consumed = start + 1;
            rest = &text[consumed..];
            continue;
        }
        if start > plain_start {
            out.push(Event::Text(text[plain_start..start].to_string().into()));
        }
        let mut anchor = String::from("<a href=\"");
        escape_href(&mut anchor, url).expect("String write");
        anchor.push_str("\" target=\"_blank\" rel=\"noopener noreferrer\">");
        out.push(Event::Html(anchor.into()));
        out.push(Event::Text(url.to_string().into()));
        out.push(Event::Html("</a>".into()));
        plain_start = start + end;
        consumed = plain_start;
        rest = &text[consumed..];
    }
    if plain_start == 0 {
        out.push(Event::Text(text.clone()));
    } else if plain_start < text.len() {
        out.push(Event::Text(text[plain_start..].to_string().into()));
    }
}

/// The offset of the next `http://` or `https://` that begins a word.
fn find_url_start(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find("http") {
        let at = from + found;
        let candidate = &text[at..];
        let scheme_ok = candidate.starts_with("http://") || candidate.starts_with("https://");
        let boundary = at == 0
            || text[..at]
                .chars()
                .last()
                .is_some_and(|c| c.is_whitespace() || matches!(c, '(' | '[' | '{' | '\'' | '"'));
        if scheme_ok && boundary {
            return Some(at);
        }
        from = at + 4;
    }
    None
}

/// The first word of a fence's info string.
fn fence_token(kind: &CodeBlockKind<'_>) -> Option<String> {
    match kind {
        CodeBlockKind::Fenced(info) => info
            .split_whitespace()
            .next()
            .map(|token| {
                token
                    .trim_matches(|c| c == '{' || c == '}' || c == '.')
                    .to_string()
            })
            .filter(|token| !token.is_empty()),
        CodeBlockKind::Indented => None,
    }
}

/// The text of a code block's events, without the block's own trailing
/// newline.
fn code_text(events: &[Event<'_>]) -> String {
    let mut source = String::new();
    for event in events {
        if let Event::Text(text) = event {
            source.push_str(text);
        }
    }
    if source.ends_with('\n') {
        source.pop();
    }
    source
}

/// The LaTeX of a paragraph that is nothing but one display equation.
fn lone_display_math(group: &[Event<'_>]) -> Option<String> {
    let inner = group.get(1..group.len().checked_sub(1)?)?;
    let mut latex = None;
    for event in inner {
        match event {
            Event::DisplayMath(source) if latex.is_none() => latex = Some(source.to_string()),
            Event::Text(text) if text.trim().is_empty() => {}
            Event::SoftBreak | Event::HardBreak => {}
            _ => return None,
        }
    }
    latex
}

fn scheme(url: &str) -> Option<&str> {
    let end = url.find(':')?;
    let candidate = &url[..end];
    let mut chars = candidate.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(candidate)
}

fn is_safe_link(url: &str) -> bool {
    matches!(
        scheme(url).map(str::to_ascii_lowercase).as_deref(),
        Some("http" | "https" | "mailto")
    )
}

fn is_safe_image(url: &str) -> bool {
    match scheme(url).map(str::to_ascii_lowercase).as_deref() {
        Some("http" | "https") => true,
        Some("data") => url[5..].to_ascii_lowercase().starts_with("image/"),
        _ => false,
    }
}

fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    escape_html(&mut out, text).expect("String write");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(markdown: &str) -> String {
        render_html(markdown, &RenderOptions::default())
    }

    fn blocks(markdown: &str) -> Vec<Block> {
        render_blocks(markdown, &RenderOptions::default())
    }

    #[test]
    fn paragraphs_and_inline_markup() {
        assert_eq!(
            html("Hello *world* and **bold** and ~~gone~~ and `code`."),
            "<p>Hello <em>world</em> and <strong>bold</strong> and <del>gone</del> and \
             <code>code</code>.</p>\n"
        );
    }

    #[test]
    fn every_top_level_block_is_its_own_entry() {
        let out = blocks("# Title\n\nParagraph.\n\n- a\n- b\n\n---\n");
        assert_eq!(out.len(), 4, "{out:#?}");
        assert!(matches!(&out[0].kind, BlockKind::Html(h) if h.starts_with("<h1>")));
        assert!(matches!(&out[3].kind, BlockKind::Html(h) if h.starts_with("<hr")));
    }

    #[test]
    fn keys_survive_edits_elsewhere() {
        let before = blocks("First.\n\nSecond.\n\nThird.");
        let after = blocks("First!\n\nSecond.\n\nThird.");
        assert_ne!(before[0].key, after[0].key);
        assert_eq!(before[1].key, after[1].key);
        assert_eq!(before[2].key, after[2].key);
    }

    #[test]
    fn identical_blocks_get_distinct_keys() {
        let out = blocks("Same.\n\nSame.");
        assert_eq!(out[0].key.hash, out[1].key.hash);
        assert_ne!(out[0].key, out[1].key);
    }

    #[test]
    fn raw_html_is_text() {
        let out = html("before <script>alert(1)</script> after");
        assert!(!out.contains("<script>"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
        let block = html("<div onclick=\"x()\">\nhi\n</div>\n");
        assert!(!block.contains("<div"), "{block}");
        assert!(block.starts_with("<p>&lt;div"), "{block}");
    }

    #[test]
    fn links_keep_only_web_schemes() {
        let ok = html("[a](https://example.com/x?y=1&z=2 \"T\")");
        assert_eq!(
            ok,
            "<p><a href=\"https://example.com/x?y=1&amp;z=2\" title=\"T\" target=\"_blank\" \
             rel=\"noopener noreferrer\">a</a></p>\n"
        );
        let js = html("[a](javascript:alert(1))");
        assert_eq!(js, "<p>a</p>\n");
        let relative = html("[a](/local)");
        assert_eq!(relative, "<p>a</p>\n");
        let mail = html("<me@example.com>");
        assert!(mail.contains("href=\"mailto:me@example.com\""), "{mail}");
    }

    #[test]
    fn bare_urls_autolink() {
        let auto = html("see https://example.com/a?b=1 now");
        assert_eq!(
            auto,
            "<p>see <a href=\"https://example.com/a?b=1\" target=\"_blank\" \
             rel=\"noopener noreferrer\">https://example.com/a?b=1</a> now</p>\n"
        );
        let trailing = html("(https://example.com/x). Then http://a.b, ok?");
        assert!(
            trailing.contains(">https://example.com/x</a>). Then"),
            "{trailing}"
        );
        assert!(trailing.contains(">http://a.b</a>, ok?"), "{trailing}");
        let paren = html("https://en.wikipedia.org/wiki/Rust_(programming_language) is");
        assert!(
            paren.contains("Rust_(programming_language)</a> is"),
            "{paren}"
        );
        let inside_link = html("[https://a.b](https://c.d)");
        assert_eq!(inside_link.matches("<a ").count(), 1, "{inside_link}");
        let inside_code = html("`https://a.b`");
        assert!(!inside_code.contains("<a "), "{inside_code}");
        let not_a_word = html("xhttps://a.b");
        assert!(!not_a_word.contains("<a "), "{not_a_word}");
        let bare = html("https:// alone");
        assert!(!bare.contains("<a "), "{bare}");
    }

    #[test]
    fn images_keep_only_safe_sources() {
        let ok = html("![alt](https://example.com/a.png)");
        assert!(
            ok.contains("<img src=\"https://example.com/a.png\" alt=\"alt\""),
            "{ok}"
        );
        let bad = html("![alt *text*](javascript:x)");
        assert_eq!(
            bad,
            "<p><span class=\"rc-image-alt\">alt <em>text</em></span></p>\n"
        );
        let off = render_html(
            "![alt](https://example.com/a.png)",
            &RenderOptions {
                images: false,
                ..RenderOptions::default()
            },
        );
        assert!(!off.contains("<img"), "{off}");
    }

    #[test]
    fn inline_and_display_math() {
        let inline = html("Euler: $e^{i\\pi} + 1 = 0$.");
        assert!(inline.contains("<p>Euler: <math"), "{inline}");
        assert!(!inline.contains("display=\"block\""), "{inline}");

        let out = blocks("Before.\n\n$$\n\\int_0^1 x\\,dx\n$$\n\nAfter.");
        assert_eq!(out.len(), 3, "{out:#?}");
        assert!(
            matches!(&out[1].kind, BlockKind::Math { source, mathml }
                if source.trim() == "\\int_0^1 x\\,dx" && mathml.contains("display=\"block\"")),
            "{out:#?}"
        );

        let mixed = html("Text $$a+b$$ more.");
        assert!(
            mixed.contains("<span class=\"rc-math-display\"><math"),
            "{mixed}"
        );
    }

    #[test]
    fn math_can_be_switched_off() {
        let out = render_html(
            "$x$",
            &RenderOptions {
                math: false,
                ..RenderOptions::default()
            },
        );
        assert_eq!(out, "<p>$x$</p>\n");
    }

    #[test]
    fn dollars_in_prose_stay_dollars() {
        assert_eq!(
            html("It costs $5 and $10."),
            "<p>It costs $5 and $10.</p>\n"
        );
    }

    #[test]
    fn fenced_code_is_a_code_block() {
        let out = blocks("```rust\nfn main() {}\n```\n");
        assert_eq!(out.len(), 1);
        match &out[0].kind {
            BlockKind::Code {
                source,
                html,
                language,
            } => {
                assert_eq!(source, "fn main() {}");
                assert!(!html.contains('<') || html.contains("<span"), "{html}");
                if cfg!(feature = "highlight") {
                    assert_eq!(language.as_deref(), Some("Rust"));
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fence_info_string_extras_are_ignored() {
        let out = blocks("```rust title=\"x\"\nlet a = 1;\n```\n");
        if cfg!(feature = "highlight") {
            assert!(
                matches!(&out[0].kind, BlockKind::Code { language: Some(l), .. } if l == "Rust")
            );
        }
    }

    #[test]
    fn code_in_a_list_is_inline_html() {
        let out = html("- item\n\n  ```js\n  let a = 1 < 2;\n  ```\n");
        assert!(out.contains("<pre class=\"rc-code\""), "{out}");
        assert!(out.contains("&lt;"), "{out}");
        assert!(!out.contains("1 < 2"), "{out}");
    }

    #[test]
    fn unclosed_fence_in_a_draft_is_still_code() {
        let out = render_blocks("```python\nprint(1)", &RenderOptions::draft());
        assert!(matches!(&out[0].kind, BlockKind::Code { source, .. } if source == "print(1)"));
    }

    #[test]
    fn draft_closes_math() {
        let out = render_html("so $x^2", &RenderOptions::draft());
        assert!(out.contains("<math"), "{out}");
        let braces = render_html("$$\\frac{n(n+1)(2n+", &RenderOptions::draft());
        assert!(braces.contains("<math"), "{braces}");
        let plain = html("so $x^2");
        assert!(!plain.contains("<math"), "{plain}");
    }

    #[test]
    fn tables_get_a_scroll_wrapper() {
        let out = html("| a | b |\n|---|--:|\n| 1 | 2 |\n");
        assert!(
            out.starts_with("<div class=\"rc-table-wrap\"><table>"),
            "{out}"
        );
        assert!(
            out.contains("<th style=\"text-align: right\">b</th>"),
            "{out}"
        );
    }

    #[test]
    fn task_lists_and_alerts() {
        let tasks = html("- [x] done\n- [ ] todo\n");
        assert!(tasks.contains("type=\"checkbox\" checked=\"\""), "{tasks}");
        let alert = html("> [!NOTE]\n> Careful.\n");
        assert!(
            alert.contains("<blockquote class=\"markdown-alert-note\">"),
            "{alert}"
        );
    }

    #[test]
    fn footnotes_are_numbered_and_collected_at_the_end() {
        let out = blocks("One[^a].\n\n[^a]: Note A.\n\nTwo[^b] and again[^a].\n\n[^b]: Note B.\n");
        assert_eq!(out.len(), 3, "{out:#?}");
        let BlockKind::Html(first) = &out[0].kind else {
            panic!()
        };
        assert!(
            first.contains("<sup class=\"rc-footnote-ref\"><a href=\"#rc-"),
            "{first}"
        );
        assert!(first.contains("\">1</a></sup>"), "{first}");
        let BlockKind::Html(second) = &out[1].kind else {
            panic!()
        };
        assert!(second.contains("\">2</a></sup>"), "{second}");
        assert!(second.contains("-fnref-1-1\">1</a></sup>"), "{second}");
        let BlockKind::Html(list) = &out[2].kind else {
            panic!()
        };
        assert!(
            list.starts_with("<section class=\"rc-footnotes\"><ol>"),
            "{list}"
        );
        let a = list.find("Note A").unwrap();
        let b = list.find("Note B").unwrap();
        assert!(a < b, "{list}");
        assert!(list.contains("class=\"rc-footnote-backref\""), "{list}");
    }

    #[test]
    fn headings_lists_quotes() {
        let out = html("## Two\n\n> quote\n\n1. one\n2. two\n");
        assert!(out.contains("<h2>Two</h2>"), "{out}");
        assert!(out.contains("<blockquote>"), "{out}");
        assert!(out.contains("<ol>"), "{out}");
    }

    #[test]
    fn empty_input_is_empty() {
        assert!(blocks("").is_empty());
        assert!(blocks("   \n\n").is_empty());
    }

    #[test]
    fn smart_punctuation_is_opt_in() {
        assert_eq!(html("\"hi\""), "<p>\"hi\"</p>\n");
        let smart = render_html(
            "\"hi\"",
            &RenderOptions {
                smart_punctuation: true,
                ..RenderOptions::default()
            },
        );
        assert_eq!(smart, "<p>\u{201c}hi\u{201d}</p>\n");
    }

    #[test]
    fn scheme_parsing() {
        assert_eq!(scheme("https://x"), Some("https"));
        assert_eq!(scheme("HTTPS://x"), Some("HTTPS"));
        assert_eq!(scheme("mailto:a@b"), Some("mailto"));
        assert_eq!(scheme("/relative"), None);
        assert_eq!(scheme("no scheme:here"), None);
        assert_eq!(scheme("#frag"), None);
        assert!(is_safe_image("data:image/png;base64,AAAA"));
        assert!(!is_safe_image("data:text/html,x"));
    }
}
