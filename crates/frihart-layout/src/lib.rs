//! Block flow. Height comes from cosmic-text wrap, not a character guess.

#![forbid(unsafe_code)]

use std::cell::RefCell;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics as CosmicMetrics, Shaping, Wrap};

use frihart_style::{Align, Computed, Display};

#[derive(Clone, Debug)]
pub struct FlowItem {
    pub text: String,
    pub style: Computed,
    pub href: Option<String>,
    pub preserve: bool,
    pub image: bool,
}

impl FlowItem {
    pub fn text(text: impl Into<String>, style: Computed) -> Self {
        Self {
            text: text.into(),
            style,
            href: None,
            preserve: false,
            image: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: String,
    pub style: Computed,
    pub href: Option<String>,
    pub preserve: bool,
    pub image: bool,
}

thread_local! {
    static FONTS: RefCell<FontSystem> = RefCell::new(FontSystem::new());
}

pub fn measure_wrapped(
    text: &str,
    font_size: f32,
    line_height: f32,
    max_width: f32,
    wrap: bool,
) -> f32 {
    if text.is_empty() {
        return line_height.max(1.0);
    }
    FONTS.with(|fonts| {
        let mut fs = fonts.borrow_mut();
        let metrics = CosmicMetrics::new(font_size.max(1.0), line_height.max(1.0));
        let mut buffer = Buffer::new(&mut fs, metrics);
        buffer.set_wrap(&mut fs, if wrap { Wrap::Word } else { Wrap::None });
        buffer.set_size(&mut fs, Some(max_width.max(1.0)), None);
        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(&mut fs, text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut fs, false);
        let mut h = line_height;
        for run in buffer.layout_runs() {
            h = h.max(run.line_y + run.line_height);
        }
        h.max(line_height)
    })
}

pub fn block_flow(items: &[FlowItem], viewport_w: f32, origin_y: f32) -> Vec<LayoutBox> {
    let mut y = origin_y;
    let mut out = Vec::new();
    let vw = viewport_w.max(1.0);
    for item in items {
        if matches!(item.style.display, Display::None) {
            continue;
        }
        let mut width = item.style.width.unwrap_or(vw).min(vw);
        if let Some(max_w) = item.style.max_width {
            width = width.min(max_w);
        }
        let x = match item.style.text_align {
            Align::Start => 0.0,
            Align::Center => ((vw - width) / 2.0).max(0.0),
            Align::End => (vw - width).max(0.0),
        };
        let inner_w = (width - item.style.padding * 2.0).max(1.0);
        let lh = item.style.line_height();
        let text_h = if item.image {
            72.0
        } else {
            measure_wrapped(
                &item.text,
                item.style.font_size,
                lh,
                inner_w,
                !item.preserve,
            )
        };
        let h = text_h + item.style.padding * 2.0;
        out.push(LayoutBox {
            x,
            y: y + item.style.margin,
            w: width,
            h,
            text: item.text.clone(),
            style: item.style.clone(),
            href: item.href.clone(),
            preserve: item.preserve,
            image: item.image,
        });
        y += item.style.margin + h + item.style.margin;
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
        let items = vec![
            FlowItem::text("A", ua_style("h1")),
            FlowItem::text("B", ua_style("p")),
        ];
        let boxes = block_flow(&items, 400.0, 0.0);
        assert_eq!(boxes.len(), 2);
        assert!(boxes[1].y > boxes[0].y);
        assert!(content_height(&boxes) > boxes[1].y);
    }

    #[test]
    fn skips_display_none() {
        let mut hidden = ua_style("script");
        hidden.display = Display::None;
        let boxes = block_flow(&[FlowItem::text("x", hidden)], 200.0, 0.0);
        assert!(boxes.is_empty());
    }

    #[test]
    fn wrap_is_taller_than_one_line() {
        let style = ua_style("p");
        let long = "word ".repeat(80);
        let wide = measure_wrapped(&long, style.font_size, style.line_height(), 800.0, true);
        let narrow = measure_wrapped(&long, style.font_size, style.line_height(), 80.0, true);
        assert!(narrow > wide + style.line_height() * 0.5);
    }
}
