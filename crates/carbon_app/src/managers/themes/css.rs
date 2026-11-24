//! The `theme.css` check. Themes are files shared by strangers, so their CSS
//! must never load anything from outside the theme: a remote URL would let a
//! theme track the people who use it.
//!
//! The CSS is tokenized the way browsers do it (`cssparser`) and every token is
//! visited, in every block and function. Relative `url()`s are spliced back
//! into the original text as absolute asset URLs; every other byte is copied
//! unchanged, so the author's CSS is never re-printed.

use cssparser::{
    BasicParseErrorKind, CowRcStr, ParseError, ParseErrorKind, Parser, Token, serialize_string,
};
use percent_encoding::percent_decode_str;

use super::error::ThemeError;
use super::files::{ThemeFiles, ThemeLimits, normalize_rel_path, shown};

/// Lowercased prefixes that make a string anywhere else in the CSS a remote or
/// inline URL. Custom properties can hold strings that `var()` later feeds to
/// `image-set()` or `src()`, where they would be fetched.
const URL_STRING_PREFIXES: [&str; 9] = [
    "http:", "https:", "ftp:", "ws:", "wss:", "file:", "data:", "blob:", "//",
];

/// Functions whose bare string arguments browsers fetch as URLs.
/// `image()` isn't shipped by browsers yet, but its strings are URLs too.
const URL_STRING_FUNCTIONS: [&str; 3] = ["image-set", "-webkit-image-set", "image"];

/// Substitution functions (plus custom `--*()` functions). Directly inside a
/// [`URL_STRING_FUNCTIONS`] function they could hand it a string the check
/// never saw as a URL, such as `"/etc/hostname"`, which loads as a local file.
/// Authors put `url(…)` in the custom property instead, which is checked.
/// `inherit()` is in CSS Values 5 but not shipped yet.
const SUBSTITUTION_FUNCTIONS: [&str; 5] = ["var", "env", "attr", "if", "inherit"];

/// Checks `theme.css` and returns it with each accepted `url()` replaced by
/// `url("<asset_url(path)><#fragment>")`. Everything else is copied unchanged.
pub fn check_theme_css(
    css: &str,
    files: &ThemeFiles,
    limits: &ThemeLimits,
    asset_url: &dyn Fn(&str) -> String,
) -> Result<String, ThemeError> {
    // Windows PowerShell 5.1 and some editors write a UTF-8 byte order mark.
    let css = css.strip_prefix('\u{FEFF}').unwrap_or(css);
    if css.len() as u64 > limits.max_css_bytes {
        return Err(ThemeError::Invalid(
            "theme.css is larger than 1 MB".to_string(),
        ));
    }

    let mut check = Check {
        files,
        asset_url,
        edits: Vec::new(),
        problem: None,
    };
    let mut parser = Parser::new(css);
    // `walk` only returns `Ok` at the real end of the input; the exhaustion
    // check makes sure nothing was left unvisited if that ever changes.
    if check.walk(&mut parser, None).is_err() || !parser.is_exhausted() {
        let problem = check.problem.take().unwrap_or_else(|| {
            format!(
                "theme.css line {}: couldn't read the CSS here",
                parser.current_source_location().line + 1
            )
        });
        return Err(ThemeError::Invalid(problem));
    }

    // The walk visits tokens in source order, so the spans are already sorted
    // and never overlap.
    let mut out = String::with_capacity(css.len());
    let mut copied = 0;
    for (start, end, replacement) in &check.edits {
        debug_assert!(*start >= copied && end >= start);
        out.push_str(&css[copied..*start]);
        out.push_str(replacement);
        copied = *end;
    }
    out.push_str(&css[copied..]);
    Ok(out)
}

struct Check<'a> {
    files: &'a ThemeFiles,
    asset_url: &'a dyn Fn(&str) -> String,
    /// `(start, end, replacement)` byte spans in the original CSS.
    edits: Vec<(usize, usize, String)>,
    /// The first problem found; the walk unwinds as soon as it is set.
    problem: Option<String>,
}

