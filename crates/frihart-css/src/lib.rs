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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttrOp {
    /// `[attr]` — the attribute is present (value ignored).
    Present,
    /// `[attr=value]`
    Exact,
    /// `[attr~=value]` — one whitespace-separated word.
    Word,
    /// `[attr|=value]` — exact, or a prefix followed by `-`.
    Dash,
    /// `[attr^=value]`
    Prefix,
    /// `[attr$=value]`
    Suffix,
    /// `[attr*=value]`
    Substring,
}

/// One attribute selector. `name` is stored lowercased (HTML names are
/// ASCII case-insensitive). `value` keeps the case from the stylesheet;
/// [`AttrTest::insensitive`] is the `i` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttrTest {
    pub name: String,
    pub op: AttrOp,
    pub value: String,
    pub insensitive: bool,
}

/// `An+B` as in `:nth-child(2n+1)`. `a == 0` is a single index (`:nth-child(3)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnB {
    pub a: i32,
    pub b: i32,
}

impl AnB {
    /// `position` is 1-based, the way CSS counts children.
    pub fn matches_position(self, position: u32) -> bool {
        if position == 0 {
            return false;
        }
        let pos = i64::from(position);
        let a = i64::from(self.a);
        let b = i64::from(self.b);
        if a == 0 {
            return pos == b;
        }
        let diff = pos - b;
        diff.rem_euclid(a) == 0 && diff.div_euclid(a) >= 0
    }

    /// `index` is 0-based among the siblings the pseudo counts.
    pub fn matches_index(self, index: u32) -> bool {
        self.matches_position(index.saturating_add(1))
    }
}

/// A pseudo-class this subset accepts. Interactive ones (`:hover`,
/// `:focus`, `:active`) stay out: there is no input state on the
/// static document. `:empty`, `:is` and `:where` stay out too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pseudo {
    Link,
    Visited,
    Root,
    FirstChild,
    LastChild,
    OnlyChild,
    FirstOfType,
    LastOfType,
    OnlyOfType,
    NthChild(AnB),
    NthLastChild(AnB),
    NthOfType(AnB),
    NthLastOfType(AnB),
    /// Selectors Level 3: exactly one simple selector, not another `:not`.
    Not(Box<Simple>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Simple {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    Pseudo(Pseudo),
    Attr(AttrTest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compound {
    pub simples: Vec<Simple>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    /// `+` — the immediate preceding element sibling.
    Adjacent,
    /// `~` — a preceding element sibling.
    General,
}

/// Selectors longer than this are ignored (the rule does not apply).
pub const MAX_SELECTOR_BYTES: usize = 1024;
/// Compounds in one selector. Longer chains are ignored.
pub const MAX_SELECTOR_COMPOUNDS: usize = 32;

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
            } else if let (Some(env), Some(cond)) = (env, at_rule_prelude(&selector, "@supports"))
                && depth < MAX_MEDIA_NESTING
                && supports_matches(cond)
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
    at_rule_prelude(prelude, "@media")
}

/// The text after `name` (for example `@supports`) in an at-rule prelude,
/// or `None` when the prelude is another at-rule.
fn at_rule_prelude<'a>(prelude: &'a str, name: &str) -> Option<&'a str> {
    let p = prelude.trim();
    let head = p.get(..name.len())?;
    if !head.eq_ignore_ascii_case(name) {
        return None;
    }
    let rest = &p[name.len()..];
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

/// Declarations the own engine applies, for `@supports`. `None` accepts any
/// value of that property; `Some(list)` only those keywords. Kept in step
/// with `frihart-style` so a page never takes a branch we cannot render.
const SUPPORTED_DECLARATIONS: &[(&str, Option<&[&str]>)] = &[
    ("color", None),
    ("background-color", None),
    ("background", None),
    ("display", Some(&["none", "inline", "block"])),
    ("font-size", None),
    ("font-weight", None),
    ("font-family", None),
    ("list-style", None),
    ("list-style-type", None),
    (
        "white-space",
        Some(&["normal", "nowrap", "pre", "pre-wrap", "pre-line"]),
    ),
    ("margin", None),
    ("margin-top", None),
    ("margin-bottom", None),
    ("padding", None),
    ("width", None),
    ("max-width", None),
    ("height", None),
    ("border", None),
    ("border-width", None),
    ("border-color", None),
    ("line-height", None),
    (
        "text-align",
        Some(&["left", "start", "center", "right", "end"]),
    ),
];

