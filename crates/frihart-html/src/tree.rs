use crate::token::{Token, tokenize};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub text: Option<String>,
}

impl Node {
    pub fn is_text(&self) -> bool {
        self.text.is_some() && self.name.is_empty()
    }

    pub fn attr(&self, key: &str) -> Option<String> {
        let key = key.to_ascii_lowercase();
        self.attrs
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.clone())
    }

    pub fn text_content(&self) -> String {
        if let Some(t) = &self.text {
            return collapse_ws(t);
        }
        let mut s = String::new();
        for child in &self.children {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&child.text_content());
        }
        collapse_ws(&s)
    }

    pub fn direct_text(&self) -> String {
        let mut s = String::new();
        for child in &self.children {
            if let Some(t) = &child.text {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(t);
            }
        }
        collapse_ws(&s)
    }

    pub fn has_child_named(&self, name: &str) -> bool {
        self.children.iter().any(|c| c.name == name)
    }
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

pub fn parse(input: &str) -> Node {
    let tokens = tokenize(input);
    let root = Node {
        name: "document".into(),
        ..Node::default()
    };
    let mut stack: Vec<Node> = vec![root];
    for token in tokens {
        match token {
            Token::Start {
                name,
                attrs,
                self_closing,
            } => {
                let node = Node {
                    name: name.clone(),
                    attrs,
                    ..Node::default()
                };
                if self_closing || VOID.contains(&name.as_str()) {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                } else {
                    stack.push(node);
                }
            }
            Token::End { name } => {
                while stack.len() > 1 {
                    let finished = stack.pop().expect("stack");
                    let matched = finished.name == name;
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(finished);
                    }
                    if matched {
                        break;
                    }
                }
            }
            Token::Text(t) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node {
                        text: Some(t),
                        ..Node::default()
                    });
                }
            }
        }
    }
    while stack.len() > 1 {
        let finished = stack.pop().expect("stack");
        if let Some(parent) = stack.last_mut() {
            parent.children.push(finished);
        }
    }
    stack.pop().unwrap_or_default()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nests_and_void() {
        let n = parse("<div><img src=\"a\"><p>Hi</p></div>");
        assert_eq!(n.name, "document");
        let div = &n.children[0];
        assert_eq!(div.name, "div");
        assert_eq!(div.children[0].name, "img");
        assert_eq!(div.children[1].name, "p");
        assert_eq!(div.children[1].text_content(), "Hi");
    }
}
