#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Start {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    End {
        name: String,
    },
    Text(String),
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let mut raw = String::new();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            flush_text(&mut raw, &mut out);
            if starts_with_ci(&bytes[i..], b"<!--") {
                i = skip_until(bytes, i + 4, b"-->");
                continue;
            }
            if starts_with_ci(&bytes[i..], b"<!doctype") {
                i = skip_until(bytes, i + 2, b">");
                continue;
            }
            if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                i += 2;
                let (name, next) = read_name(bytes, i);
                i = skip_until(bytes, next, b">");
                if !name.is_empty() {
                    out.push(Token::End { name });
                }
                continue;
            }
            i += 1;
            let (name, next) = read_name(bytes, i);
            i = next;
            if name.is_empty() {
                raw.push('<');
                continue;
            }
            let (attrs, self_closing, next) = read_attrs(bytes, i);
            i = next;
            if name == "script" || name == "style" {
                out.push(Token::Start {
                    name: name.clone(),
                    attrs,
                    self_closing,
                });
                if !self_closing {
                    i = find_end_tag(bytes, i, &name);
                    out.push(Token::End { name });
                }
                continue;
            }
            out.push(Token::Start {
                name,
                attrs,
                self_closing,
            });
            continue;
        }
        raw.push(bytes[i] as char);
        i += 1;
    }
    flush_text(&mut raw, &mut out);
    out
}

fn flush_text(raw: &mut String, out: &mut Vec<Token>) {
    if raw.is_empty() {
        return;
    }
    let decoded = decode_entities(raw);
    if !decoded.trim().is_empty() {
        out.push(Token::Text(decoded));
    }
    raw.clear();
}

fn read_name(bytes: &[u8], mut i: usize) -> (String, usize) {
    let start = i;
    while i < bytes.len() && is_name_char(bytes[i]) {
        i += 1;
    }
    (
        String::from_utf8_lossy(&bytes[start..i]).to_ascii_lowercase(),
        i,
    )
}

fn read_attrs(bytes: &[u8], mut i: usize) -> (Vec<(String, String)>, bool, usize) {
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'>' {
            i += 1;
            break;
        }
        if bytes[i] == b'/' {
            self_closing = true;
            i += 1;
            continue;
        }
        let (name, next) = read_name(bytes, i);
        i = next;
        if name.is_empty() {
            i += 1;
            continue;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let q = bytes[i];
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != q {
                    i += 1;
                }
                value = String::from_utf8_lossy(&bytes[start..i]).into_owned();
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                    i += 1;
                }
                value = String::from_utf8_lossy(&bytes[start..i]).into_owned();
            }
        }
        attrs.push((name, decode_entities(&value)));
    }
    (attrs, self_closing, i)
}

fn find_end_tag(bytes: &[u8], start: usize, name: &str) -> usize {
    let needle = format!("</{name}");
    let n = needle.as_bytes();
    let mut i = start;
    while i + n.len() <= bytes.len() {
        if eq_ci(&bytes[i..i + n.len()], n) {
            return skip_until(bytes, i, b">");
        }
        i += 1;
    }
    bytes.len()
}

fn skip_until(bytes: &[u8], mut i: usize, end: &[u8]) -> usize {
    while i + end.len() <= bytes.len() {
        if &bytes[i..i + end.len()] == end {
            return i + end.len();
        }
        i += 1;
    }
    bytes.len()
}

fn starts_with_ci(hay: &[u8], needle: &[u8]) -> bool {
    hay.len() >= needle.len() && eq_ci(&hay[..needle.len()], needle)
}

fn eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.eq_ignore_ascii_case(y))
}

fn is_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b':'
}

fn decode_entities(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            if let Some(rel) = chars[i + 1..].iter().position(|&c| c == ';') {
                let ent: String = chars[i + 1..i + 1 + rel].iter().collect();
                if let Some(c) = map_entity(&ent) {
                    out.push(c);
                    i += rel + 2;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn map_entity(ent: &str) -> Option<char> {
    match ent {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        other => {
            if let Some(hex) = other
                .strip_prefix("#x")
                .or_else(|| other.strip_prefix("#X"))
            {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else if let Some(num) = other.strip_prefix('#') {
                num.parse::<u32>().ok().and_then(char::from_u32)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_basic() {
        let t = tokenize("<p class=\"x\">a&amp;b</p>");
        assert!(matches!(&t[0], Token::Start { name, .. } if name == "p"));
        assert!(matches!(&t[1], Token::Text(s) if s.contains('&')));
        assert!(matches!(&t[2], Token::End { name } if name == "p"));
    }
}