impl Check<'_> {
    fn fail(&mut self, line: u32, message: impl Into<String>) -> ParseError<()> {
        if self.problem.is_none() {
            self.problem = Some(format!("theme.css line {line}: {}", message.into()));
        }
        ParseError::custom(())
    }

    /// Visits every token of the current block. `url_strings` is the function
    /// name when the block is directly inside one of [`URL_STRING_FUNCTIONS`].
    fn walk(
        &mut self,
        input: &mut Parser<'_>,
        url_strings: Option<&'static str>,
    ) -> Result<(), ParseError<()>> {
        loop {
            // Whitespace and comments are tokens here, so the position before
            // `next` is where the next token starts.
            let start = input.position().byte_index();
            let line = input.current_source_location().line + 1;
            let token = match input.next_including_whitespace_and_comments() {
                Ok(token) => token.clone(),
                Err(e) if e.kind == BasicParseErrorKind::EndOfInput => return Ok(()),
                Err(_) => return Err(self.fail(line, "couldn't read the CSS here")),
            };

            match token {
                Token::AtKeyword(name) if name.eq_ignore_ascii_case("import") => {
                    return Err(self.fail(line, "@import isn't allowed"));
                }
                Token::BadUrl(_) => {
                    return Err(self.fail(line, "this url() is malformed"));
                }
                Token::BadString(_) => {
                    return Err(self.fail(line, "a string isn't closed before the end of the line"));
                }
                Token::UnquotedUrl(value) => {
                    let end = input.position().byte_index();
                    self.url(line, start, end, &value)?;
                }
                Token::QuotedString(value) if url_strings.is_some() => {
                    let end = input.position().byte_index();
                    self.url(line, start, end, &value)?;
                }
                Token::QuotedString(value) => self.string(line, &value)?,
                Token::Function(name)
                    if name.eq_ignore_ascii_case("url") || name.eq_ignore_ascii_case("src") =>
                {
                    let value = self.nested(input, line, |check, input| {
                        check.single_string(input, line, &name)
                    })?;
                    let end = input.position().byte_index();
                    self.url(line, start, end, &value)?;
                }
                Token::Function(name) if url_strings.is_some() && is_substitution(&name) => {
                    // Custom function names are case-sensitive; the others aren't.
                    let inner = if name.starts_with("--") {
                        shown(&name)
                    } else {
                        name.to_ascii_lowercase()
                    };
                    return Err(self.fail(
                        line,
                        format!(
                            "{inner}() can't be used inside {}(); use url() instead",
                            url_strings.unwrap_or_default()
                        ),
                    ));
                }
                Token::Function(name) => {
                    let inner = URL_STRING_FUNCTIONS
                        .into_iter()
                        .find(|f| name.eq_ignore_ascii_case(f));
                    self.nested(input, line, |check, input| check.walk(input, inner))?;
                }
                Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
                    self.nested(input, line, |check, input| check.walk(input, None))?;
                }
                _ => {}
            }
        }
    }

    /// Runs `parse` on the block or function that was just opened. cssparser
    /// refuses to go deeper than its nesting limit without looking inside, so
    /// that is a problem too, not something to skip.
    fn nested<'i, T>(
        &mut self,
        input: &mut Parser<'i>,
        line: u32,
        parse: impl FnOnce(&mut Self, &mut Parser<'i>) -> Result<T, ParseError<()>>,
    ) -> Result<T, ParseError<()>> {
        input
            .parse_nested_block(|input| parse(self, input))
            .map_err(|e| match e.kind {
                ParseErrorKind::Custom(()) => e,
                ParseErrorKind::Basic(BasicParseErrorKind::TooManyNestedBlocks) => {
                    self.fail(line, "the CSS is nested too deeply")
                }
                ParseErrorKind::Basic(_) => self.fail(line, "couldn't read the CSS here"),
            })
    }

    /// The inside of `url(` or `src(`: one quoted string, nothing else.
    fn single_string<'i>(
        &mut self,
        input: &mut Parser<'i>,
        line: u32,
        name: &str,
    ) -> Result<CowRcStr<'i>, ParseError<()>> {
        let mut found = None;
        loop {
            match input.next_including_whitespace_and_comments() {
                Err(e) if e.kind == BasicParseErrorKind::EndOfInput => break,
                Err(_) => return Err(self.fail(line, "couldn't read the CSS here")),
                Ok(Token::WhiteSpace(_) | Token::Comment(_)) => {}
                Ok(Token::QuotedString(value)) if found.is_none() => found = Some(value.clone()),
                Ok(_) => {
                    found = None;
                    break;
                }
            }
        }
        found.ok_or_else(|| {
            self.fail(
                line,
                format!(
                    "{}() must hold one quoted file path",
                    name.to_ascii_lowercase()
                ),
            )
        })
    }

    /// A URL value. Accepted ones are recorded as an edit of `start..end`.
    fn url(
        &mut self,
        line: u32,
        start: usize,
        end: usize,
        raw: &str,
    ) -> Result<(), ParseError<()>> {
        let value = clean(raw);

        // `url(#id)` points into the page itself (SVG filters, masks).
        if value.starts_with('#') {
            return Ok(());
        }
        if points_outside(&value) {
            return Err(self.fail(line, outside(&value)));
        }

        let (path, fragment) = match value.find('#') {
            Some(i) => value.split_at(i),
            None => (value.as_str(), ""),
        };
        if path.contains('?') {
            return Err(self.fail(
                line,
                format!("a file path can't have a query: \"{}\"", shown(&value)),
            ));
        }
        let rel = match percent_decode_str(path).decode_utf8() {
            Ok(decoded) => normalize_rel_path(&decoded),
            Err(_) => None,
        };
        let Some(rel) = rel else {
            return Err(self.fail(line, outside(&value)));
        };

        match self.files.kind_of(&rel) {
            Some(kind) if kind.is_asset() => {}
            Some(_) => {
                return Err(self.fail(
                    line,
                    format!(
                        "\"{}\" isn't an image, font, audio or video file",
                        shown(&rel)
                    ),
                ));
            }
            None => {
                let message = self.missing(&rel);
                return Err(self.fail(line, message));
            }
        }

        // Escaped as a CSS string, so nothing in the fragment (or in a file
        // name) can close the string and add CSS of its own.
        let mut replacement = String::from("url(");
        let target = format!("{}{fragment}", (self.asset_url)(&rel));
        serialize_string(&target, &mut replacement).expect("writing to a String can't fail");
        replacement.push(')');
        self.edits.push((start, end, replacement));
        Ok(())
    }

    /// Any other quoted string: refused when it reads as a remote or inline URL.
    fn string(&mut self, line: u32, raw: &str) -> Result<(), ParseError<()>> {
        let value = clean(raw).replace('\\', "/").to_ascii_lowercase();
        if URL_STRING_PREFIXES.iter().any(|p| value.starts_with(p)) {
            return Err(self.fail(line, outside(&clean(raw))));
        }
        Ok(())
    }

    fn missing(&self, rel: &str) -> String {
        let lower = rel.to_lowercase();
        match self.files.files.keys().find(|k| k.to_lowercase() == lower) {
            Some(other) => format!(
                "\"{}\" isn't in the theme (the case must match \"{}\")",
                shown(rel),
                shown(other)
            ),
            None => format!("\"{}\" isn't in the theme", shown(rel)),
        }
    }
}

