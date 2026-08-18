//! HTML subset: tokenize, tree, forms, visible fragments.

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

/// Tag + id + class, plus ancestors from root toward the parent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Qual {
    pub tag: String,
    pub id: String,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub qual: Qual,
    pub ancestors: Vec<Qual>,
    pub kind: Block,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, String),
    Text(String),
    Link {
        text: String,
        href: String,
    },
    Field(FormField),
    ListItem {
        ordered: bool,
        index: usize,
        text: String,
    },
    Pre(String),
    Quote(String),
    Image {
        alt: String,
        src: String,
    },
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
    visible_fragments(root)
        .into_iter()
        .map(|f| f.kind)
        .collect()
}

pub fn visible_fragments(root: &Node) -> Vec<Fragment> {
    let mut out = Vec::new();
    walk(root, &[], &mut out);
    out
}

fn qual_of(node: &Node) -> Qual {
    Qual {
        tag: node.name.clone(),
        id: node.attr("id").unwrap_or_default(),
        classes: node
            .attr("class")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect(),
    }
}

fn push_frag(out: &mut Vec<Fragment>, qual: Qual, ancestors: &[Qual], kind: Block) {
    out.push(Fragment {
        qual,
        ancestors: ancestors.to_vec(),
        kind,
    });
}

fn walk(node: &Node, ancs: &[Qual], out: &mut Vec<Fragment>) {
    if node.is_text() {
        return;
    }
    let name = node.name.as_str();
    if matches!(name, "script" | "style" | "noscript" | "head") {
        return;
    }
    let qual = qual_of(node);
    if let Some(level) = heading_level(name) {
        let t = node.text_content();
        if !t.is_empty() {
            push_frag(out, qual, ancs, Block::Heading(level, t));
        }
        return;
    }
    if name == "pre" {
        let t = raw_text(node);
        if !t.trim().is_empty() {
            push_frag(out, qual, ancs, Block::Pre(t));
        }
        return;
    }
    if name == "blockquote" {
        let t = node.text_content();
        if !t.is_empty() {
            push_frag(out, qual, ancs, Block::Quote(t));
        }
        return;
    }
    if name == "ul" || name == "ol" {
        let ordered = name == "ol";
        let mut child_ancs = ancs.to_vec();
        child_ancs.push(qual);
        let mut index = 1usize;
        for child in &node.children {
            if child.name == "li" {
                let t = child.text_content();
                if !t.is_empty() {
                    push_frag(
                        out,
                        qual_of(child),
                        &child_ancs,
                        Block::ListItem {
                            ordered,
                            index,
                            text: t,
                        },
                    );
                }
                index += 1;
            }
        }
        return;
    }
    if name == "img" {
        push_frag(
            out,
            qual,
            ancs,
            Block::Image {
                alt: node.attr("alt").unwrap_or_default(),
                src: node.attr("src").unwrap_or_default(),
            },
        );
        return;
    }
    if name == "input" || name == "textarea" {
        if let Some(field) = field_from(node) {
            push_frag(out, qual, ancs, Block::Field(field));
        }
        return;
    }
    if name == "a" && !has_block_child(node) {
        let t = node.text_content();
        let href = node.attr("href").unwrap_or_default();
        if !t.is_empty() {
            push_frag(out, qual.clone(), ancs, Block::Link { text: t, href });
        }
        return;
    }
    let mut child_ancs = ancs.to_vec();
    if !name.is_empty() && name != "document" {
        child_ancs.push(qual.clone());
    }
    if is_flow_container(name) {
        walk_inlines(node, &child_ancs, &qual, out);
        return;
    }
    for child in &node.children {
        walk(child, &child_ancs, out);
    }
}

