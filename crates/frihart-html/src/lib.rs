//! HTML subset: tokenize, tree, forms.

#![forbid(unsafe_code)]

mod token;
mod tree;

pub use token::{Token, tokenize};
pub use tree::{Node, parse, serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormField {
    pub name: String,
    pub id: String,
    pub input_type: String,
    pub autocomplete: String,
    pub label: String,
}

pub fn document_title(root: &Node) -> String {
    if let Some(t) = find_text(root, "title") {
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if !t.is_empty() {
            return t;
        }
    }
    String::new()
}

/// Concatenate every `<style>` element's text. Author CSS for the pipeline.
pub fn author_css(root: &Node) -> String {
    let mut out = String::new();
    collect_style(root, &mut out);
    out
}

fn collect_style(node: &Node, out: &mut String) {
    if node.name == "style" {
        for child in &node.children {
            if let Some(t) = &child.text {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(t);
            }
        }
        return;
    }
    for child in &node.children {
        collect_style(child, out);
    }
}

pub fn first_form(root: &Node) -> (String, String) {
    if let Some((a, m)) = find_form(root) {
        return (a, m);
    }
    (String::new(), "get".into())
}

fn find_form(node: &Node) -> Option<(String, String)> {
    if node.name == "form" {
        return Some((
            node.attr("action").unwrap_or_default(),
            node.attr("method").unwrap_or_else(|| "get".into()),
        ));
    }
    for child in &node.children {
        if let Some(found) = find_form(child) {
            return Some(found);
        }
    }
    None
}

pub fn visible_blocks(root: &Node) -> Vec<Block> {
    let mut out = Vec::new();
    walk_blocks(root, &mut out);
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, String),
    Text(String),
    Link { text: String, href: String },
    Field(FormField),
}

fn walk_blocks(node: &Node, out: &mut Vec<Block>) {
    if node.is_text() {
        return;
    }
    let name = node.name.as_str();
    if matches!(name, "script" | "style" | "noscript" | "head") {
        return;
    }
    if let Some(level) = heading_level(name) {
        let t = node.text_content();
        if !t.is_empty() {
            out.push(Block::Heading(level, t));
        }
        return;
    }
    if name == "a" {
        let t = node.text_content();
        let href = node.attr("href").unwrap_or_default();
        if !t.is_empty() {
            out.push(Block::Link { text: t, href });
        }
        return;
    }
    if name == "input" || name == "textarea" {
        if let Some(field) = field_from(node) {
            out.push(Block::Field(field));
        }
        return;
    }
    if matches!(name, "p" | "li" | "div" | "article" | "section" | "main") {
        let direct = node.direct_text();
        if !direct.is_empty() && !node.has_child_named("p") && !node.has_child_named("input") {
            out.push(Block::Text(direct));
        }
    }
    for child in &node.children {
        walk_blocks(child, out);
    }
}

fn field_from(node: &Node) -> Option<FormField> {
    let input_type = node.attr("type").unwrap_or_else(|| {
        if node.name == "textarea" {
            "textarea".into()
        } else {
            "text".into()
        }
    });
    if matches!(
        input_type.as_str(),
        "hidden" | "submit" | "button" | "image" | "reset"
    ) {
        return None;
    }
    Some(FormField {
        name: node.attr("name").unwrap_or_default(),
        id: node.attr("id").unwrap_or_default(),
        input_type,
        autocomplete: node.attr("autocomplete").unwrap_or_default(),
        label: node
            .attr("placeholder")
            .or_else(|| node.attr("aria-label"))
            .unwrap_or_default(),
    })
}

fn heading_level(name: &str) -> Option<u8> {
    match name {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

fn find_text(node: &Node, tag: &str) -> Option<String> {
    if node.name == tag {
        return Some(node.text_content());
    }
    for child in &node.children {
        if let Some(t) = find_text(child, tag) {
            return Some(t);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_and_form() {
        let html = r#"<html><head><title>Login</title></head>
        <body><h1>Hi</h1><p>Welcome</p>
        <form><input type="email" name="user" placeholder="mail">
        <input type="password" name="pass"></form></body></html>"#;
        let root = parse(html);
        assert_eq!(document_title(&root), "Login");
        let blocks = visible_blocks(&root);
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, Block::Heading(1, t) if t == "Hi"))
        );
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, Block::Field(f) if f.input_type == "email"))
        );
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, Block::Field(f) if f.input_type == "password"))
        );
    }

    #[test]
    fn extracts_author_css() {
        let root = parse("<style>p { color: red }</style><p>x</p>");
        assert!(author_css(&root).contains("color"));
    }
}
