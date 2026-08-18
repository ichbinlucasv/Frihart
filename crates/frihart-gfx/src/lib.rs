//! Display list. Chrome paints these ops; GPU comes later.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use frihart_layout::LayoutBox;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DisplayOp {
    Fill {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
    Text {
        x: f32,
        y: f32,
        color: u32,
        size: f32,
        weight: u16,
        text: String,
        href: Option<String>,
        max_width: f32,
        wrap: bool,
    },
    Field {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        index: usize,
        secret: bool,
        label: String,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DisplayList {
    pub ops: Vec<DisplayOp>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextHit {
    pub y: f32,
    pub text: String,
}

impl DisplayList {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn searchable(&self) -> String {
        let mut out = String::new();
        for op in &self.ops {
            if let DisplayOp::Text { text, .. } | DisplayOp::Field { label: text, .. } = op {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(text);
            }
        }
        out
    }

    pub fn find(&self, needle: &str) -> Option<TextHit> {
        let n = needle.trim();
        if n.is_empty() {
            return None;
        }
        let n = n.to_ascii_lowercase();
        for op in &self.ops {
            let (y, text) = match op {
                DisplayOp::Text { y, text, .. } => (*y, text.as_str()),
                DisplayOp::Field { y, label, .. } => (*y, label.as_str()),
                DisplayOp::Fill { .. } => continue,
            };
            if text.to_ascii_lowercase().contains(&n) {
                return Some(TextHit {
                    y,
                    text: text.to_string(),
                });
            }
        }
        None
    }

    pub fn hit_test(&self, px: f32, py: f32) -> Option<&str> {
        for op in self.ops.iter().rev() {
            if let DisplayOp::Text {
                x,
                y,
                href: Some(href),
                max_width,
                size,
                ..
            } = op
            {
                let h = size * 2.0;
                if px >= *x && px <= *x + max_width && py >= *y && py <= *y + h {
                    return Some(href);
                }
            }
        }
        None
    }
}

pub fn from_boxes(boxes: &[LayoutBox]) -> DisplayList {
    let mut list = DisplayList::default();
    for b in boxes {
        if b.rule {
            list.ops.push(DisplayOp::Fill {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h.max(1.0),
                color: if b.style.background != 0 {
                    b.style.background
                } else {
                    0x00333333
                },
            });
            continue;
        }
        push_box_chrome(&mut list, b);
        if let Some(field) = &b.field {
            list.ops.push(DisplayOp::Field {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
                index: field.index,
                secret: field.secret,
                label: b.text.clone(),
            });
        } else {
            let inset = b.style.padding + b.style.border_width;
            list.ops.push(DisplayOp::Text {
                x: b.x + inset,
                y: b.y + inset,
                color: b.style.color,
                size: b.style.font_size,
                weight: b.style.font_weight,
                text: b.text.clone(),
                href: b.href.clone(),
                max_width: (b.w - inset * 2.0).max(1.0),
                wrap: !b.preserve,
            });
        }
    }
    list
}

fn push_box_chrome(list: &mut DisplayList, b: &LayoutBox) {
    let bw = b.style.border_width;
    if bw > 0.0 {
        let c = if b.style.border_color != 0 {
            b.style.border_color
        } else {
            0x00333333
        };
        list.ops.push(DisplayOp::Fill {
            x: b.x,
            y: b.y,
            w: b.w,
            h: bw.max(1.0),
            color: c,
        });
        list.ops.push(DisplayOp::Fill {
            x: b.x,
            y: b.y + b.h - bw,
            w: b.w,
            h: bw.max(1.0),
            color: c,
        });
        list.ops.push(DisplayOp::Fill {
            x: b.x,
            y: b.y,
            w: bw.max(1.0),
            h: b.h,
            color: c,
        });
        list.ops.push(DisplayOp::Fill {
            x: b.x + b.w - bw,
            y: b.y,
            w: bw.max(1.0),
            h: b.h,
            color: c,
        });
    }
    if b.cell {
        list.ops.push(DisplayOp::Fill {
            x: b.x + bw,
            y: b.y + bw,
            w: (b.w - bw * 2.0).max(1.0),
            h: (b.h - bw * 2.0).max(1.0),
            color: if b.style.background != 0 {
                b.style.background
            } else {
                0x00181818
            },
        });
    } else if b.image || b.style.background != 0 {
        list.ops.push(DisplayOp::Fill {
            x: b.x + bw,
            y: b.y + bw,
            w: (b.w - bw * 2.0).max(1.0),
            h: (b.h - bw * 2.0).max(1.0),
            color: if b.style.background != 0 {
                b.style.background
            } else {
                0x00202020
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_layout::{FlowItem, block_flow};
    use frihart_style::ua_style;

    #[test]
    fn one_text_op_per_box() {
        let boxes = block_flow(&[FlowItem::text("Hi", ua_style("p"))], 100.0, 0.0);
        let list = from_boxes(&boxes);
        assert_eq!(list.len(), 1);
        assert!(matches!(list.ops[0], DisplayOp::Text { .. }));
    }

    #[test]
    fn link_hit_from_display_list() {
        let mut item = FlowItem::text("here", ua_style("a"));
        item.href = Some("https://ex.test/".into());
        let boxes = block_flow(&[item], 200.0, 0.0);
        let list = from_boxes(&boxes);
        assert_eq!(list.hit_test(8.0, 4.0), Some("https://ex.test/"));
        assert!(list.hit_test(1000.0, 1000.0).is_none());
        assert!(list.find("HERE").is_some());
        assert!(list.find("zzzz").is_none());
    }

    #[test]
    fn rule_is_a_fill() {
        let mut item = FlowItem::text("", ua_style("hr"));
        item.rule = true;
        let boxes = block_flow(&[item], 200.0, 0.0);
        let list = from_boxes(&boxes);
        assert!(matches!(
            list.ops[0],
            DisplayOp::Fill {
                color: 0x00333333,
                ..
            }
        ));
    }
}
