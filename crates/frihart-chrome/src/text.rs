//! Text shaping via cosmic-text.

use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics as CosmicMetrics, Shaping, SwashCache,
    Weight, Wrap,
};

use crate::raster::Framebuffer;
use crate::theme::Rgb;

pub struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

/// How a string should be painted.
pub struct DrawText {
    pub x: i32,
    pub y: i32,
    pub max_width: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub color: Rgb,
    pub weight: Weight,
    pub ellipsis: bool,
    pub wrap: bool,
}

impl TextEngine {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    fn attrs(weight: Weight) -> Attrs<'static> {
        Attrs::new().family(Family::SansSerif).weight(weight)
    }

    pub fn draw(&mut self, fb: &mut Framebuffer, text: &str, spec: DrawText) -> (i32, i32) {
        let DrawText {
            x,
            y,
            max_width,
            font_size,
            line_height,
            color,
            weight,
            ellipsis,
            wrap,
        } = spec;
        if text.is_empty() || max_width <= 1.0 {
            return (0, line_height.round() as i32);
        }

        let metrics = CosmicMetrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_wrap(
            &mut self.font_system,
            if wrap && !ellipsis {
                Wrap::Word
            } else {
                Wrap::None
            },
        );
        let height = if ellipsis { Some(line_height) } else { None };
        buffer.set_size(&mut self.font_system, Some(max_width), height);

        let display = if ellipsis {
            ellipsize(
                &mut self.font_system,
                text,
                max_width,
                font_size,
                line_height,
                weight,
            )
        } else {
            text.to_string()
        };

        let attrs = Self::attrs(weight);
        set_buffer_text(&mut buffer, &mut self.font_system, &display, &attrs);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let (r, g, b) = rgb_parts(color);
        let cosmic_color = Color::rgb(r, g, b);
        let mut drawn_w = 0.0f32;
        let mut drawn_h = line_height;

        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            cosmic_color,
            |gx, gy, w, h, c| {
                let alpha = c.a();
                if alpha == 0 {
                    return;
                }
                let src = rgb(c.r(), c.g(), c.b());
                fb.blend_rect(x + gx, y + gy, w, h, src, alpha);
            },
        );

        for run in buffer.layout_runs() {
            drawn_w = drawn_w.max(run.line_w);
            drawn_h = drawn_h.max(run.line_y + run.line_height);
        }
        (drawn_w.ceil() as i32, drawn_h.ceil() as i32)
    }

    pub fn measure_width(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        weight: Weight,
    ) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let metrics = CosmicMetrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_wrap(&mut self.font_system, Wrap::None);
        buffer.set_size(&mut self.font_system, Some(4000.0), Some(line_height));
        let attrs = Self::attrs(weight);
        set_buffer_text(&mut buffer, &mut self.font_system, text, &attrs);
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
            .layout_runs()
            .next()
            .map(|run| run.line_w)
            .unwrap_or(0.0)
    }
}

fn set_buffer_text(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    text: &str,
    attrs: &Attrs<'static>,
) {
    buffer.set_text(font_system, text, attrs, Shaping::Advanced);
}

fn ellipsize(
    font_system: &mut FontSystem,
    text: &str,
    max_width: f32,
    font_size: f32,
    line_height: f32,
    weight: Weight,
) -> String {
    let metrics = CosmicMetrics::new(font_size, line_height);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_wrap(font_system, Wrap::None);
    buffer.set_size(font_system, Some(4000.0), Some(line_height));
    let attrs = TextEngine::attrs(weight);
    set_buffer_text(&mut buffer, font_system, text, &attrs);
    buffer.shape_until_scroll(font_system, false);
    let width = buffer.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
    if width <= max_width {
        return text.to_string();
    }
    let mut end = text.chars().count();
    while end > 1 {
        end -= 1;
        let candidate: String = text.chars().take(end).collect::<String>() + "…";
        set_buffer_text(&mut buffer, font_system, &candidate, &attrs);
        buffer.shape_until_scroll(font_system, false);
        let w = buffer.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
        if w <= max_width {
            return candidate;
        }
    }
    "…".into()
}

fn rgb_parts(color: Rgb) -> (u8, u8, u8) {
    (
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    )
}

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}