/// Longest `@supports` condition evaluated, in bytes; longer ones are false.
pub const MAX_SUPPORTS_BYTES: usize = 512;

/// Evaluate an `@supports` condition (CSS Conditional 3 plus `selector()`).
///
/// `(prop: value)` is true when the own engine applies that declaration
/// (see the supported list above; `display: grid` and `flex` are false
/// today). `selector(...)` is true when [`parse_selector`] accepts it.
/// `not`, `and` and `or` combine parenthesised conditions; mixing `and` and
/// `or` at one level without parentheses is invalid and false, as the spec
/// requires. Unknown functions are false.
pub fn supports_matches(cond: &str) -> bool {
    if cond.len() > MAX_SUPPORTS_BYTES {
        return false;
    }
    supports_condition(cond.trim(), 0).unwrap_or(false)
}

fn supports_condition(c: &str, depth: usize) -> Option<bool> {
    if depth > 8 || c.is_empty() {
        return None;
    }
    if let Some(rest) = strip_keyword(c, "not") {
        return supports_in_parens(rest.trim(), depth).map(|b| !b);
    }
    let and_parts = split_keyword(c, "and");
    let or_parts = split_keyword(c, "or");
    match (and_parts.len() > 1, or_parts.len() > 1) {
        (true, true) => None,
        (true, false) => {
            let mut all = true;
            for p in and_parts {
                all &= supports_in_parens(p.trim(), depth)?;
            }
            Some(all)
        }
        (false, true) => {
            let mut any = false;
            for p in or_parts {
                any |= supports_in_parens(p.trim(), depth)?;
            }
            Some(any)
        }
        (false, false) => supports_in_parens(c, depth),
    }
}

/// `( condition )`, `( prop: value )` or `selector( ... )`.
fn supports_in_parens(c: &str, depth: usize) -> Option<bool> {
    let c = c.trim();
    if let Some(inner) = c
        .get(..9)
        .filter(|h| h.eq_ignore_ascii_case("selector("))
        .and_then(|_| c[9..].strip_suffix(')'))
    {
        return Some(parse_selector(inner.trim()).is_some());
    }
    let inner = c.strip_prefix('(')?.strip_suffix(')')?;
    // The outer parens must enclose everything: "(a) and (b)" is not one group.
    if !balanced(inner) {
        return None;
    }
    let inner = inner.trim();
    if inner.starts_with('(')
        || strip_keyword(inner, "not").is_some()
        || inner
            .get(..9)
            .is_some_and(|h| h.eq_ignore_ascii_case("selector("))
    {
        return supports_condition(inner, depth + 1);
    }
    let (name, value) = inner.split_once(':')?;
    let name = name.trim().to_ascii_lowercase();
    let value = value
        .trim()
        .trim_end_matches("!important")
        .trim()
        .to_ascii_lowercase();
    if value.is_empty() {
        return Some(false);
    }
    Some(SUPPORTED_DECLARATIONS.iter().any(|(n, allowed)| {
        *n == name && allowed.is_none_or(|list| list.contains(&value.as_str()))
    }))
}

fn balanced(s: &str) -> bool {
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// `rest` after a leading keyword followed by whitespace or `(`.
fn strip_keyword<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    let head = s.get(..kw.len())?;
    if !head.eq_ignore_ascii_case(kw) {
        return None;
    }
    let rest = &s[kw.len()..];
    rest.starts_with(|c: char| c.is_whitespace() || c == '(')
        .then_some(rest)
}

