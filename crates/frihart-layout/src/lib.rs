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
    /// Non-empty: this item is a table row. Cells are laid out in columns.
    pub cells: Vec<String>,
    pub field: Option<FieldSlot>,
}

#[derive(Clone, Debug)]
pub struct FieldSlot {
    pub index: usize,
    pub secret: bool,
}

impl FlowItem {
    pub fn text(text: impl Into<String>, style: Computed) -> Self {
        Self {
            text: text.into(),
            style,
            href: None,
            preserve: false,
            image: false,
            cells: Vec::new(),
            field: None,
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
    pub cell: bool,
    pub field: Option<FieldSlot>,
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
    let mut i = 0;
    while i < items.len() {
        if matches!(items[i].style.display, Display::None) {
            i += 1;
            continue;
        }
        if !items[i].cells.is_empty() {
            let start = i;
            while i < items.len()
                && !items[i].cells.is_empty()
                && !matches!(items[i].style.display, Display::None)
            {
                i += 1;
            }
            let (boxes, next_y) = layout_table(&items[start..i], vw, y);
            out.extend(boxes);
            y = next_y;
            continue;
        }
        out.push(layout_block(&items[i], vw, y));
        let last = out.last().expect("just pushed");
        y = last.y + last.h + items[i].style.margin;
        i += 1;
    }
    out
}

fn layout_block(item: &FlowItem, vw: f32, y: f32) -> LayoutBox {
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
    let mut h = text_h + item.style.padding * 2.0;
    if item.field.is_some() {
        h += lh + 20.0;
    }
    LayoutBox {
        x,
        y: y + item.style.margin,
        w: width,
        h,
        text: item.text.clone(),
        style: item.style.clone(),
        href: item.href.clone(),
        preserve: item.preserve,
        image: item.image,
        cell: false,
        field: item.field.clone(),
    }
}

fn layout_table(rows: &[FlowItem], vw: f32, mut y: f32) -> (Vec<LayoutBox>, f32) {
    let cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0).max(1);
    let gap = 2.0;
    let col_w = ((vw - gap * (cols.saturating_sub(1) as f32)) / cols as f32).max(8.0);
    let mut out = Vec::new();
    for row in rows {
        let pad = row.style.padding.max(4.0);
        let lh = row.style.line_height();
        let inner = (col_w - pad * 2.0).max(1.0);
        let mut row_h = lh + pad * 2.0;
        for cell in 0..cols {
            let text = row.cells.get(cell).cloned().unwrap_or_default();
            let h = measure_wrapped(&text, row.style.font_size, lh, inner, true) + pad * 2.0;
            row_h = row_h.max(h);
        }
        for cell in 0..cols {
            let text = row.cells.get(cell).cloned().unwrap_or_default();
            out.push(LayoutBox {
                x: cell as f32 * (col_w + gap),
                y: y + row.style.margin,
                w: col_w,
                h: row_h,
                text,
                style: row.style.clone(),
                href: None,
                preserve: false,
                image: false,
                cell: true,
                field: None,
            });
        }
        y += row.style.margin + row_h + row.style.margin;
    }
    (out, y)
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

    #[test]
    fn table_cells_sit_in_columns() {
        let mut row = FlowItem::text("", ua_style("td"));
        row.cells = vec!["left".into(), "right".into()];
        let boxes = block_flow(&[row.clone(), row], 400.0, 0.0);
        assert_eq!(boxes.len(), 4);
        assert!(boxes[0].cell && boxes[1].cell);
        assert!(boxes[1].x > boxes[0].x);
        assert!((boxes[0].y - boxes[1].y).abs() < 0.5);
        assert!(boxes[2].y > boxes[0].y);
        assert_eq!(boxes[0].text, "left");
        assert_eq!(boxes[1].text, "right");
    }
}
