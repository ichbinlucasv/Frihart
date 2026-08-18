//! Engine spine: HTML bytes → display list.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use frihart_css::parse_stylesheet;
use frihart_gfx::DisplayOp;
use frihart_gfx::{DisplayList, from_boxes};
use frihart_html::{Block, author_css, document_title, parse, visible_fragments};
use frihart_layout::{FlowItem, LayoutBox, block_flow};
use frihart_style::{Align, Display, Element, contrast_on, style_in};

/// JSON job for the sandboxed content worker (`frihart --content-worker`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutJob {
    pub html: String,
    pub extra_css: String,
    pub viewport_w: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutOut {
    pub title: String,
    pub display: DisplayList,
    pub sandboxed: bool,
    pub detail: String,
}

pub fn execute(job: &LayoutJob) -> LayoutOut {
    let frame = layout_html(&job.html, &job.extra_css, job.viewport_w);
    LayoutOut {
        title: frame.title,
        display: frame.display,
        sandboxed: false,
        detail: "in-process".into(),
    }
}

pub struct Frame {
    pub title: String,
    pub boxes: Vec<LayoutBox>,
    pub display: DisplayList,
    pub author_css: String,
}

pub fn layout_html(html: &str, extra_css: &str, viewport_w: f32) -> Frame {
    layout_html_ex(html, extra_css, viewport_w, (viewport_w * 0.75).max(480.0))
}