/// Split on a keyword (` and ` / ` or `) at parenthesis depth 0.
fn split_keyword<'a>(s: &'a str, kw: &str) -> Vec<&'a str> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b if depth == 0 && b.is_ascii_whitespace() => {
                let after = &s[i + 1..];
                let trimmed = after.trim_start();
                if let Some(rest) = strip_keyword(trimmed, kw) {
                    out.push(&s[start..i]);
                    let consumed = s.len() - rest.len();
                    start = consumed;
                    i = consumed;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    out.push(&s[start..]);
    out
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
    if s.is_empty() || s.len() > MAX_SELECTOR_BYTES {
        return None;
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut parts = Vec::new();
    // `pending` is a combinator already seen. Whitespace alone is not one
    // until the next compound, so `p > b` and `p  b` both stay valid.
    let mut pending: Option<Combinator> = None;
    let mut saw_ws = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            saw_ws = true;
            i += 1;
            continue;
        }
        if matches!(bytes[i], b'>' | b'+' | b'~') {
            // A second combinator (`p + > b`, `>>`) or a leading one is invalid.
            if pending.is_some() || parts.is_empty() {
                return None;
            }
            pending = Some(match bytes[i] {
                b'>' => Combinator::Child,
                b'+' => Combinator::Adjacent,
                b'~' => Combinator::General,
                _ => return None,
            });
            saw_ws = false;
            i += 1;
            continue;
        }
        if !parts.is_empty() && pending.is_none() {
            if !saw_ws {
                return None;
            }
            pending = Some(Combinator::Descendant);
        }
        let (compound, next) = parse_compound(s, i)?;
        if parts.len() >= MAX_SELECTOR_COMPOUNDS {
            return None;
        }
        parts.push((pending, compound));
        pending = None;
        saw_ws = false;
        i = next;
    }
    // A trailing combinator (`p +`) is not a selector.
    if parts.is_empty() || pending.is_some() {
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
                if name.is_empty() {
                    return None;
                }
                i = n;
                let pseudo = if i < bytes.len() && bytes[i] == b'(' {
                    let (inner, n) = read_function(s, i)?;
                    i = n;
                    parse_functional_pseudo(&name, inner)?
                } else {
                    parse_pseudo_ident(&name)?
                };
                simples.push(Simple::Pseudo(pseudo));
            }
            b'[' => {
                let (attr, n) = parse_attr(s, i)?;
                simples.push(attr);
                i = n;
            }
            _ => break,
        }
    }
    if simples.is_empty() {
        return None;
    }
    Some((Compound { simples }, i))
}

fn parse_pseudo_ident(name: &str) -> Option<Pseudo> {
    Some(match name {
        "link" => Pseudo::Link,
        "visited" => Pseudo::Visited,
        "root" => Pseudo::Root,
        "first-child" => Pseudo::FirstChild,
        "last-child" => Pseudo::LastChild,
        "only-child" => Pseudo::OnlyChild,
        "first-of-type" => Pseudo::FirstOfType,
        "last-of-type" => Pseudo::LastOfType,
        "only-of-type" => Pseudo::OnlyOfType,
        _ => return None,
    })
}

fn parse_functional_pseudo(name: &str, inner: &str) -> Option<Pseudo> {
    match name {
        "nth-child" => Some(Pseudo::NthChild(parse_an_b(inner)?)),
        "nth-last-child" => Some(Pseudo::NthLastChild(parse_an_b(inner)?)),
        "nth-of-type" => Some(Pseudo::NthOfType(parse_an_b(inner)?)),
        "nth-last-of-type" => Some(Pseudo::NthLastOfType(parse_an_b(inner)?)),
        "not" => Some(Pseudo::Not(Box::new(parse_not_argument(inner)?))),
        _ => None,
    }
}

