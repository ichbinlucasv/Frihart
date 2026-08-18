//! Engine spine: HTML bytes → display list.

#![forbid(unsafe_code)]

use frihart_css::parse_stylesheet;
use frihart_gfx::{DisplayList, from_boxes};
use frihart_html::{Block, author_css, document_title, parse, visible_fragments};
use frihart_layout::{FlowItem, LayoutBox, block_flow};
use frihart_style::{Element, style_element};

pub struct Frame {
    pub title: String,
    pub boxes: Vec<LayoutBox>,
    pub display: DisplayList,
    pub author_css: String,
}

pub fn layout_html(html: &str, extra_css: &str, viewport_w: f32) -> Frame {
    let tree = parse(html);
    let author = author_css(&tree);
    let user = parse_stylesheet(extra_css);
    let author_sheet = parse_stylesheet(&author);
    let mut items: Vec<FlowItem> = Vec::new();
    for frag in visible_fragments(&tree) {
        let el = element_from(&frag);
        let style = style_element(&el, &user, &author_sheet);
        let (text, href, preserve, image, cells) = match frag.kind {
            Block::Heading(_, t) | Block::Text(t) | Block::Quote(t) => {
                (t, None, false, false, Vec::new())
            }
            Block::Pre(t) => (t, None, true, false, Vec::new()),
            Block::Link { text, href } => (text, Some(href), false, false, Vec::new()),
            Block::ListItem {
                ordered,
                index,
                text,
            } => {
                let prefix = if ordered {
                    format!("{index}. ")
                } else {
                    "• ".into()
                };
                (format!("{prefix}{text}"), None, false, false, Vec::new())
            }
            Block::Image { alt, src } => {
                let label = if alt.is_empty() {
                    format!("[img {src}]")
                } else {
                    format!("[img] {alt}")
                };
                (label, None, false, true, Vec::new())
            }
            Block::Field(f) => {
                let name = if f.label.is_empty() { f.name } else { f.label };
                (name, None, false, false, Vec::new())
            }
            Block::TableRow { cells } => (String::new(), None, false, false, cells),
        };
        items.push(FlowItem {
            text,
            style,
            href,
            preserve,
            image,
            cells,
        });
    }
    let boxes = block_flow(&items, viewport_w, 0.0);
    let display = from_boxes(&boxes);
    Frame {
        title: document_title(&tree),
        boxes,
        display,
        author_css: author,
    }
}

fn element_from(frag: &frihart_html::Fragment) -> Element {
    Element {
        tag: frag.qual.tag.clone(),
        id: frag.qual.id.clone(),
        classes: frag.qual.classes.clone(),
        ancestors: frag
            .ancestors
            .iter()
            .map(|q| Element {
                tag: q.tag.clone(),
                id: q.id.clone(),
                classes: q.classes.clone(),
                ancestors: Vec::new(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_heading() {
        let f = layout_html("<title>T</title><h1>Hi</h1><p>there</p>", "", 640.0);
        assert_eq!(f.title, "T");
        assert!(f.boxes.len() >= 2);
        assert!(!f.display.ops.is_empty());
    }

    #[test]
    fn style_tag_changes_size() {
        let f = layout_html(
            "<style>h1{font-size:10px}</style><title>T</title><h1>Hi</h1>",
            "",
            400.0,
        );
        assert!(f.author_css.contains("font-size"));
        assert_eq!(f.boxes[0].style.font_size, 10.0);
    }

    #[test]
    fn user_css_then_author() {
        let html = r#"<article id="main"><p class="lead">Hello there this is a paragraph.</p>
            <a href="https://ex.test/x">link</a>
            <ul><li>item</li></ul>
            <style>.lead { font-size: 18px }</style></article>"#;
        let f = layout_html(html, "p { max-width: 320px; line-height: 1.6 }", 640.0);
        let p = f
            .boxes
            .iter()
            .find(|b| b.text.contains("Hello"))
            .expect("p");
        assert_eq!(p.style.font_size, 18.0);
        assert_eq!(p.style.max_width, Some(320.0));
        assert!(
            f.display
                .hit_test(
                    4.0,
                    f.boxes
                        .iter()
                        .find(|b| b.href.is_some())
                        .map(|b| b.y + 2.0)
                        .unwrap_or(0.0)
                )
                .is_some()
                || f.boxes
                    .iter()
                    .any(|b| b.href.as_deref() == Some("https://ex.test/x"))
        );
        assert!(f.boxes.iter().any(|b| b.text.starts_with('•')));
    }

    #[test]
    fn tables_become_columns() {
        let f = layout_html(
            "<table><tr><td>left</td><td>right</td></tr><tr><td>1</td><td>2</td></tr></table>",
            "",
            400.0,
        );
        let left = f.boxes.iter().find(|b| b.text == "left").expect("left");
        let right = f.boxes.iter().find(|b| b.text == "right").expect("right");
        let one = f.boxes.iter().find(|b| b.text == "1").expect("1");
        assert!(right.x > left.x);
        assert!((left.y - right.y).abs() < 0.5);
        assert!(one.y > left.y);
        assert!((one.x - left.x).abs() < 0.5);
        assert!(left.cell && right.cell);
    }
}
