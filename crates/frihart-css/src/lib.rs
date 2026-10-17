//! CSS subset: declarations, rules, and a small selector grammar.

#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub selector: String,
    pub parsed: Option<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Simple {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    Pseudo(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compound {
    pub simples: Vec<Simple>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<(Option<Combinator>, Compound)>,
}

impl Stylesheet {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

pub fn parse_declarations(input: &str) -> Vec<Declaration> {
    let mut out = Vec::new();
    for part in input.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((n, v)) = part.split_once(':') {
            // Custom property names (`--x`) are case-sensitive; others are not.
            let name = if n.trim().starts_with("--") {
                n.trim().to_string()
            } else {
                n.trim().to_ascii_lowercase()
            };
            if name.is_empty() {
                continue;
            }
            out.push(Declaration {
                name,
                value: v.trim().to_string(),
            });
        }
    }
    out
}

/// What `@media` queries are evaluated against. Only the viewport size is
/// real; every preference feature answers with a fixed, common value so a
/// stylesheet cannot read the user's settings (fingerprinting).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaEnv {
    /// Viewport width and height in CSS px (letterboxed by the chrome).
    pub width: f32,
    pub height: f32,
}

/// Deepest `@media` nesting followed; deeper blocks are skipped.
pub const MAX_MEDIA_NESTING: usize = 4;
/// Longest media query text evaluated, in bytes; longer ones never match.
pub const MAX_MEDIA_QUERY_BYTES: usize = 512;
/// One `em` / `rem` inside a media query (the initial font size).
const MEDIA_EM_PX: f32 = 16.0;

/// Parse a stylesheet, skipping every at-rule block (`@media` included).
pub fn parse_stylesheet(input: &str) -> Stylesheet {
    parse_stylesheet_in(input, None)
}

/// Parse a stylesheet and keep the rules of `@media` blocks whose query
/// matches `env` (see [`media_matches`]); other at-rules are skipped.
pub fn parse_stylesheet_media(input: &str, env: MediaEnv) -> Stylesheet {
    parse_stylesheet_in(input, Some(env))
}

fn parse_stylesheet_in(input: &str, env: Option<MediaEnv>) -> Stylesheet {
    let input = strip_comments(input);
    let mut sheet = Stylesheet::default();
    parse_rules_into(&input, env, 0, &mut sheet);
    sheet
}

fn parse_rules_into(input: &str, env: Option<MediaEnv>, depth: usize, sheet: &mut Stylesheet) {
    let mut rest = input;
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        rest = &rest[open + 1..];
        if selector.starts_with('@') {
            let after = skip_balanced(rest);
            let body = &rest[..rest.len() - after.len()];
            let body = body.strip_suffix('}').unwrap_or(body);
            rest = after;
            if let (Some(env), Some(query)) = (env, at_media_query(&selector))
                && depth < MAX_MEDIA_NESTING
                && media_matches(query, env)
            {
                parse_rules_into(body, Some(env), depth + 1, sheet);
            }
            continue;
        }
        let Some(close) = rest.find('}') else {
            break;
        };
        let body = &rest[..close];
        rest = &rest[close + 1..];
        if selector.is_empty() {
            continue;
        }
        let decls = parse_declarations(body);
        for sel in selector.split(',') {
            let sel = sel.trim();
            if sel.is_empty() {
                continue;
            }
            sheet.rules.push(Rule {
                selector: sel.to_string(),
                parsed: parse_selector(sel),
                declarations: decls.clone(),
            });
        }
    }
}

/// The query text of an `@media` prelude, or `None` for other at-rules.
fn at_media_query(prelude: &str) -> Option<&str> {
    let p = prelude.trim();
    let head = p.get(..6)?;
    if !head.eq_ignore_ascii_case("@media") {
        return None;
    }
    let rest = &p[6..];
    // `@mediafoo` is not `@media`.
    if rest
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }
    Some(rest.trim())
}

