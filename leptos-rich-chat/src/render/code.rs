//! Syntax highlighting for fenced code blocks.
//!
//! Highlighting is class-based: every token becomes a `<span>` whose
//! classes name its TextMate scope (`hl-keyword hl-control`), and the
//! crate's stylesheet maps those classes to colors for the light and dark
//! themes. Colors therefore live in CSS, where a consumer can override
//! them, not in the markup.
//!
//! The classes have a prefix of their own, [`CLASS_PREFIX`], not the
//! components' `rc-`. Every dotted atom of a scope becomes a class, so a
//! `meta.block` scope would otherwise come out as `rc-block`, the wrapper
//! `RichText` puts around each block, and `entity.name.function` as
//! `rc-name`, and any rule against a component's class would land on
//! the highlighted spans too. A test in `style` checks that no class
//! the components or the stylesheets use appears in the highlight
//! stylesheet, so a collision cannot come back.
//!
//! The grammar set is two-face's superset of Sublime Text's defaults:
//! about two hundred languages, resolved by name, alias, or file
//! extension. Deserializing it is cheap; what costs is that syntect
//! compiles each grammar's regular expressions the first time that
//! language is highlighted, up to half a second for a large grammar like
//! TypeScript. [`warm`] pays that price ahead of time, and
//! [`WARM_LANGUAGES`] lists the languages worth paying it for.

use pulldown_cmark_escape::escape_html;

/// Prefix on every highlight class, so the stylesheet can be scoped.
/// Not the components' `rc-`: see the [module docs](self).
pub const CLASS_PREFIX: &str = "hl-";

/// How much of a block gets highlighted. Past this many bytes the rest of
/// the block renders plain rather than stalling the UI.
pub const HIGHLIGHT_CAP: usize = 64 * 1024;

/// The highlighted form of a code block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Highlighted {
    /// The language's display name (`Rust`, `TypeScript`), when the
    /// fence's token resolved to a grammar.
    pub language: Option<String>,
    /// The block's text as HTML: escaped, with highlight spans when a
    /// grammar was found.
    pub html: String,
}

/// Highlights `code` as the language `token` names.
///
/// `token` is the first word of the fence's info string, if any; an
/// unknown or absent token yields escaped plain text. Highlighting never
/// fails and never panics — the worst case is plain text.
pub fn highlight(token: Option<&str>, code: &str) -> Highlighted {
    #[cfg(feature = "highlight")]
    if let Some(highlighted) = syntect_impl::highlight(token, code) {
        return highlighted;
    }
    Highlighted {
        language: None,
        html: escape(code),
    }
}

/// Deserializes the grammar set now instead of on the first highlight.
/// A no-op without the `highlight` feature.
pub fn preload() {
    #[cfg(feature = "highlight")]
    syntect_impl::preload();
}

/// The languages most often pasted into a chat, in the order a warm-up
/// should compile them.
pub const WARM_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "javascript",
    "typescript",
    "bash",
    "json",
    "yaml",
    "toml",
    "html",
    "css",
    "sql",
    "go",
    "java",
    "c",
    "cpp",
    "markdown",
];

