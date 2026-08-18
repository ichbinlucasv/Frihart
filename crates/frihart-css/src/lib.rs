//! CSS subset: declarations and rules. Unknown properties stay in the OM.

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
    pub declarations: Vec<Declaration>,
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
        for sel in selector.split(',') {
            let sel = sel.trim();
            if sel.is_empty() {
                continue;
            }
            sheet.rules.push(Rule {
                selector: sel.to_string(),
                declarations: parse_declarations(body),
            });
        }
    }
    sheet
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
}