/// One simple selector. A compound (`p.a`), a combinator, or a nested
/// `:not` makes the whole selector invalid so the rule is ignored.
fn parse_not_argument(inner: &str) -> Option<Simple> {
    let bytes = inner.as_bytes();
    let start = skip_ws(bytes, 0);
    if start >= bytes.len() {
        return None;
    }
    let (compound, next) = parse_compound(inner, start)?;
    if !inner[next..].trim().is_empty() || compound.simples.len() != 1 {
        return None;
    }
    let simple = compound.simples.into_iter().next()?;
    if matches!(simple, Simple::Pseudo(Pseudo::Not(_))) {
        return None;
    }
    Some(simple)
}

/// `open` points at `(`. Returns the interior and the index after `)`.
/// Quotes hide parentheses so `:not([title=")"])` stays one argument.
fn read_function(s: &str, open: usize) -> Option<(&str, usize)> {
    let bytes = s.as_bytes();
    if open >= bytes.len() || bytes[open] != b'(' {
        return None;
    }
    let mut i = open + 1;
    let start = i;
    let mut depth = 1u32;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == b'\\' {
                i += 1;
                if i < bytes.len() {
                    i += 1;
                }
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'\'' | b'"' => {
                quote = Some(c);
                i += 1;
            }
            b'(' => {
                depth = depth.checked_add(1)?;
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[start..i], i + 1));
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// CSS An+B microsyntax. Whitespace is allowed around the `B` sign
/// (`2n + 1`) and nowhere else (`2 n` is rejected, not repaired).
fn parse_an_b(input: &str) -> Option<AnB> {
    let s = input.trim();
    if s.is_empty() || s.len() > 64 {
        return None;
    }
    if s.eq_ignore_ascii_case("odd") {
        return Some(AnB { a: 2, b: 1 });
    }
    if s.eq_ignore_ascii_case("even") {
        return Some(AnB { a: 2, b: 0 });
    }
    if !s.bytes().all(|b| {
        b.is_ascii_whitespace() || b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'n' | b'N')
    }) {
        return None;
    }
    let lower: String = s.chars().map(|c| c.to_ascii_lowercase()).collect();
    let Some(n_pos) = lower.find('n') else {
        return Some(AnB {
            a: 0,
            b: parse_css_int(&lower)?,
        });
    };
    if lower[n_pos + 1..].contains('n') {
        return None;
    }
    let a_raw = &lower[..n_pos];
    if a_raw.chars().any(char::is_whitespace) {
        return None;
    }
    let a = match a_raw {
        "" | "+" => 1,
        "-" => -1,
        other => parse_css_int(other)?,
    };
    let b_raw = lower[n_pos + 1..].trim();
    let b = if b_raw.is_empty() {
        0
    } else {
        parse_b_term(b_raw)?
    };
    Some(AnB { a, b })
}

/// The `B` of `An+B`: an operator sign, optional whitespace, then an
/// integer. `2n+-1` is `+` applied to `-1`. A space inside that integer
/// (`2n+ - 1`) is rejected.
fn parse_b_term(s: &str) -> Option<i32> {
    let s = s.trim();
    let mut chars = s.chars();
    let sign = chars.next()?;
    if sign != '+' && sign != '-' {
        return None;
    }
    let rest: String = chars.collect();
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (neg_mag, digits) = if let Some(stripped) = rest.strip_prefix('+') {
        (false, stripped)
    } else if let Some(stripped) = rest.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, rest)
    };
    if digits.is_empty() || digits.chars().any(|c| !c.is_ascii_digit()) {
        return None;
    }
    let mag = parse_css_int(digits)?;
    let value = if neg_mag { mag.checked_neg()? } else { mag };
    if sign == '-' {
        value.checked_neg()
    } else {
        Some(value)
    }
}

fn parse_css_int(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (neg, digits) = if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    };
    if digits.is_empty()
        || digits.chars().any(char::is_whitespace)
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    if digits.len() > 10 {
        return None;
    }
    let mag: i64 = digits.parse().ok()?;
    let signed = if neg { -mag } else { mag };
    i32::try_from(signed).ok()
}

