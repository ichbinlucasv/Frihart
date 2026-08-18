//! Display list. Chrome paints these ops; GPU comes later.

#![forbid(unsafe_code)]

use frihart_layout::LayoutBox;

#[derive(Clone, Debug)]
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
        text: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub ops: Vec<DisplayOp>,
}

impl DisplayList {
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }
}

pub fn from_boxes(boxes: &[LayoutBox]) -> DisplayList {
    let mut list = DisplayList::default();
    for b in boxes {
        if b.style.background != 0 {
            list.ops.push(DisplayOp::Fill {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
                color: b.style.background,
            });
        }
        list.ops.push(DisplayOp::Text {
            x: b.x + b.style.padding,
            y: b.y + b.style.padding,
            color: b.style.color,
            size: b.style.font_size,
            text: b.text.clone(),
        });
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_layout::block_flow;
    use frihart_style::ua_style;

    #[test]
    fn one_text_op_per_box() {
        let boxes = block_flow(&[("Hi".into(), ua_style("p"))], 100.0, 0.0);
        let list = from_boxes(&boxes);
        assert_eq!(list.len(), 1);
        assert!(matches!(list.ops[0], DisplayOp::Text { .. }));
    }
}
