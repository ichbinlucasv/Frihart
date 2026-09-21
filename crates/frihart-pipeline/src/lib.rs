//! Engine spine: HTML bytes → display list.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use frihart_css::parse_stylesheet;
use frihart_gfx::DisplayOp;
use frihart_gfx::{DisplayList, from_boxes};
use frihart_html::{Block, author_css, document_title, parse, visible_fragments};
use frihart_layout::{FlowItem, LayoutBox, block_flow};
use frihart_style::{Align, Display, Element, contrast_on, list_marker, style_in};

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
        let (text, href, image, cells, field, rule) = match frag.kind {
            Block::Heading(_, t)
            | Block::Text(t)
            | Block::Inline(t)
            | Block::Quote(t)
            | Block::Caption(t)
            | Block::Pre(t) => (t, None, false, Vec::new(), None, false),
            Block::Link { text, href } => (text, Some(href), false, Vec::new(), None, false),
            Block::Rule => (String::new(), None, false, Vec::new(), None, true),
            Block::ListItem { index, text, .. } => {
                let prefix = list_marker(style.list_style, index);
                (
                    format!("{prefix}{text}"),
                    None,
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
                (label, None, true, Vec::new(), None, false)
            }
            Block::Field(f) => {
                let name = if f.label.is_empty() { f.name } else { f.label };
                let secret = f.input_type == "password";
                let slot = Some(frihart_layout::FieldSlot {
                    index: field_i,
                    secret,
                });
                field_i += 1;
                (name, None, false, Vec::new(), slot, false)
            }
            Block::TableRow { cells, header } => {
                if header {
                    style.font_weight = 700;
                }
                (String::new(), None, false, cells, None, false)
            }
        };
        // soft-wrap off for `pre` / `nowrap`; on for `normal` / `pre-wrap` / `pre-line`
        let preserve = style.white_space.no_wrap();
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
    use frihart_style::FontSlot;

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
    fn font_family_uses_engine_slots() {
        let html =
            r#"<style>p{font-family:"Comic Sans MS", monospace}</style><p>codey</p><pre>raw</pre>"#;
        let f = layout_html(html, "", 400.0);
        let p = f
            .boxes
            .iter()
            .find(|b| b.text.contains("codey"))
            .expect("p");
        assert_eq!(p.style.font_family, FontSlot::Mono);
        let pre = f
            .boxes
            .iter()
            .find(|b| b.text.contains("raw"))
            .expect("pre");
        assert_eq!(pre.style.font_family, FontSlot::Mono);
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

    #[test]
    fn rfc1918_is_readable() {
        let html = include_str!("../testdata/rfc1918.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert_eq!(f.title, "Address Allocation for Private Internets");
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("10.0.0.0"));
        assert!(blob.contains("172.16"));
        assert!(blob.contains("192.168"));
        assert!(blob.contains("private"));
        assert!(f.boxes.iter().any(|b| b.preserve));
        assert!(f.boxes.iter().any(|b| b.rule));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert_eq!(wide.title, f.title);
        assert!(wide.boxes.iter().any(|b| b.text.contains("10.0.0.0")));
    }

    #[test]
    fn suckless_is_readable() {
        let html = include_str!("../testdata/suckless.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.to_ascii_lowercase().contains("suckless"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("dwm") || f.boxes.iter().any(|b| b.text.contains("dwm")));
        assert!(f.boxes.iter().any(|b| b.text.contains("News")));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("dwm.suckless.org"))
        }));
        assert!(
            f.boxes
                .iter()
                .any(|b| b.href.as_deref() == Some("https://dwm.suckless.org/")
                    || b.href.as_deref() == Some("https://dwm.suckless.org"))
        );
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.to_ascii_lowercase().contains("suckless"));
    }

    #[test]
    fn gnu_philosophy_is_readable() {
        let html = include_str!("../testdata/gnu-philosophy.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("Philosophy of the GNU Project"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("Free software") || blob.contains("free software"));
        assert!(blob.contains("four essential freedoms"));
        assert!(f.boxes.iter().any(|b| b.text.contains("Introduction")));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/philosophy/free-sw.html"))
        }));
        assert!(
            blob.contains('—') || blob.contains("copied and changed"),
            "mdash or surrounding sentence"
        );
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("Philosophy of the GNU Project"));
        assert!(wide.boxes.iter().any(|b| b.text.contains("Free Software")));
    }

    #[test]
    fn kernel_org_is_readable() {
        let html = include_str!("../testdata/kernel.org.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("Linux Kernel Archives"));
        assert!(
            f.boxes
                .iter()
                .any(|b| b.text.contains("Linux Kernel Archives"))
        );
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("mainline"));
        assert!(blob.contains("stable"));
        assert!(blob.contains("7.2"));
        assert!(f.boxes.iter().any(|b| b.cell && b.text.contains("7.2")));
        assert!(
            blob.contains("git.kernel.org")
                || f.boxes.iter().any(|b| b.text.contains("git.kernel.org"))
        );
        assert!(f.boxes.iter().any(|b| b.href.as_deref().is_some_and(|h| {
            h.contains("kernel.org") && (h.contains("about") || h.contains("releases"))
        })));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("Linux Kernel Archives"));
        assert!(
            wide.boxes
                .iter()
                .any(|b| b.cell && b.text.contains("mainline"))
        );
    }

    #[test]
    fn docs_kernel_org_is_readable() {
        let html = include_str!("../testdata/docs.kernel.org.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("Linux Kernel documentation"));
        assert!(
            f.boxes
                .iter()
                .any(|b| matches!(b.text.as_str(), t if t.contains("Linux Kernel documentation") && !t.contains('¶')))
        );
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("work in progress") || blob.contains("development community"));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("process/development-process.html"))
        }));
        assert!(
            f.boxes
                .iter()
                .any(|b| b.text.contains("Submitting patches"))
        );
        assert!(!f.boxes.iter().any(|b| b.text.contains("Quick search")));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("Linux Kernel documentation"));
    }

    #[test]
    fn ietf_org_is_readable() {
        let html = include_str!("../testdata/ietf.org.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("IETF"));
        assert!(f.title.contains("Internet Engineering Task Force"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("Welcome to the IETF") || blob.contains("IETF 126"));
        assert!(blob.contains("San Francisco"));
        assert!(
            blob.contains("standards")
                || blob.contains("Internet standards")
                || blob.contains("open standards")
        );
        assert!(f.boxes.iter().any(|b| {
            b.href.as_deref().is_some_and(|h| {
                h.contains("/meeting/127") || h.contains("/live/") || h.contains("/meeting/")
            })
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("IETF"));
    }

    #[test]
    fn rfc_editor_org_is_readable() {
        let html = include_str!("../testdata/rfc-editor.org.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("RFC Editor"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("official home of RFCs") || blob.contains("The official home"));
        assert!(blob.contains("Latest RFCs") || blob.contains("What Is an RFC"));
        assert!(blob.contains("RFC 10030") || blob.contains("RFC 10031"));
        assert!(blob.contains("Network Time Protocol") || blob.contains("Latest RFCs"));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/info/rfc10030"))
                && (b.text.contains("RFC 10030") || b.text.contains("Network Time Protocol"))
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("RFC Editor"));
    }

    #[test]
    fn w3_org_is_readable() {
        let html = include_str!("../testdata/w3.org.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("W3C"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("Making the web work"));
        assert!(blob.contains("World Wide Web Consortium"));
        assert!(blob.contains("TPAC 2026") || blob.contains("Web standards"));
        assert!(blob.contains("ARIA in HTML") || blob.contains("standards and guidelines"));
        assert!(f.boxes.iter().any(|b| {
            b.href.as_deref().is_some_and(|h| h.contains("/standards"))
                && b.text.contains("standards")
        }));
        assert!(f.boxes.iter().any(|b| {
            b.href.as_deref().is_some_and(|h| h.contains("/news/")) && b.text.contains("ARIA")
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("W3C"));
        assert!(
            wide.boxes
                .iter()
                .any(|b| b.text.contains("Making the web work"))
        );
    }

    #[test]
    fn w3_org_tr_index_is_readable() {
        let html = include_str!("../testdata/w3.org-tr.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("W3C standards and drafts"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("W3C publishes a range of technical reports"));
        assert!(blob.contains("1236"));
        assert!(blob.contains("families"));
        assert!(blob.contains("RDF 1.2 Turtle"));
        assert!(blob.contains("Draft Standard") || blob.contains("Recommendation"));
        assert!(
            f.boxes
                .iter()
                .any(|b| b.text.contains("RDF") && !b.text.contains("Turtle"))
        );
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/TR/rdf12-turtle/"))
                && b.text.contains("RDF 1.2 Turtle")
        }));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/TR/WCAG22/") || h.contains("/TR/wcag-3.0/"))
                || b.text.contains("WCAG")
        }));
        assert!(blob.contains("Tags"));
        assert!(blob.contains("Deliverers"));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("W3C standards and drafts"));
        assert!(wide.boxes.iter().any(|b| b.text.contains("RDF 1.2 Turtle")));
    }

    #[test]
    fn webarch_is_readable() {
        let html = include_str!("../testdata/webarch.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("Architecture of the World Wide Web"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("W3C Recommendation") || blob.contains("15 December 2004"));
        assert!(blob.contains("identification of resources"));
        assert!(blob.contains("Identification"));
        assert!(blob.contains("Interaction"));
        assert!(blob.contains("Data Formats"));
        assert!(blob.contains("Orthogonal") || blob.contains("orthogonal"));
        assert!(blob.contains("URI"));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/TR/webarch") || h.contains("REC-webarch"))
        }));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("#intro") || h.contains("#identification"))
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("Architecture of the World Wide Web"));
        assert!(wide.boxes.iter().any(|b| b.text.contains("Identification")));
    }

    #[test]
    fn rfc9110_is_readable() {
        let html = include_str!("../testdata/rfc9110.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("RFC 9110"));
        assert!(f.title.contains("HTTP Semantics"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("HTTP Semantics"));
        assert!(blob.contains("Abstract"));
        assert!(blob.contains("Status of This Memo") || blob.contains("Standards Track"));
        assert!(blob.contains("Methods") || blob.contains("GET"));
        assert!(blob.contains("idempotent") || blob.contains("safe"));
        assert!(
            f.boxes
                .iter()
                .any(|b| b.preserve && (b.text.contains("HTTP") || b.text.contains("=")))
        );
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("rfc-editor.org") || h.contains("/rfc/rfc"))
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("HTTP Semantics"));
        assert!(wide.boxes.iter().any(|b| b.text.contains("Abstract")));
    }

    #[test]
    fn wcag22_is_readable() {
        let html = include_str!("../testdata/wcag22.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("WCAG 2.2") || f.title.contains("Web Content Accessibility"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("Web Content Accessibility Guidelines"));
        assert!(blob.contains("Abstract"));
        assert!(blob.contains("Perceivable"));
        assert!(blob.contains("Operable"));
        assert!(blob.contains("Understandable") || blob.contains("Robust"));
        assert!(blob.contains("Guideline 1.1") || blob.contains("Text Alternatives"));
        assert!(f.boxes.iter().any(|b| {
            b.href
                .as_deref()
                .is_some_and(|h| h.contains("/TR/WCAG22") || h.contains("REC-WCAG22"))
        }));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("WCAG") || wide.title.contains("Accessibility"));
        assert!(wide.boxes.iter().any(|b| b.text.contains("Perceivable")));
    }

    #[test]
    fn rfc8446_is_readable() {
        let html = include_str!("../testdata/rfc8446.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(f.title.contains("Transport Layer Security") || f.title.contains("TLS"));
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("TLS") && blob.contains("1.3"));
        assert!(blob.contains("Abstract"));
        assert!(blob.contains("Handshake") || blob.contains("ClientHello"));
        assert!(f.boxes.iter().any(|b| b.preserve));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(wide.title.contains("TLS") || wide.title.contains("Transport Layer Security"));
    }

    #[test]
    fn rfc5280_is_readable() {
        let html = include_str!("../testdata/rfc5280.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(
            f.title.contains("X.509")
                || f.title.contains("Public Key Infrastructure")
                || f.title.contains("Certificate")
        );
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("X.509"));
        assert!(blob.contains("Abstract"));
        assert!(blob.contains("Certificate") || blob.contains("CRL"));
        assert!(f.boxes.iter().any(|b| b.preserve));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(
            wide.title.contains("X.509")
                || wide.title.contains("Certificate")
                || wide.title.contains("Public Key")
        );
    }

    #[test]
    fn rfc8032_is_readable() {
        let html = include_str!("../testdata/rfc8032.html");
        let f = layout_html_ex(html, "", 1000.0, 800.0);
        assert!(
            f.title.contains("EdDSA")
                || f.title.contains("Edwards-Curve")
                || f.title.contains("Ed25519")
        );
        let blob: String = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(blob.contains("EdDSA") || blob.contains("Ed25519"));
        assert!(blob.contains("Abstract"));
        assert!(blob.contains("edwards25519") || blob.contains("Ed25519"));
        assert!(f.boxes.iter().any(|b| b.preserve));
        let wide = layout_html_ex(html, "", 5120.0, 1440.0);
        assert!(
            wide.title.contains("EdDSA")
                || wide.title.contains("Edwards-Curve")
                || wide.title.contains("Ed25519")
        );
    }

    #[test]
    fn list_style_none_drops_markers() {
        let html = r#"<style>ul{list-style:none} ol{list-style-type:square}</style>
<ul><li>alpha</li><li>beta</li></ul>
<ol><li>one</li></ol>"#;
        let f = layout_html(html, "", 640.0);
        let texts: Vec<&str> = f.boxes.iter().map(|b| b.text.as_str()).collect();
        assert!(texts.iter().any(|t| *t == "alpha"), "{texts:?}");
        assert!(texts.iter().any(|t| *t == "beta"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.starts_with("•")), "{texts:?}");
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("▪ ") && t.contains("one")),
            "{texts:?}"
        );
    }

    #[test]
    fn white_space_nowrap_and_pre_wrap() {
        let html = r#"<style>p{white-space:nowrap} pre{white-space:pre-wrap}</style>
<p>plain</p><pre>line1
line2</pre>"#;
        let f = layout_html(html, "", 640.0);
        let p = f.boxes.iter().find(|b| b.text == "plain").expect("p");
        assert!(p.preserve, "nowrap → no soft wrap");
        let pre = f
            .boxes
            .iter()
            .find(|b| b.text.contains("line1"))
            .expect("pre");
        assert!(!pre.preserve, "pre-wrap soft-wraps");
        assert_eq!(pre.style.white_space, frihart_style::WhiteSpace::PreWrap);
    }
}