/// `[attr]`, `[attr=value]`, and the `~=` `|=` `^=` `$=` `*=` matchers.
///
/// A namespace (`*|href`, `svg|a`) is rejected: the whole selector is
/// ignored. A backslash in a value is rejected too — hex escapes are not
/// decoded, so the rule is ignored instead of matched against a guessed
/// string. The `i` flag compares values ASCII-case-insensitively; `s`
/// (and the default) keeps the stylesheet's case.
fn parse_attr(s: &str, start: usize) -> Option<(Simple, usize)> {
    let bytes = s.as_bytes();
    if start >= bytes.len() || bytes[start] != b'[' {
        return None;
    }
    let mut i = skip_ws(bytes, start + 1);
    let (name, n) = read_ident(s, i);
    if name.is_empty() {
        return None;
    }
    i = skip_ws(bytes, n);
    if i >= bytes.len() {
        return None;
    }
    if bytes[i] == b']' {
        return Some((
            Simple::Attr(AttrTest {
                name,
                op: AttrOp::Present,
                value: String::new(),
                insensitive: false,
            }),
            i + 1,
        ));
    }
    // `ns|attr` / `*|attr` — not this subset. `|=` is the dash matcher.
    if bytes[i] == b'|' && (i + 1 >= bytes.len() || bytes[i + 1] != b'=') {
        return None;
    }
    let (op, n) = parse_attr_op(bytes, i)?;
    i = skip_ws(bytes, n);
    let (value, n) = read_attr_value(s, i)?;
    i = skip_ws(bytes, n);
    let mut insensitive = false;
    if i < bytes.len() && is_ident_start(bytes[i]) {
        let (flag, n) = read_ident(s, i);
        match flag.as_str() {
            "i" => insensitive = true,
            "s" => {}
            _ => return None,
        }
        i = skip_ws(bytes, n);
    }
    if i >= bytes.len() || bytes[i] != b']' {
        return None;
    }
    Some((
        Simple::Attr(AttrTest {
            name,
            op,
            value,
            insensitive,
        }),
        i + 1,
    ))
}

fn parse_attr_op(bytes: &[u8], i: usize) -> Option<(AttrOp, usize)> {
    if i >= bytes.len() {
        return None;
    }
    let (op, width) = match bytes[i] {
        b'=' => (AttrOp::Exact, 1),
        b'~' => (AttrOp::Word, 2),
        b'|' => (AttrOp::Dash, 2),
        b'^' => (AttrOp::Prefix, 2),
        b'$' => (AttrOp::Suffix, 2),
        b'*' => (AttrOp::Substring, 2),
        _ => return None,
    };
    if width == 2 && (i + 1 >= bytes.len() || bytes[i + 1] != b'=') {
        return None;
    }
    Some((op, i + width))
}

