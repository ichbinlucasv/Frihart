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
            let name = n.trim().to_ascii_lowercase();
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

pub fn parse_stylesheet(input: &str) -> Stylesheet {
    let input = strip_comments(input);
    let mut sheet = Stylesheet::default();
    let mut rest = input.as_str();
    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim().to_string();
        rest = &rest[open + 1..];
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
    sheet
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
                if !matches!(name.as_str(), "link" | "visited") {
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
}
