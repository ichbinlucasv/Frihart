//! Block flow. Flex/grid come later.

#![forbid(unsafe_code)]

use frihart_style::{Align, Computed, Display};

#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: String,
    pub style: Computed,
}

pub fn block_flow(items: &[(String, Computed)], viewport_w: f32, origin_y: f32) -> Vec<LayoutBox> {
    let mut y = origin_y;
    let mut out = Vec::new();
    let vw = viewport_w.max(1.0);
    for (text, style) in items {
        if matches!(style.display, Display::None) {
            continue;
        }
        let width = style.width.unwrap_or(vw).min(vw);
        let x = match style.text_align {
            Align::Start => 0.0,
            Align::Center => ((vw - width) / 2.0).max(0.0),
            Align::End => (vw - width).max(0.0),
        };
        let char_w = (style.font_size * 0.5).max(1.0);
        let cols = (width / char_w).max(1.0);
        let lines = ((text.len() as f32 / cols).ceil()).max(1.0);
        let h = style.font_size * 1.4 * lines + style.margin * 2.0 + style.padding * 2.0;
        out.push(LayoutBox {
            x,
            y: y + style.margin,
            w: width,
            h,
            text: text.clone(),
            style: style.clone(),
        });
        y += h + style.margin;
    }
    out
}

pub fn content_height(boxes: &[LayoutBox]) -> f32 {
    boxes.iter().map(|b| b.y + b.h).fold(0.0_f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_style::ua_style;

    #[test]
    fn stacks_blocks() {
        let items = vec![("A".into(), ua_style("h1")), ("B".into(), ua_style("p"))];
        let boxes = block_flow(&items, 400.0, 0.0);
        assert_eq!(boxes.len(), 2);
        assert!(boxes[1].y > boxes[0].y);
        assert!(content_height(&boxes) > boxes[1].y);
    }

    #[test]
    fn skips_display_none() {
        let mut hidden = ua_style("script");
        hidden.display = Display::None;
        let boxes = block_flow(&[("x".into(), hidden)], 200.0, 0.0);
        assert!(boxes.is_empty());
    }
}