/// Evaluate a media query list (Media Queries 4 subset) against `env`.
///
/// Supported: comma lists (any match), `not` and `only`, media types `all`,
/// `screen` (true) and `print` (false), `and`, and features `width`,
/// `height` (with `min-`/`max-` and range syntax such as `(width >= 40em)`),
/// `orientation`, and the preference features `prefers-color-scheme`
/// (always `light`), `prefers-reduced-motion` (`no-preference`),
/// `prefers-contrast` (`no-preference`), `hover`/`any-hover` (`hover`),
/// `pointer`/`any-pointer` (`fine`), `color` (true) and `monochrome`
/// (false). Anything else makes that query false, as the spec requires.
pub fn media_matches(query: &str, env: MediaEnv) -> bool {
    if query.len() > MAX_MEDIA_QUERY_BYTES {
        return false;
    }
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    split_top_level(q, ',')
        .into_iter()
        .any(|one| single_query(one.trim(), env))
}

fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

fn single_query(q: &str, env: MediaEnv) -> bool {
    if q.is_empty() {
        return false;
    }
    let lower = q.to_ascii_lowercase();
    let (negate, body) = if let Some(r) = lower.strip_prefix("not ") {
        (true, r.trim().to_string())
    } else if let Some(r) = lower.strip_prefix("only ") {
        (false, r.trim().to_string())
    } else {
        (false, lower)
    };
    let Some(result) = conjunction(&body, env) else {
        // Unknown syntax: the query is "not all", even under `not`.
        return false;
    };
    result != negate
}

/// `type and (f) and (g)` → Some(bool); unknown parts → None.
fn conjunction(body: &str, env: MediaEnv) -> Option<bool> {
    let mut all = true;
    let mut rest = body.trim();
    let mut first = true;
    while !rest.is_empty() {
        if !first {
            rest = rest.strip_prefix("and")?.trim_start();
        }
        first = false;
        if rest.starts_with('(') {
            let close = matching_paren(rest)?;
            let inner = &rest[1..close];
            all &= feature(inner.trim(), env)?;
            rest = rest[close + 1..].trim_start();
        } else {
            let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
            let word = &rest[..end];
            all &= match word {
                "all" | "screen" => true,
                "print" | "speech" | "tty" | "tv" | "projection" | "handheld" | "braille"
                | "embossed" | "aural" => false,
                _ => return None,
            };
            rest = rest[end..].trim_start();
        }
    }
    Some(all)
}

fn matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn media_length(v: &str) -> Option<f32> {
    let v = v.trim();
    if v == "0" {
        return Some(0.0);
    }
    let (num, mult) = if let Some(n) = v.strip_suffix("rem") {
        (n, MEDIA_EM_PX)
    } else if let Some(n) = v.strip_suffix("em") {
        (n, MEDIA_EM_PX)
    } else {
        (v.strip_suffix("px")?, 1.0)
    };
    let x: f32 = num.trim().parse().ok()?;
    x.is_finite().then_some(x * mult)
}

fn compare(actual: f32, op: &str, value: f32) -> Option<bool> {
    Some(match op {
        ">=" => actual >= value,
        "<=" => actual <= value,
        ">" => actual > value,
        "<" => actual < value,
        "=" => (actual - value).abs() < 0.01,
        _ => return None,
    })
}

fn dimension(name: &str, env: MediaEnv) -> Option<f32> {
    match name {
        "width" => Some(env.width),
        "height" => Some(env.height),
        _ => None,
    }
}

/// One `( … )` media feature.
fn feature(inner: &str, env: MediaEnv) -> Option<bool> {
    // Range syntax: `width >= 40em`, `40em <= width`, `400px < width < 800px`.
    for op in [">=", "<=", ">", "<", "="] {
        if inner.contains(op) && !inner.contains(':') {
            return range_feature(inner, env);
        }
    }
    let (name, value) = match inner.split_once(':') {
        Some((n, v)) => (n.trim(), Some(v.trim())),
        None => (inner.trim(), None),
    };
    if let Some(dim) = name.strip_prefix("min-").and_then(|n| dimension(n, env)) {
        return compare(dim, ">=", media_length(value?)?);
    }
    if let Some(dim) = name.strip_prefix("max-").and_then(|n| dimension(n, env)) {
        return compare(dim, "<=", media_length(value?)?);
    }
    if let Some(dim) = dimension(name, env) {
        return match value {
            Some(v) => compare(dim, "=", media_length(v)?),
            None => Some(dim > 0.0),
        };
    }
    let fixed = |want: &str| -> Option<bool> {
        match value {
            Some(v) => Some(v == want),
            None => Some(want != "none" && want != "no-preference"),
        }
    };
    match name {
        "orientation" => {
            let o = if env.height >= env.width {
                "portrait"
            } else {
                "landscape"
            };
            match value? {
                v @ ("portrait" | "landscape") => Some(v == o),
                _ => None,
            }
        }
        "prefers-color-scheme" => match value {
            Some(v @ ("light" | "dark")) => Some(v == "light"),
            Some(_) => None,
            None => Some(true),
        },
        "prefers-reduced-motion" | "prefers-contrast" | "prefers-reduced-transparency" => {
            fixed("no-preference")
        }
        "hover" | "any-hover" => fixed("hover"),
        "pointer" | "any-pointer" => fixed("fine"),
        "color" => Some(value.is_none_or(|v| v.parse::<u32>().is_ok_and(|n| n <= 8))),
        "monochrome" => Some(value.is_some_and(|v| v == "0")),
        "scripting" => fixed("none"),
        _ => None,
    }
}

