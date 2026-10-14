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

    /// Copy an RGBA bitmap (`w` × `h`, 4 bytes per pixel) with its top-left
    /// at (`x`, `y`), clipped to the framebuffer and to `clip`. Alpha is
    /// ignored (Servo frames are opaque). Does nothing if `rgba` is short.
    pub fn blit_rgba(&mut self, x: i32, y: i32, w: u32, h: u32, rgba: &[u8], clip: Rect) {
        let need = (w as usize)
            .checked_mul(h as usize)
            .and_then(|n| n.checked_mul(4));
        if need != Some(rgba.len()) || w == 0 || h == 0 {
            return;
        }
        let cx0 = clip.x.max(0).max(x);
        let cy0 = clip.y.max(0).max(y);
        let cx1 = clip.right().min(self.width as i32).min(x + w as i32);
        let cy1 = clip.bottom().min(self.height as i32).min(y + h as i32);
        if cx0 >= cx1 || cy0 >= cy1 {
            return;
        }
        for dy in cy0..cy1 {
            let src_row = (dy - y) as usize * w as usize;
            let dst_row = dy as usize * self.width as usize;
            for dx in cx0..cx1 {
                let si = (src_row + (dx - x) as usize) * 4;
                let px =
                    ((rgba[si] as u32) << 16) | ((rgba[si + 1] as u32) << 8) | rgba[si + 2] as u32;
                self.pixels[dst_row + dx as usize] = px;
            }
        }
    }

    /// Fill with rounded corners (radius in px). Used for pills and tabs.
    pub fn fill_round_rect(&mut self, rect: Rect, radius: i32, color: Rgb) {
        let r = radius.clamp(0, rect.w.min(rect.h) / 2);
        if r == 0 {
            self.fill_rect(rect, color);
            return;
        }
        for dy in 0..rect.h {
            let inset = if dy < r {
                corner_inset(r, r - dy)
            } else if dy >= rect.h - r {
                corner_inset(r, dy - (rect.h - r) + 1)
            } else {
                0
            };
            self.fill_rect(
                Rect::new(rect.x + inset, rect.y + dy, rect.w - 2 * inset, 1),
                color,
            );
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

/// Horizontal inset for a row `d` px into a corner of radius `r`.
fn corner_inset(r: i32, d: i32) -> i32 {
    let y = d as f32 - 0.5;
    let x = ((r * r) as f32 - y * y).max(0.0).sqrt();
    (r as f32 - x).round() as i32
}

#[cfg(test)]
mod blit_tests {
    use super::*;

    #[test]
    fn blit_copies_rgb_and_clips() {
        let mut fb = Framebuffer::new(4, 4);
        fb.fill(0);
        // 2×2 red/green/blue/white bitmap at (3, 3): only the top-left pixel fits.
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        fb.blit_rgba(3, 3, 2, 2, &rgba, Rect::new(0, 0, 4, 4));
        assert_eq!(fb.pixels[15], 0x00FF_0000);
        assert_eq!(fb.pixels.iter().filter(|p| **p != 0).count(), 1);

        fb.fill(0);
        fb.blit_rgba(-1, 0, 2, 2, &rgba, Rect::new(0, 0, 4, 4));
        assert_eq!(fb.pixels[0], 0x0000_FF00, "left column clipped off");
        assert_eq!(fb.pixels[4], 0x00FF_FFFF);

        fb.fill(0);
        fb.blit_rgba(0, 0, 2, 2, &rgba[..15], Rect::new(0, 0, 4, 4));
        assert!(fb.pixels.iter().all(|p| *p == 0), "short buffer ignored");
        fb.blit_rgba(0, 0, 2, 2, &rgba, Rect::new(0, 0, 1, 1));
        assert_eq!(
            fb.pixels.iter().filter(|p| **p != 0).count(),
            1,
            "clip rect"
        );
    }
}
