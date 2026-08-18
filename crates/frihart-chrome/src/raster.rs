//! Software framebuffer. No GPU, no unsafe.

use crate::theme::Rgb;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }

    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub fn inset(self, n: i32) -> Self {
        Self {
            x: self.x + n,
            y: self.y + n,
            w: (self.w - 2 * n).max(0),
            h: (self.h - 2 * n).max(0),
        }
    }
}

pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
        }
    }

    pub fn fill(&mut self, color: Rgb) {
        self.pixels.fill(color);
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Rgb) {
        if rect.w <= 0 || rect.h <= 0 {
            return;
        }
        let x0 = rect.x.max(0) as u32;
        let y0 = rect.y.max(0) as u32;
        let x1 = (rect.right()).max(0) as u32;
        let y1 = (rect.bottom()).max(0) as u32;
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for y in y0..y1 {
            let start = (y * self.width + x0) as usize;
            let end = (y * self.width + x1) as usize;
            self.pixels[start..end].fill(color);
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Rgb) {
        if rect.w <= 0 || rect.h <= 0 {
            return;
        }
        self.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1), color);
        self.fill_rect(Rect::new(rect.x, rect.bottom() - 1, rect.w, 1), color);
        self.fill_rect(Rect::new(rect.x, rect.y, 1, rect.h), color);
        self.fill_rect(Rect::new(rect.right() - 1, rect.y, 1, rect.h), color);
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb, alpha: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y * self.width + x) as usize;
        let dst = self.pixels[idx];
        self.pixels[idx] = blend(dst, color, alpha);
    }

    pub fn blend_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Rgb, alpha: u8) {
        for dy in 0..h as i32 {
            for dx in 0..w as i32 {
                self.blend_pixel(x + dx, y + dy, color, alpha);
            }
        }
    }
}

fn blend(dst: Rgb, src: Rgb, alpha: u8) -> Rgb {
    if alpha == 0 {
        return dst;
    }
    if alpha == 255 {
        return src;
    }
    let a = u32::from(alpha);
    let ia = 255 - a;
    let dr = (dst >> 16) & 0xff;
    let dg = (dst >> 8) & 0xff;
    let db = dst & 0xff;
    let sr = (src >> 16) & 0xff;
    let sg = (src >> 8) & 0xff;
    let sb = src & 0xff;
    let r = (sr * a + dr * ia) / 255;
    let g = (sg * a + dg * ia) / 255;
    let b = (sb * a + db * ia) / 255;
    (r << 16) | (g << 8) | b
}