/// A quoted string (case kept, no backslash escapes) or an unquoted ident.
fn read_attr_value(s: &str, start: usize) -> Option<(String, usize)> {
    let bytes = s.as_bytes();
    if start >= bytes.len() {
        return None;
    }
    if bytes[start] == b'"' || bytes[start] == b'\'' {
        let q = bytes[start];
        let mut i = start + 1;
        let mut out = String::new();
        while i < bytes.len() && bytes[i] != q {
            if bytes[i] == b'\\' {
                return None;
            }
            let ch = s[i..].chars().next()?;
            out.push(ch);
            i += ch.len_utf8();
        }
        if i >= bytes.len() || bytes[i] != q {
            return None;
        }
        return Some((out, i + 1));
    }
    if !is_ident_start(bytes[start]) {
        return None;
    }
    let mut i = start + 1;
    while i < bytes.len() && is_ident(bytes[i]) {
        i += 1;
    }
    Some((s[start..i].to_string(), i))
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
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
                .any(|s| matches!(s, Simple::Pseudo(Pseudo::Link)))
        );
        assert!(parse_selector("a:hover").is_none());
        assert!(parse_selector("h1 + p").is_some());
        assert!(parse_selector("h1~p").is_some());
        assert!(parse_selector("h1 + > p").is_none());
        assert!(parse_selector("+ p").is_none());
        assert!(parse_selector("p +").is_none());
        assert!(parse_selector("p++p").is_none());
        assert!(parse_selector("a[href]").is_some());
        assert!(parse_selector("a[*|href]").is_none());
        assert!(parse_selector(r#"a[title="\41"]"#).is_none());
    }

    #[test]
    fn parses_attribute_selectors() {
        let present = parse_selector("a[href]").unwrap();
        assert!(matches!(
            &present.parts[0].1.simples[1],
            Simple::Attr(a) if a.name == "href" && a.op == AttrOp::Present
        ));
        let both = parse_selector(r#"a[ HREF ^= "https" i ][rel~=me]"#).unwrap();
        let simples = &both.parts[0].1.simples;
        assert!(matches!(
            &simples[1],
            Simple::Attr(a)
                if a.name == "href"
                    && a.op == AttrOp::Prefix
                    && a.value == "https"
                    && a.insensitive
        ));
        assert!(matches!(
            &simples[2],
            Simple::Attr(a)
                if a.name == "rel" && a.op == AttrOp::Word && a.value == "me" && !a.insensitive
        ));
        for (sel, op, value, insensitive) in [
            (r#"[type="TEXT" s]"#, AttrOp::Exact, "TEXT", false),
            ("[lang|=en]", AttrOp::Dash, "en", false),
            (r#"[href$=".pdf"]"#, AttrOp::Suffix, ".pdf", false),
            ("[title*=Hello]", AttrOp::Substring, "Hello", false),
        ] {
            let parsed = parse_selector(sel).unwrap();
            assert!(
                matches!(
                    &parsed.parts[0].1.simples[0],
                    Simple::Attr(a)
                        if a.op == op && a.value == value && a.insensitive == insensitive
                ),
                "{sel}"
            );
        }
        assert!(parse_selector("a[href=]").is_none());
        assert!(parse_selector("a[href=\"unterminated]").is_none());
        assert!(parse_selector(r#"a[href="x" z]"#).is_none());
        assert!(parse_selector("[*|href]").is_none());
        assert!(parse_selector("[]").is_none());
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
    fn supports_conditions() {
        for (c, want) in [
            ("(display: block)", true),
            ("(display: grid)", false),
            ("(display: flex)", false),
            ("(color: red)", true),
            ("(COLOR: Red)", true),
            ("(gap: 1em)", false),
            ("(color:)", false),
            ("not (display: grid)", true),
            ("not (color: red)", false),
            ("(color: red) and (display: block)", true),
            ("(color: red) and (display: grid)", false),
            ("(display: grid) or (display: block)", true),
            ("(display: grid) or (gap: 1em)", false),
            ("(color: red) and (width: 1px) or (gap: 0)", false),
            ("((display: grid) or (color: red)) and (width: 1px)", true),
            ("selector(.a > b)", true),
            ("selector(h1 + p)", true),
            ("selector(h1 ~ p)", true),
            ("selector(h1 + > p)", false),
            ("selector(+ h1)", false),
            ("selector(a[href])", true),
            ("selector(a[href^=https])", true),
            ("selector(a[*|href])", false),
            ("selector(:first-child)", true),
            ("selector(:nth-child(2n+1))", true),
            ("selector(:nth-last-of-type(odd))", true),
            ("selector(:not(p))", true),
            ("selector(:not())", false),
            ("selector(:not(:not(p)))", false),
            ("selector(:has(a))", false),
            ("font-tech(color-COLRv1)", false),
            ("", false),
            ("color: red", false),
        ] {
            assert_eq!(supports_matches(c), want, "{c}");
        }
        assert!(!supports_matches(&format!("({})", "(".repeat(600))));
    }

    #[test]
    fn supports_blocks_apply_in_media_mode() {
        let css = "@supports (display: grid) { .grid { color: red } } \
                   @supports not (display: grid) { .fallback { color: red } } \
                   @supports (color: red) { @media (min-width: 10px) { .both { color: red } } } \
                   @supportsx (color: red) { .bogus { color: red } }";
        let s = parse_stylesheet_media(css, env(800.0, 600.0));
        let sel: Vec<&str> = s.rules.iter().map(|r| r.selector.as_str()).collect();
        assert_eq!(sel, [".fallback", ".both"]);
        // The plain parser still skips every at-rule.
        assert!(parse_stylesheet(css).rules.is_empty());
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
    fn parses_structural_pseudos_and_not() {
        let odd = parse_selector("li:nth-child(odd)").unwrap();
        assert!(matches!(
            &odd.parts[0].1.simples[1],
            Simple::Pseudo(Pseudo::NthChild(AnB { a: 2, b: 1 }))
        ));
        let func = parse_selector("h2:nth-of-type( 2n + 1 )").unwrap();
        assert!(matches!(
            &func.parts[0].1.simples[1],
            Simple::Pseudo(Pseudo::NthOfType(AnB { a: 2, b: 1 }))
        ));
        let neg = parse_selector("p:nth-last-child(-n+3)").unwrap();
        assert!(matches!(
            &neg.parts[0].1.simples[1],
            Simple::Pseudo(Pseudo::NthLastChild(AnB { a: -1, b: 3 }))
        ));
        let not_attr = parse_selector(r#"a:not([title=")"])"#).unwrap();
        assert!(matches!(
            &not_attr.parts[0].1.simples[1],
            Simple::Pseudo(Pseudo::Not(inner))
                if matches!(inner.as_ref(), Simple::Attr(a) if a.value == ")")
        ));
        assert!(parse_selector("p:first-child").is_some());
        assert!(parse_selector("p:last-of-type").is_some());
        assert!(parse_selector("p:only-child").is_some());
        assert!(parse_selector(":not( :link )").is_some());
        assert!(parse_selector("p:nth-child(2n+-1)").is_some());
        // A compound, a combinator, or a nested :not is not one simple.
        assert!(parse_selector(":not(.a.b)").is_none());
        assert!(parse_selector(":not(p > a)").is_none());
        assert!(parse_selector(":not(:not(p))").is_none());
        assert!(parse_selector("p:nth-child()").is_none());
        assert!(parse_selector("p:nth-child(2 n)").is_none());
        assert!(parse_selector("p:nth-child(2n+)").is_none());
        assert!(parse_selector("p:nth-child(2n+1 of p)").is_none());
        assert!(parse_selector("p:hover").is_none());
        assert!(parse_selector("p:empty").is_none());
        assert!(parse_selector("p::before").is_none());
    }

    #[test]
    fn an_b_matches_one_based_positions() {
        let odd = AnB { a: 2, b: 1 };
        assert!(odd.matches_position(1) && odd.matches_position(5));
        assert!(!odd.matches_position(2) && !odd.matches_position(0));
        let even = parse_an_b("even").unwrap();
        assert!(even.matches_index(1) && !even.matches_index(0));
        let third = parse_an_b("3").unwrap();
        assert_eq!(third, AnB { a: 0, b: 3 });
        assert!(third.matches_position(3) && !third.matches_position(1));
        let from_end = parse_an_b("-n+3").unwrap();
        assert!(from_end.matches_position(1) && from_end.matches_position(3));
        assert!(!from_end.matches_position(4));
        assert_eq!(parse_an_b("2N + 1"), Some(AnB { a: 2, b: 1 }));
        assert_eq!(parse_an_b("+n"), Some(AnB { a: 1, b: 0 }));
        assert_eq!(parse_an_b("-n"), Some(AnB { a: -1, b: 0 }));
        assert_eq!(parse_an_b("2n+-1"), Some(AnB { a: 2, b: -1 }));
        assert_eq!(parse_an_b("+5"), Some(AnB { a: 0, b: 5 }));
        assert!(parse_an_b("+ 5").is_none());
        assert!(parse_an_b("2 n+1").is_none());
        assert!(parse_an_b("2n+ - 1").is_none());
        assert!(parse_an_b("99999999999").is_none());
        assert!(parse_an_b("").is_none());
        assert!(!parse_an_b("0").unwrap().matches_position(1));
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