fn walk_inlines(node: &Node, ancs: &[Qual], container: &Qual, out: &mut Vec<Fragment>) {
    let mut buf = String::new();
    for child in &node.children {
        if child.is_text() {
            append_collapsed(&mut buf, child.text.as_deref().unwrap_or(""));
            continue;
        }
        match child.name.as_str() {
            "br" => buf.push('\n'),
            "a" => {
                flush_text(&mut buf, container, ancs, out);
                let t = child.text_content();
                let href = child.attr("href").unwrap_or_default();
                if !t.is_empty() {
                    push_frag(out, qual_of(child), ancs, Block::Link { text: t, href });
                }
                for inner in &child.children {
                    if inner.name == "img" {
                        let mut img_ancs = ancs.to_vec();
                        img_ancs.push(qual_of(child));
                        push_frag(
                            out,
                            qual_of(inner),
                            &img_ancs,
                            Block::Image {
                                alt: inner.attr("alt").unwrap_or_default(),
                                src: inner.attr("src").unwrap_or_default(),
                            },
                        );
                    }
                }
            }
            "img" => {
                flush_text(&mut buf, container, ancs, out);
                push_frag(
                    out,
                    qual_of(child),
                    ancs,
                    Block::Image {
                        alt: child.attr("alt").unwrap_or_default(),
                        src: child.attr("src").unwrap_or_default(),
                    },
                );
            }
            "input" | "textarea" => {
                flush_text(&mut buf, container, ancs, out);
                if let Some(field) = field_from(child) {
                    push_frag(out, qual_of(child), ancs, Block::Field(field));
                }
            }
            "script" | "style" | "noscript" => {}
            name if is_block(name) || !is_phrasing(name) => {
                flush_text(&mut buf, container, ancs, out);
                walk(child, ancs, out);
            }
            _ => append_collapsed(&mut buf, &child.text_content()),
        }
    }
    flush_text(&mut buf, container, ancs, out);
}

fn flush_text(buf: &mut String, container: &Qual, ancs: &[Qual], out: &mut Vec<Fragment>) {
    let t = if container.tag == "pre" {
        buf.clone()
    } else {
        collapse_keep_newlines(buf)
    };
    buf.clear();
    if t.is_empty() {
        return;
    }
    push_frag(out, container.clone(), ancs, Block::Text(t));
}

fn append_collapsed(buf: &mut String, text: &str) {
    let piece = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if piece.is_empty() {
        return;
    }
    if !buf.is_empty() && !buf.ends_with('\n') && !buf.ends_with(' ') {
        buf.push(' ');
    }
    buf.push_str(&piece);
}

fn collapse_keep_newlines(s: &str) -> String {
    let mut lines: Vec<String> = s
        .split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn raw_text(node: &Node) -> String {
    if let Some(t) = &node.text {
        return t.clone();
    }
    let mut s = String::new();
    for child in &node.children {
        s.push_str(&raw_text(child));
    }
    s
}

fn is_flow_container(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "article"
            | "section"
            | "main"
            | "body"
            | "document"
            | "form"
            | "header"
            | "footer"
            | "nav"
            | "td"
            | "th"
            | "dd"
    )
}

fn is_phrasing(name: &str) -> bool {
    matches!(
        name,
        "span"
            | "em"
            | "strong"
            | "code"
            | "b"
            | "i"
            | "u"
            | "small"
            | "abbr"
            | "time"
            | "mark"
            | "sub"
            | "sup"
            | "label"
    )
}

fn is_block(name: &str) -> bool {
    heading_level(name).is_some()
        || matches!(
            name,
            "p" | "div"
                | "article"
                | "section"
                | "main"
                | "ul"
                | "ol"
                | "li"
                | "pre"
                | "blockquote"
                | "form"
                | "header"
                | "footer"
                | "nav"
                | "hr"
                | "table"
        )
}

fn has_block_child(node: &Node) -> bool {
    node.children.iter().any(|c| is_block(&c.name))
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

    #[test]
    fn lists_pre_quote_br_img() {
        let html = r#"<article class="post" id="main">
            <ul><li>one</li><li>two</li></ul>
            <ol><li>first</li></ol>
            <pre>line1
line2</pre>
            <blockquote>quoted</blockquote>
            <p>hello<br>world</p>
            <img src="pic.png" alt="a picture" class="hero">
        </article>"#;
        let frags = visible_fragments(&parse(html));
        assert!(frags.iter().any(
            |f| matches!(&f.kind, Block::ListItem { ordered: false, text, .. } if text == "one")
        ));
        assert!(frags.iter().any(|f| matches!(
            &f.kind,
            Block::ListItem {
                ordered: true,
                index: 1,
                ..
            }
        )));
        assert!(frags.iter().any(
            |f| matches!(&f.kind, Block::Pre(t) if t.contains("line1") && t.contains("line2"))
        ));
        assert!(
            frags
                .iter()
                .any(|f| matches!(&f.kind, Block::Quote(t) if t == "quoted"))
        );
        assert!(
            frags.iter().any(
                |f| matches!(&f.kind, Block::Text(t) if t.contains("hello") && t.contains('\n'))
            )
        );
        assert!(frags.iter().any(|f| {
            matches!(&f.kind, Block::Image { alt, src } if alt == "a picture" && src == "pic.png")
                && f.qual.classes.iter().any(|c| c == "hero")
        }));
        assert!(frags.iter().any(|f| {
            f.ancestors
                .iter()
                .any(|a| a.tag == "article" && a.id == "main")
        }));
    }
}