pub fn layout_html_ex(html: &str, extra_css: &str, viewport_w: f32, viewport_h: f32) -> Frame {
    let tree = parse(html);
    let author = author_css(&tree);
    let user = parse_stylesheet(extra_css);
    let author_sheet = parse_stylesheet(&author);
    let body = style_in(
        &Element::tag("body"),
        &user,
        &author_sheet,
        viewport_w,
        viewport_h,
    );
    let content_w = body.width.unwrap_or(viewport_w).min(viewport_w).max(1.0);
    let mut items: Vec<FlowItem> = Vec::new();
    let mut field_i = 0usize;
    for frag in visible_fragments(&tree) {
        let el = element_from(&frag);
        let mut style = style_in(&el, &user, &author_sheet, content_w, viewport_h);
        style.color = contrast_on(body.background, style.color);
        let inline = matches!(frag.kind, Block::Inline(_) | Block::Link { .. });
        let (text, href, preserve, image, cells, field, rule) = match frag.kind {
            Block::Heading(_, t)
            | Block::Text(t)
            | Block::Inline(t)
            | Block::Quote(t)
            | Block::Caption(t) => (t, None, false, false, Vec::new(), None, false),
            Block::Pre(t) => (t, None, true, false, Vec::new(), None, false),
            Block::Link { text, href } => (text, Some(href), false, false, Vec::new(), None, false),
            Block::Rule => (String::new(), None, false, false, Vec::new(), None, true),
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
                (
                    format!("{prefix}{text}"),
                    None,
                    false,
                    false,
                    Vec::new(),
                    None,
                    false,
                )
            }
            Block::Image { alt, src } => {
                let label = if alt.is_empty() {
                    format!("[img {src}]")
                } else {
                    format!("[img] {alt}")
                };
                (label, None, false, true, Vec::new(), None, false)
            }
            Block::Field(f) => {
                let name = if f.label.is_empty() { f.name } else { f.label };
                let secret = f.input_type == "password";
                let slot = Some(frihart_layout::FieldSlot {
                    index: field_i,
                    secret,
                });
                field_i += 1;
                (name, None, false, false, Vec::new(), slot, false)
            }
            Block::TableRow { cells, header } => {
                if header {
                    style.font_weight = 700;
                }
                (String::new(), None, false, false, cells, None, false)
            }
        };
        if inline {
            style.display = Display::Inline;
        }
        items.push(FlowItem {
            text,
            style,
            href,
            preserve,
            image,
            rule,
            cells,
            field,
        });
    }
    let origin_y = body.margin;
    let mut boxes = block_flow(&items, content_w, origin_y);
    if matches!(body.text_align, Align::Center) {
        let dx = ((viewport_w - content_w) / 2.0).max(0.0);
        for b in &mut boxes {
            b.x += dx;
        }
    }
    let mut display = from_boxes(&boxes);
    if body.background != 0 {
        let h = boxes
            .iter()
            .map(|b| b.y + b.h)
            .fold(viewport_h, f32::max)
            .max(1.0);
        display.ops.insert(
            0,
            DisplayOp::Fill {
                x: 0.0,
                y: 0.0,
                w: viewport_w,
                h,
                color: body.background,
            },
        );
    }
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

    #[test]
    fn fields_are_display_ops() {
        let f = layout_html(
            r#"<form><input name="email" type="email" placeholder="mail"></form>"#,
            "",
            400.0,
        );
        assert!(
            f.display
                .ops
                .iter()
                .any(|op| matches!(op, frihart_gfx::DisplayOp::Field { index: 0, .. }))
        );
    }

    #[test]
    fn hr_is_a_rule_fill() {
        let f = layout_html("<p>a</p><hr><p>b</p>", "", 400.0);
        assert!(f.boxes.iter().any(|b| b.rule));
        assert!(f.display.ops.iter().any(|op| matches!(
            op,
            frihart_gfx::DisplayOp::Fill { h, .. } if *h <= 4.0
        )));
    }

    #[test]
    fn caption_is_text() {
        let f = layout_html(
            "<table><caption>Nums</caption><tr><td>1</td></tr></table>",
            "",
            400.0,
        );
        assert!(f.boxes.iter().any(|b| b.text == "Nums"));
    }

    #[test]
    fn em_weight_and_border() {
        let f = layout_html(
            "<style>p{font-size:2em;font-weight:700;border:2px solid #445566}</style><p>Hi</p>",
            "",
            400.0,
        );
        let p = f.boxes.iter().find(|b| b.text == "Hi").expect("p");
        assert_eq!(p.style.font_size, 32.0);
        assert_eq!(p.style.font_weight, 700);
        assert_eq!(p.style.border_width, 2.0);
        assert!(
            f.display
                .ops
                .iter()
                .any(|op| matches!(op, frihart_gfx::DisplayOp::Text { weight: 700, .. }))
        );
    }

    #[test]
    fn job_roundtrip_json() {
        let job = LayoutJob {
            html: "<h1>Hi</h1>".into(),
            extra_css: String::new(),
            viewport_w: 400.0,
        };
        let raw = serde_json::to_string(&job).unwrap();
        let back: LayoutJob = serde_json::from_str(&raw).unwrap();
        let out = execute(&back);
        assert!(!out.display.is_empty());
        assert!(out.display.find("Hi").is_some());
    }

    #[test]
    fn strong_paints_bold_on_the_same_line() {
        let f = layout_html("<p>hello <strong>bold</strong> world</p>", "", 400.0);
        let bold = f.boxes.iter().find(|b| b.text == "bold").expect("bold");
        assert_eq!(bold.style.font_weight, 700);
        assert!(f.boxes.iter().any(|b| b.text.contains("hello")));
        assert!(f.boxes.iter().any(|b| b.text.contains("world")));
        let hello = f
            .boxes
            .iter()
            .find(|b| b.text.contains("hello"))
            .expect("hello");
        assert!((hello.y - bold.y).abs() < 1.0);
        assert!(bold.x > hello.x);
    }

    const EXAMPLE_COM: &str = r#"<!doctype html><html lang="en"><head><title>Example Domain</title><link rel="icon" href="data:,"><meta name="viewport" content="width=device-width, initial-scale=1"><style>body{background:#eee;width:60vw;margin:15vh auto;font-family:system-ui,sans-serif}h1{font-size:1.5em}div{opacity:0.8}a:link,a:visited{color:#348}</style></head><body><div><h1>Example Domain</h1><p>This domain is for use in documentation examples without needing permission. Avoid use in operations.</p><p><a href="https://iana.org/domains/example">Learn more</a></p></div></body></html>"#;

    #[test]
    fn example_com_is_readable() {
        let f = layout_html_ex(EXAMPLE_COM, "", 1000.0, 800.0);
        assert_eq!(f.title, "Example Domain");
        let h1 = f
            .boxes
            .iter()
            .find(|b| b.text == "Example Domain")
            .expect("h1");
        assert!((h1.style.font_size - 24.0).abs() < 0.5);
        assert!(
            f.boxes
                .iter()
                .any(|b| b.text.contains("documentation examples"))
        );
        let link = f
            .boxes
            .iter()
            .find(|b| b.href.as_deref() == Some("https://iana.org/domains/example"))
            .expect("iana");
        assert_eq!(link.text, "Learn more");
        assert_eq!(link.style.color, 0x00334488);
        assert!(h1.x > 50.0);
        assert!(h1.style.color < 0x00800000);
        assert!(f.display.ops.iter().any(|op| matches!(
            op,
            DisplayOp::Fill {
                color: 0x00eeeeee,
                ..
            }
        )));
    }
}
