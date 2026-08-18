//! Engine spine: HTML bytes → display list.

#![forbid(unsafe_code)]

use frihart_css::parse_stylesheet;
use frihart_gfx::{DisplayList, from_boxes};
use frihart_html::{author_css, document_title, parse, visible_blocks};
use frihart_layout::{LayoutBox, block_flow};
use frihart_style::{Computed, style_tag};

pub struct Frame {
    pub title: String,
    pub boxes: Vec<LayoutBox>,
    pub display: DisplayList,
    pub author_css: String,
}

pub fn layout_html(html: &str, extra_css: &str, viewport_w: f32) -> Frame {
    let tree = parse(html);
    let mut css = author_css(&tree);
    if !extra_css.is_empty() {
        if !css.is_empty() {
            css.push('\n');
        }
        css.push_str(extra_css);
    }
    let sheet = parse_stylesheet(&css);
    let mut items: Vec<(String, Computed)> = Vec::new();
    for block in visible_blocks(&tree) {
        let (tag, text) = match block {
            frihart_html::Block::Heading(1, t) => ("h1", t),
            frihart_html::Block::Heading(2, t) => ("h2", t),
            frihart_html::Block::Heading(_, t) => ("h3", t),
            frihart_html::Block::Text(t) | frihart_html::Block::Link { text: t, .. } => ("p", t),
            frihart_html::Block::Field(f) => ("p", f.name),
        };
        items.push((text, style_tag(tag, &sheet)));
    }
    let boxes = block_flow(&items, viewport_w, 0.0);
    let display = from_boxes(&boxes);
    Frame {
        title: document_title(&tree),
        boxes,
        display,
        author_css: css,
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
}