fn range_feature(inner: &str, env: MediaEnv) -> Option<bool> {
    // Tokenise into operands and operators.
    let mut parts: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '<' | '>' | '=') {
            if !cur.trim().is_empty() {
                parts.push(cur.trim().to_string());
            }
            cur.clear();
            let mut op = c.to_string();
            if c != '=' && chars.peek() == Some(&'=') {
                op.push('=');
                chars.next();
            }
            parts.push(op);
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    let is_op = |s: &str| matches!(s, ">=" | "<=" | ">" | "<" | "=");
    fn flip(op: &str) -> &str {
        match op {
            ">=" => "<=",
            "<=" => ">=",
            ">" => "<",
            "<" => ">",
            o => o,
        }
    }
    match parts.as_slice() {
        [a, op, b] if is_op(op) => {
            if let Some(d) = dimension(a, env) {
                compare(d, op, media_length(b)?)
            } else {
                let d = dimension(b, env)?;
                compare(d, flip(op), media_length(a)?)
            }
        }
        [a, op1, name, op2, b] if is_op(op1) && is_op(op2) => {
            let d = dimension(name, env)?;
            Some(compare(d, flip(op1), media_length(a)?)? && compare(d, op2, media_length(b)?)?)
        }
        _ => None,
    }
}

pub fn parse_selector(input: &str) -> Option<Selector> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut parts = Vec::new();
    let mut pending = None;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
            if pending.is_none() && !parts.is_empty() {
                pending = Some(Combinator::Descendant);
            }
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'>' {
            pending = Some(Combinator::Child);
            i += 1;
            continue;
        }
        if bytes[i] == b'+' || bytes[i] == b'~' {
            return None;
        }
        let (compound, next) = parse_compound(s, i)?;
        parts.push((pending, compound));
        pending = None;
        i = next;
    }
    if parts.is_empty() {
        return None;
    }
    Some(Selector { parts })
}

fn parse_compound(s: &str, start: usize) -> Option<(Compound, usize)> {
    let bytes = s.as_bytes();
    let mut i = start;
    let mut simples = Vec::new();
    if i < bytes.len() && bytes[i] == b'*' {
        simples.push(Simple::Universal);
        i += 1;
    } else if i < bytes.len() && is_ident_start(bytes[i]) {
        let (name, n) = read_ident(s, i);
        simples.push(Simple::Type(name));
        i = n;
    }
    loop {
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b'.' => {
                i += 1;
                let (name, n) = read_ident(s, i);
                if name.is_empty() {
                    return None;
                }
                simples.push(Simple::Class(name));
                i = n;
            }
            b'#' => {
                i += 1;
                let (name, n) = read_ident(s, i);
                if name.is_empty() {
                    return None;
                }
                simples.push(Simple::Id(name));
                i = n;
            }
            b':' => {
                i += 1;
                if i < bytes.len() && bytes[i] == b':' {
                    return None;
                }
                let (name, n) = read_ident(s, i);
                if !matches!(name.as_str(), "link" | "visited" | "root") {
                    return None;
                }
                simples.push(Simple::Pseudo(name));
                i = n;
            }
            b'[' => return None,
            _ => break,
        }
    }
    if simples.is_empty() {
        return None;
    }
    Some((Compound { simples }, i))
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'-'
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn read_ident(s: &str, start: usize) -> (String, usize) {
    let bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() && is_ident(bytes[i]) {
        i += 1;
    }
    (s[start..i].to_ascii_lowercase(), i)
}

fn skip_balanced(input: &str) -> &str {
    let mut depth = 1usize;
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &input[i + 1..];
                }
            }
            _ => {}
        }
        i += 1;
    }
    ""
}

fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            out.push(' ');
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rule() {
        let s = parse_stylesheet("p { color: red; margin: 0 }");
        assert_eq!(s.rules.len(), 1);
        assert_eq!(s.rules[0].selector, "p");
        assert_eq!(s.rules[0].declarations[0].name, "color");
    }

    #[test]
    fn splits_comma_and_skips_comments() {
        let s = parse_stylesheet("/* ua */ h1, h2 { font-size: 24px }");
        assert_eq!(s.rules.len(), 2);
        assert_eq!(s.rules[0].selector, "h1");
        assert_eq!(s.rules[1].selector, "h2");
    }

    #[test]
    fn parses_class_id_descendant() {
        let s = parse_selector("article.post #x p.lead").unwrap();
        assert_eq!(s.parts.len(), 3);
        assert!(matches!(s.parts[1].0, Some(Combinator::Descendant)));
        assert!(
            s.parts[2]
                .1
                .simples
                .iter()
                .any(|p| matches!(p, Simple::Class(c) if c == "lead"))
        );
        assert!(parse_selector(".nav > a").is_some());
        let link = parse_selector("a:link").unwrap();
        assert!(
            link.parts[0]
                .1
                .simples
                .iter()
                .any(|s| matches!(s, Simple::Pseudo(p) if p == "link"))
        );
        assert!(parse_selector("a:hover").is_none());
    }

    fn env(w: f32, h: f32) -> MediaEnv {
        MediaEnv {
            width: w,
            height: h,
        }
    }

    #[test]
    fn media_blocks_apply_when_the_query_matches() {
        let css = "@media (min-width: 55em) { .table { display: table } } \
                   @media screen and (max-width: 600px) { .nav { display: none } } \
                   @media print { .ad { color: red } } .lead { color: red }";
        let wide = parse_stylesheet_media(css, env(1280.0, 800.0));
        let sel: Vec<&str> = wide.rules.iter().map(|r| r.selector.as_str()).collect();
        assert_eq!(sel, [".table", ".lead"]);
        let narrow = parse_stylesheet_media(css, env(400.0, 800.0));
        let sel: Vec<&str> = narrow.rules.iter().map(|r| r.selector.as_str()).collect();
        assert_eq!(sel, [".nav", ".lead"]);
        // Nested @media and @supports-style blocks.
        let nested = "@media screen { @media (min-width: 10px) { p { color: red } } } \
                      @supports (display: grid) { q { color: red } }";
        let s = parse_stylesheet_media(nested, env(800.0, 600.0));
        assert_eq!(s.rules.len(), 1);
        assert_eq!(s.rules[0].selector, "p");
    }

    #[test]
    fn media_query_features() {
        let e = env(800.0, 600.0);
        for (q, want) in [
            ("", true),
            ("all", true),
            ("screen", true),
            ("print", false),
            ("not print", true),
            ("only screen and (min-width: 50em)", true),
            ("(max-width: 799px)", false),
            ("(min-width: 40rem) and (max-width: 60rem)", true),
            ("print, (min-width: 100px)", true),
            ("(width >= 800px)", true),
            ("(width > 800px)", false),
            ("(400px < width <= 800px)", true),
            ("(30em <= width)", true),
            ("(orientation: landscape)", true),
            ("(orientation: portrait)", false),
            ("(prefers-color-scheme: dark)", false),
            ("(prefers-color-scheme: light)", true),
            ("(prefers-reduced-motion: reduce)", false),
            ("(prefers-reduced-motion)", false),
            ("(hover: hover) and (pointer: fine)", true),
            ("(min-resolution: 2dppx)", false),
            ("not (min-resolution: 2dppx)", false),
            ("(min-width: 10vw)", false),
            ("screen and", false),
            ("(width", false),
        ] {
            assert_eq!(media_matches(q, e), want, "{q}");
        }
        assert!(!media_matches(&"(min-width: 1px) and ".repeat(40), e));
    }

    #[test]
    fn skips_media_queries() {
        let s = parse_stylesheet(
            "@media (min-width: 55em) { .table { display: table } } .lead { color: red }",
        );
        assert!(s.rules.iter().any(|r| r.selector == ".lead"));
        assert!(!s.rules.iter().any(|r| r.selector.contains("@media")));
        assert!(!s.rules.iter().any(|r| r.selector == ".table"));
    }
}