/// What browsers do to a URL before parsing it: tab, LF and CR are removed
/// everywhere (FF too, to be safe), and C0 controls and spaces are trimmed
/// from both ends.
fn clean(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r' | '\x0C'))
        .collect();
    kept.trim_matches(|c: char| c <= ' ').to_string()
}

/// A cleaned URL value that can only mean something outside the theme: it has
/// a scheme, or it starts at a root (`/x`, `//host`, `\\host`).
fn points_outside(value: &str) -> bool {
    has_scheme(value) || value.starts_with('/') || value.starts_with('\\')
}

fn is_substitution(name: &str) -> bool {
    name.starts_with("--")
        || SUBSTITUTION_FUNCTIONS
            .iter()
            .any(|f| name.eq_ignore_ascii_case(f))
}

/// `^[A-Za-z][A-Za-z0-9+.-]*:`
fn has_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

fn outside(value: &str) -> String {
    format!(
        "only files inside the theme can be used, not \"{}\"",
        shown(value)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::themes::files::{ThemeFiles, ThemeLimits};

    fn files() -> ThemeFiles {
        let list = [
            "assets/a.png",
            "assets/b.png",
            "assets/f.woff2",
            "assets/i.svg",
            "assets/My Snow ❄.svg",
            "theme.css",
            "manifest.json",
            "fonts/OFL.txt",
        ];
        ThemeFiles {
            files: list.iter().map(|f| (f.to_string(), 10)).collect(),
            fingerprint: 0,
        }
    }

    fn check_with(css: &str, limits: &ThemeLimits) -> Result<String, ThemeError> {
        check_theme_css(css, &files(), limits, &|p| format!("asset:{p}"))
    }

    fn check(css: &str) -> Result<String, ThemeError> {
        check_with(css, &ThemeLimits::DEFAULT)
    }

    #[test]
    fn a_leading_byte_order_mark_is_ignored() {
        let out = check("\u{FEFF}a{color:red}").unwrap();
        assert_eq!(out, "a{color:red}");
    }

    fn msg(r: Result<String, ThemeError>) -> String {
        match r {
            Err(ThemeError::Invalid(m)) => m,
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    const OUTSIDE: &str = "only files inside the theme can be used";

    /// Asserts that `css` is refused, and for the expected reason.
    fn refused(css: &str, reason: &str) {
        let m = msg(check(css));
        assert!(m.contains(reason), "{css:?} was refused with {m:?}");
    }

    #[test]
    fn rewrites_relative_urls_and_keeps_the_rest() -> Result<(), ThemeError> {
        let css = "a{background:url(assets/a.png)}\n@font-face{font-family:X;src:url('assets/f.woff2') format('woff2')}\nb{color:red}";
        assert_eq!(
            check(css)?,
            "a{background:url(\"asset:assets/a.png\")}\n@font-face{font-family:X;src:url(\"asset:assets/f.woff2\") format('woff2')}\nb{color:red}"
        );
        Ok(())
    }

    #[test]
    fn rewrites_inside_nested_blocks_and_image_set() -> Result<(), ThemeError> {
        let css = r#"@media (min-width:1px){@supports (display:grid){.x{background:image-set("assets/a.png" 1x, url(assets/b.png) 2x)}}}"#;
        assert_eq!(
            check(css)?,
            r#"@media (min-width:1px){@supports (display:grid){.x{background:image-set(url("asset:assets/a.png") 1x, url("asset:assets/b.png") 2x)}}}"#
        );
        Ok(())
    }

    #[test]
    fn keeps_fragment_only_urls() -> Result<(), ThemeError> {
        assert_eq!(check(".x{filter:url(#glow)}")?, ".x{filter:url(#glow)}");
        assert_eq!(
            check(".x{mask:url(assets/i.svg#star)}")?,
            ".x{mask:url(\"asset:assets/i.svg#star\")}"
        );
        Ok(())
    }

    #[test]
    fn decodes_names_with_spaces_and_unicode() -> Result<(), ThemeError> {
        let want = ".x{background:url(\"asset:assets/My Snow ❄.svg\")}";
        assert_eq!(check(".x{background:url(\"assets/My Snow ❄.svg\")}")?, want);
        assert_eq!(
            check(".x{background:url(assets/My%20Snow%20%E2%9D%84.svg)}")?,
            want
        );
        refused(
            ".x{background:url(\"assets/my snow ❄.svg\")}",
            "the case must match \"assets/My Snow ❄.svg\"",
        );
        Ok(())
    }

    #[test]
    fn rejects_remote_and_inline_urls() {
        for url in [
            "url(https://e.com/a.png)",
            "url(http://e.com/a.png)",
            "url(data:image/png;base64,AAAA)",
            "url(file:///etc/passwd)",
            "url(//e.com/a.png)",
            "url(/assets/a.png)",
        ] {
            refused(&format!("a{{background:{url}}}"), OUTSIDE);
        }
        for css in ["@import \"x.css\";", "@import url(x.css);"] {
            refused(css, "@import isn't allowed");
        }
    }

    #[test]
    fn rejects_obfuscated_remote_urls() {
        for value in [
            r"URL(https://e.com/a)",
            r"u\72l(https://e.com/a)",
            r#"url("ht\9tp://e.com/a")"#,
            r#"url("HTTPS://e.com/a")"#,
            r#"url("\\\\e.com/a.png")"#,
            r#"src("https://e.com/a")"#,
            r#"image-set("https://e.com/a.png" 1x)"#,
            r#"-webkit-image-set("//e.com/a.png" 1x)"#,
        ] {
            refused(&format!("a{{background:{value}}}"), OUTSIDE);
        }
        refused(r#".x{--u:"https://e.com/a.png"}"#, OUTSIDE);
    }

    #[test]
    fn rejects_more_disguises() {
        for (value, reason) in [
            // Function form, any case, with whitespace and comments.
            (r#"URL("https://e.com/a")"#, OUTSIDE),
            (r#"url( "https://e.com/a" /**/ )"#, OUTSIDE),
            (r#"SrC("https://e.com/a")"#, OUTSIDE),
            (r#"s\72 c("https://e.com/a")"#, OUTSIDE),
            // Escapes and controls inside the value.
            (r"url(ht\74ps://e.com/a)", OUTSIDE),
            (r"url(\2f\2f e.com/a.png)", OUTSIDE),
            (r"url(\\\\e.com/a.png)", OUTSIDE),
            (r#"url("\\e.com/a.png")"#, OUTSIDE),
            (r#"url("\1 https://e.com/a")"#, OUTSIDE),
            (r#"url("\a //e.com/a")"#, OUTSIDE),
            (r#"url("C:/x.png")"#, OUTSIDE),
            // Percent-encoding can't hide a leading slash or a traversal.
            (r"url(%2F%2Fe.com/a.png)", OUTSIDE),
            (r"url(assets/%2e%2e/%2e%2e/x.png)", OUTSIDE),
            (r"url(assets/%FF.png)", OUTSIDE),
            // url() and src() take exactly one string.
            (
                r#"url("assets/a.png" "https://e.com/a")"#,
                "url() must hold one quoted file path",
            ),
            (r"url(var(--x))", "this url() is malformed"),
            (r"src(var(--x))", "src() must hold one quoted file path"),
            (
                r#"src("assets/a.png" format("png"))"#,
                "src() must hold one quoted file path",
            ),
            (r"src(assets/a.png)", "src() must hold one quoted file path"),
            // image-set() and image() strings, any case or escape.
            (r#"IMAGE-SET("https://e.com/a.png" 1x)"#, OUTSIDE),
            (r#"image-s\65t("https://e.com/a.png" 1x)"#, OUTSIDE),
            (r#"image-set("ht\9tps://e.com/a.png" 1x)"#, OUTSIDE),
            (r#"image("https://e.com/a.png")"#, OUTSIDE),
            // URL-looking strings anywhere else.
            (r#"" \1//e.com/a""#, OUTSIDE),
            (r#""/\\e.com/a""#, OUTSIDE),
            (r#""\9//e.com/a""#, OUTSIDE),
            (r#""HTTP://e.com/a""#, OUTSIDE),
            (r#""wss://e.com""#, OUTSIDE),
            (r#""blob:https://e.com/x""#, OUTSIDE),
            (r#""data:image/png;base64,AAAA""#, OUTSIDE),
        ] {
            refused(&format!("a{{--v:{value}}}"), reason);
        }
        for css in [
            "@\\69mport \"x.css\";",
            "@IMPORT url(x.css);",
            "@media screen{@import \"x.css\";}",
        ] {
            refused(css, "@import isn't allowed");
        }
    }

    #[test]
    fn refuses_substitution_inside_url_string_functions() {
        // A string that reaches image-set() through var() would load as a
        // local file (or the app's own file), never as a theme asset.
        for css in [
            r#"body{--u:"/etc/hostname";background-image:image-set(var(--u) 1x)}"#,
            r#"body{--u:"../../../../../../../../dev/zero";background-image:image-set(var(--u) 1x)}"#,
            r#"body{--u:"assets/rel.png";background-image:image-set(var(--u) 1x)}"#,
        ] {
            refused(
                css,
                "var() can't be used inside image-set(); use url() instead",
            );
        }
        for (value, reason) in [
            (
                r"IMAGE-SET(VAR(--u) 1x)",
                "var() can't be used inside image-set()",
            ),
            (
                r"image-set(v\61r(--u) 1x)",
                "var() can't be used inside image-set()",
            ),
            (
                r"-webkit-image-set(var(--u) 1x)",
                "var() can't be used inside -webkit-image-set()",
            ),
            (r"image(var(--u))", "var() can't be used inside image()"),
            (r"image-set(env(--u) 1x)", "env() can't be used"),
            (r"image-set(attr(data-u) 1x)", "attr() can't be used"),
            (
                r"image-set(if(media(print): 'a'; else: 'b') 1x)",
                "if() can't be used",
            ),
            (r"image-set(inherit(--u) 1x)", "inherit() can't be used"),
            (r"image-set(--pick(1) 1x)", "--pick() can't be used"),
            (r"image-set(\2d -pick(1) 1x)", "--pick() can't be used"),
            (
                r#"image-set(url(assets/a.png) 1x, var(--u) 2x)"#,
                "var() can't be used",
            ),
        ] {
            refused(&format!("a{{background-image:{value}}}"), reason);
        }
    }

    #[test]
    fn allows_url_in_custom_properties_and_var_elsewhere() -> Result<(), ThemeError> {
        assert_eq!(
            check("a{--x: url(assets/a.png); background: var(--x)}")?,
            "a{--x: url(\"asset:assets/a.png\"); background: var(--x)}"
        );
        // Substitution is fine where it can't become a file path.
        for css in [
            "a{color:var(--c, red)}",
            "a{background-image:image-set(linear-gradient(var(--c), red) 1x)}",
            r#"a{background-image:image-set("assets/a.png" type(var(--t)))}"#,
        ] {
            check(css)?;
        }
        Ok(())
    }

    #[test]
    fn spots_schemes_and_rooted_paths() {
        for yes in [
            "https://e.com",
            "HTTPS:",
            "a+b.c-d:x",
            "C:/x.png",
            "javascript:x",
        ] {
            assert!(has_scheme(yes), "{yes}");
            assert!(points_outside(yes), "{yes}");
        }
        for no in [
            "",
            ":x",
            "1http://e.com",
            "+a:x",
            "assets/a.png",
            "assets/a:b.png",
            "a b:c",
            "ｈttps://e.com",
        ] {
            assert!(!has_scheme(no), "{no}");
        }
        for rooted in ["/etc/hostname", "//e.com/a", "\\\\e.com\\a", "\\a", "/"] {
            assert!(!has_scheme(rooted), "{rooted}");
            assert!(points_outside(rooted), "{rooted}");
        }
        for inside in ["assets/a.png", "./assets/a.png", "../x.png", "a/b:c"] {
            assert!(!points_outside(inside), "{inside}");
        }
    }

    #[test]
    fn allows_whitespace_and_other_url_forms() -> Result<(), ThemeError> {
        let want = r#"a{b:url("asset:assets/a.png")}"#;
        for css in [
            r#"a{b:url( "assets/a.png" )}"#,
            r"a{b:url(  assets/a.png  )}",
            r#"a{b:SRC("assets/a.png")}"#,
            r#"a{b:url("./assets/a.png")}"#,
        ] {
            assert_eq!(check(css)?, want, "{css}");
        }
        assert_eq!(
            check(r#"a{b:image("assets/a.png")}"#)?,
            r#"a{b:image(url("asset:assets/a.png"))}"#
        );
        assert_eq!(
            check("a{}\r\nb{background:url(assets/a.png)}\r\n")?,
            "a{}\r\nb{background:url(\"asset:assets/a.png\")}\r\n"
        );
        Ok(())
    }

    #[test]
    fn a_fragment_cant_break_out_of_the_rewritten_string() -> Result<(), ThemeError> {
        // The fragment decodes to `") url(https://e.com)`.
        let css = r"a{b:url(assets/i.svg#\22\29\20url\28 https\3a//e.com\29)}";
        assert_eq!(
            check(css)?,
            r#"a{b:url("asset:assets/i.svg#\") url(https://e.com)")}"#
        );
        Ok(())
    }

    #[test]
    fn rejects_bad_tokens_and_deep_nesting() {
        refused(
            "a{background:url(assets/a .png)}",
            "this url() is malformed",
        );
        refused(
            "a{background:url(assets/a(.png)}",
            "this url() is malformed",
        );
        refused(
            "a{content:\"abc\n}",
            "a string isn't closed before the end of the line",
        );
        let deep = format!(
            "a{{b:{}url(https://e.com/a){}}}",
            "(".repeat(200),
            ")".repeat(200)
        );
        refused(&deep, "the CSS is nested too deeply");
        let fine = format!(
            "a{{b:{}url(assets/a.png){}}}",
            "(".repeat(20),
            ")".repeat(20)
        );
        assert!(check(&fine).is_ok());
    }

    #[test]
    fn rejects_urls_to_missing_or_non_asset_files() {
        for (url, reason) in [
            (
                "url(assets/missing.png)",
                "\"assets/missing.png\" isn't in the theme",
            ),
            (
                "url(theme.css)",
                "\"theme.css\" isn't an image, font, audio or video file",
            ),
            (
                "url(manifest.json)",
                "\"manifest.json\" isn't an image, font, audio or video file",
            ),
            (
                "url(fonts/OFL.txt)",
                "\"fonts/OFL.txt\" isn't an image, font, audio or video file",
            ),
            ("url(../x.png)", OUTSIDE),
            ("url(assets/a.png?x=1)", "a file path can't have a query"),
        ] {
            refused(&format!("a{{background:{url}}}"), reason);
        }
    }

    #[test]
    fn ignores_urls_in_comments() -> Result<(), ThemeError> {
        let css = "/* url(https://e.com) */ a{}";
        assert_eq!(check(css)?, css);
        Ok(())
    }

    #[test]
    fn allows_harmless_strings() -> Result<(), ThemeError> {
        for css in [
            r#"a::before{content:"Note: hello"}"#,
            r#"a{font-family:"My Font"}"#,
            r#"@font-face{font-family:X;src:url(assets/f.woff2) format("woff2")}"#,
            r#"@font-face{font-family:X;src:local("Arial")}"#,
        ] {
            check(css)?;
        }
        Ok(())
    }

    #[test]
    fn enforces_the_size_limit() {
        let limits = ThemeLimits {
            max_css_bytes: 10,
            ..ThemeLimits::DEFAULT
        };
        assert_eq!(
            msg(check_with("a{color:red}", &limits)),
            "theme.css is larger than 1 MB"
        );
        assert!(check_with("a{color:0}", &limits).is_ok());
    }

    #[test]
    fn error_names_the_line() {
        let m = msg(check(
            "a{color:red}\nb{color:blue}\nc{background:url(https://e.com/a.png)}",
        ));
        assert!(m.contains("line 3"), "{m}");
        assert!(
            m.contains("only files inside the theme can be used, not \"https://e.com/a.png\""),
            "{m}"
        );
    }
}