/// Compiles a language's grammar now, so that the first block in it
/// renders without a pause. Every pattern a grammar tries on a line is
/// compiled by the attempt, so a few lines with the usual shapes of code
/// in them cover what an ordinary block would reach. A no-op for an
/// unknown token or without the `highlight` feature.
pub fn warm(token: &str) {
    const SNIPPET: &str = "\
// comment  # comment  -- comment  /* comment */
fn f(x: u32) -> Option<&str> { return Some(\"text\"); } // rust, go, c, java
def f(x=1.5): return 'text' + str(x)  # python
const g = async (a, b) => { await a?.b ?? `tmpl ${b}`; };  // javascript
let y = 2; var z = [1, 2]; y += z[0]; if (y === 3 && !z) { y++; } else { y--; }
for (let i = 0; i < 10; i++) { while (true) { break; } }
import { a } from \"b\"; export default function h(p: string): void {}
class C<T> extends B implements I { private x: number = 0; constructor() { super(); } }
struct S { a: Vec<u8> } impl S { pub fn new() -> Self { Self { a: vec![] } } }
match x { Some(v) => v, None => 0 }
try: pass\nexcept Exception as e: raise\nwith open(f) as h: pass\nfor i in range(3): print(i)
<div class=\"a\" id='b'>{ x }</div>
SELECT a, count(*) FROM t WHERE b = 'c' GROUP BY a;
key: [1, 2.0, true, null, \"s\"]
[section]
name = \"value\"
if [ -f \"$file\" ]; then echo \"$file\" | grep -c x; fi
";
    let _ = highlight(Some(token), SNIPPET);
}

fn escape(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    escape_html(&mut out, code).expect("a String never refuses a write");
    out
}

#[cfg(feature = "highlight")]
mod syntect_impl {
    use std::sync::LazyLock;

    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::parsing::{SyntaxReference, SyntaxSet};
    use syntect::util::LinesWithEndings;

    use super::{CLASS_PREFIX, HIGHLIGHT_CAP, Highlighted};

    static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

    pub fn preload() {
        LazyLock::force(&SYNTAXES);
    }

    /// Fence tokens that neither a grammar name nor a file extension
    /// covers, mapped to the grammar name they mean.
    const ALIASES: &[(&str, &str)] = &[
        ("shell", "Bourne Again Shell (bash)"),
        ("sh", "Bourne Again Shell (bash)"),
        ("zsh", "Bourne Again Shell (bash)"),
        ("console", "Bourne Again Shell (bash)"),
        ("terminal", "Bourne Again Shell (bash)"),
        ("c++", "C++"),
        ("cpp", "C++"),
        ("c#", "C#"),
        ("csharp", "C#"),
        ("f#", "F#"),
        ("fsharp", "F#"),
        ("golang", "Go"),
        ("js", "JavaScript"),
        ("jsx", "JavaScript"),
        ("ts", "TypeScript"),
        ("tsx", "TypeScriptReact"),
        ("py", "Python"),
        ("rb", "Ruby"),
        ("kt", "Kotlin"),
        ("hs", "Haskell"),
        ("rs", "Rust"),
        ("yml", "YAML"),
        ("md", "Markdown"),
        ("tex", "LaTeX"),
        ("latex", "LaTeX"),
        ("objc", "Objective-C"),
        ("objective-c", "Objective-C"),
        ("docker", "Dockerfile"),
        ("dockerfile", "Dockerfile"),
        ("make", "Makefile"),
        ("makefile", "Makefile"),
        ("proto", "Protocol Buffer"),
        ("protobuf", "Protocol Buffer"),
        ("bat", "Batch File"),
        ("cmd", "Batch File"),
        ("asm", "x86_64 Assembly"),
        ("nasm", "x86_64 Assembly"),
        ("fortran", "Fortran (Modern)"),
        ("f90", "Fortran (Modern)"),
        ("regex", "Regular Expression"),
        ("regexp", "Regular Expression"),
        ("gdscript", "GDScript (Godot Engine)"),
        ("scheme", "Racket"),
        ("scm", "Racket"),
        ("rkt", "Racket"),
        ("plaintext", "Plain Text"),
        ("plain", "Plain Text"),
        ("text", "Plain Text"),
        ("txt", "Plain Text"),
        ("jsonc", "JSON"),
        ("json5", "JSON"),
        ("html", "HTML"),
        ("xhtml", "HTML"),
        ("vue", "Vue Component"),
        ("svelte", "Svelte"),
        ("hcl", "Terraform"),
        ("tf", "Terraform"),
        ("terraform", "Terraform"),
        ("ex", "Elixir"),
        ("exs", "Elixir"),
        ("erl", "Erlang"),
        ("ml", "OCaml"),
        ("clj", "Clojure"),
        ("cljs", "Clojure"),
        ("el", "Lisp"),
        ("elisp", "Lisp"),
        ("m", "Objective-C"),
        ("jl", "Julia"),
        ("nginx", "nginx"),
        ("gql", "GraphQL"),
        ("graphql", "GraphQL"),
        ("sql", "SQL"),
        ("pgsql", "SQL"),
        ("mysql", "SQL"),
        ("sqlite", "SQL"),
        ("diff", "Diff"),
        ("patch", "Diff"),
        ("ini", "INI"),
        ("cfg", "INI"),
        ("conf", "INI"),
        ("vim", "VimL"),
        ("viml", "VimL"),
        ("svg", "XML"),
        ("plist", "XML"),
        ("csv", "Comma Separated Values"),
    ];

    /// Resolves a fence token to a grammar: by alias, then by grammar
    /// name, then by file extension, all case-insensitively.
    pub fn resolve(token: &str) -> Option<&'static SyntaxReference> {
        let token = token.trim();
        if token.is_empty() {
            return None;
        }
        let lower = token.to_ascii_lowercase();
        let set = &*SYNTAXES;
        ALIASES
            .iter()
            .find(|(alias, _)| *alias == lower)
            .and_then(|(_, name)| set.find_syntax_by_name(name))
            .or_else(|| set.find_syntax_by_token(token))
            .or_else(|| set.find_syntax_by_token(&lower))
    }

    pub fn highlight(token: Option<&str>, code: &str) -> Option<Highlighted> {
        let set = &*SYNTAXES;
        let syntax = match token {
            Some(token) => resolve(token)?,
            None => set.find_syntax_by_first_line(code)?,
        };
        if syntax.name == "Plain Text" {
            return None;
        }
        let style = ClassStyle::SpacedPrefixed {
            prefix: CLASS_PREFIX,
        };
        let mut generator = ClassedHTMLGenerator::new_with_class_style(syntax, set, style);
        let mut consumed = 0;
        for line in LinesWithEndings::from(code) {
            if consumed + line.len() > HIGHLIGHT_CAP
                || generator
                    .parse_html_for_line_which_includes_newline(line)
                    .is_err()
            {
                break;
            }
            consumed += line.len();
        }
        let mut html = generator.finalize();
        if consumed < code.len() {
            html.push_str(&super::escape(&code[consumed..]));
        }
        Some(Highlighted {
            language: Some(syntax.name.clone()),
            html,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn unknown_language_is_escaped_plain_text() {
        let out = highlight(Some("no-such-language"), "a < b && c");
        assert_eq!(out.language, None);
        assert_eq!(out.html, "a &lt; b &amp;&amp; c");
    }

    #[test]
    fn no_language_and_no_shebang_is_plain() {
        let out = highlight(None, "just words");
        assert_eq!(out.language, None);
        assert_eq!(out.html, "just words");
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn rust_gets_highlight_spans() {
        let out = highlight(Some("rust"), "fn main() {}\n");
        assert_eq!(out.language.as_deref(), Some("Rust"));
        assert!(out.html.contains("class=\"hl-"), "{}", out.html);
        assert!(out.html.contains("main"), "{}", out.html);
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn a_shebang_picks_the_language() {
        let out = highlight(None, "#!/usr/bin/env python3\nprint(1)\n");
        assert_eq!(out.language.as_deref(), Some("Python"));
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn html_in_code_is_escaped() {
        let out = highlight(Some("html"), "<script>alert(1)</script>\n");
        assert!(!out.html.contains("<script>"), "{}", out.html);
    }

    #[test]
    fn tokens_are_matched_loosely() {
        let exact = highlight(Some("rust"), "let x = 1;\n");
        for token in ["Rust", "RUST", " rust ", "rs"] {
            assert_eq!(highlight(Some(token), "let x = 1;\n"), exact, "{token:?}");
        }
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn plain_text_tokens_are_plain() {
        for token in ["text", "plain", "txt", "plaintext"] {
            let out = highlight(Some(token), "a < b\n");
            assert_eq!(out.language, None, "{token}");
            assert_eq!(out.html, "a &lt; b\n");
        }
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn past_the_cap_the_rest_is_escaped_plain_text() {
        let line = "let s = \"<tag>\"; // x\n";
        let count = super::HIGHLIGHT_CAP / line.len() + 10;
        let code = line.repeat(count);
        let out = highlight(Some("rust"), &code);
        assert_eq!(out.language.as_deref(), Some("Rust"));
        assert!(
            !out.html.contains("<tag>"),
            "raw markup leaked past the cap"
        );
        assert_eq!(
            out.html.matches("&lt;tag&gt;").count(),
            count,
            "every line's text is present exactly once"
        );
        let highlighted = out.html.matches("<span").count();
        let plain_tail = out.html.trim_end().ends_with("// x");
        assert!(
            highlighted > 0 && plain_tail,
            "{}…",
            &out.html[out.html.len() - 80..]
        );
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn highlighting_preserves_the_text() {
        let code = "fn a() -> &'static str { \"x < y && z\" }\n";
        let out = highlight(Some("rust"), code);
        // Every literal `<` in the output is a tag, since text is escaped.
        let mut without_tags = String::new();
        let mut in_tag = false;
        for c in out.html.chars() {
            match c {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                c if !in_tag => without_tags.push(c),
                _ => {}
            }
        }
        let text = without_tags
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&");
        assert_eq!(text, code);
    }

    #[test]
    fn empty_code_is_empty() {
        assert_eq!(highlight(Some("rust"), "").html, "");
        assert_eq!(highlight(None, "").html, "");
    }

    #[test]
    fn preloading_is_harmless_and_changes_nothing() {
        let cold = highlight(Some("python"), "print(1)\n");
        super::preload();
        assert_eq!(highlight(Some("python"), "print(1)\n"), cold);
    }

    #[test]
    fn warming_is_harmless_and_changes_nothing() {
        let cold = highlight(Some("rust"), "fn main() {}\n");
        super::warm("rust");
        super::warm("no-such-language");
        super::warm("");
        assert_eq!(highlight(Some("rust"), "fn main() {}\n"), cold);
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn every_warm_language_resolves() {
        let unresolved: Vec<_> = super::WARM_LANGUAGES
            .iter()
            .filter(|token| highlight(Some(token), "x\n").language.is_none())
            .collect();
        assert!(unresolved.is_empty(), "unresolved: {unresolved:?}");
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn common_tokens_resolve() {
        let tokens = [
            "rust",
            "rs",
            "python",
            "py",
            "javascript",
            "js",
            "typescript",
            "ts",
            "tsx",
            "jsx",
            "go",
            "golang",
            "java",
            "kotlin",
            "swift",
            "c",
            "cpp",
            "c++",
            "csharp",
            "c#",
            "ruby",
            "rb",
            "php",
            "perl",
            "lua",
            "r",
            "julia",
            "scala",
            "haskell",
            "hs",
            "ocaml",
            "elixir",
            "erlang",
            "clojure",
            "lisp",
            "scheme",
            "elm",
            "dart",
            "zig",
            "nim",
            "bash",
            "sh",
            "shell",
            "zsh",
            "fish",
            "sql",
            "json",
            "jsonc",
            "yaml",
            "yml",
            "toml",
            "xml",
            "html",
            "css",
            "scss",
            "less",
            "markdown",
            "md",
            "tex",
            "latex",
            "dockerfile",
            "docker",
            "makefile",
            "cmake",
            "nginx",
            "ini",
            "diff",
            "patch",
            "graphql",
            "protobuf",
            "proto",
            "terraform",
            "groovy",
            "objc",
            "matlab",
            "fortran",
            "asm",
            "verilog",
            "vhdl",
            "lean",
            "vim",
            "http",
            "csv",
            "svg",
            "crystal",
            "fsharp",
            "cs",
            "cabal",
            "awk",
            "regex",
            "purescript",
            "racket",
            "rst",
            "sass",
            "tcl",
            "twig",
            "scheme",
            "zig",
            "nix",
            "solidity",
            "wgsl",
            "glsl",
            "typst",
            "odin",
            "gdscript",
        ];
        let unresolved: Vec<_> = tokens
            .iter()
            .filter(|token| highlight(Some(token), "x\n").language.is_none())
            .collect();
        assert!(unresolved.is_empty(), "unresolved: {unresolved:?}");
    }
}
