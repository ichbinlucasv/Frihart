//! Chrome metrics and the black / yellow palette.

/// Packed 0x00RRGGBB, which matches softbuffer on little-endian.
pub type Rgb = u32;

pub const BG_CHROME: Rgb = 0x00000000;
pub const BG_TAB_ACTIVE: Rgb = 0x00161616;
pub const BG_TAB_HOVER: Rgb = 0x00202020;
pub const BG_TOOLBAR: Rgb = 0x000A0A0A;
pub const BG_URL: Rgb = 0x00000000;
pub const BG_URL_FOCUS: Rgb = 0x001A1400;
pub const BG_CONTENT: Rgb = 0x000A0A0A;
pub const BG_STATUS: Rgb = 0x00000000;
pub const BG_NOTE: Rgb = 0x001A1600;
pub const BG_TOGGLE_OFF: Rgb = 0x00333333;
pub const BG_TOGGLE_ON: Rgb = 0x00F5C400;
pub const BG_FIND: Rgb = 0x00141414;

pub const ACCENT: Rgb = 0x00F5C400;
pub const ACCENT_DIM: Rgb = 0x00B38F00;
pub const TEXT_CHROME: Rgb = 0x00F5C400;
pub const TEXT_MUTED: Rgb = 0x00A09040;
pub const TEXT_CONTENT: Rgb = 0x00F2F2F2;
pub const TEXT_CONTENT_MUTED: Rgb = 0x00B8B8B8;
pub const TEXT_LINK: Rgb = 0x00F5C400;
pub const HAIRLINE: Rgb = 0x002A2A2A;
pub const GOOD: Rgb = 0x00F5C400;
pub const WARN: Rgb = 0x00E6B800;
pub const BAD: Rgb = 0x00C04040;
pub const PRIVATE: Rgb = 0x00F5C400;

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub scale: f32,
}

impl Metrics {
    pub fn new(scale: f32) -> Self {
        Self {
            scale: scale.max(1.0),
        }
    }

    pub(crate) fn s(&self, logical: f32) -> i32 {
        (logical * self.scale).round() as i32
    }

    pub fn tab_h(&self) -> i32 {
        self.s(36.0)
    }

    pub fn toolbar_h(&self) -> i32 {
        self.s(44.0)
    }

    pub fn status_h(&self) -> i32 {
        self.s(24.0)
    }

    pub fn find_h(&self) -> i32 {
        self.s(32.0)
    }

    pub fn pad(&self) -> i32 {
        self.s(10.0)
    }

    pub fn btn(&self) -> i32 {
        self.s(28.0)
    }

    pub fn url_h(&self) -> i32 {
        self.s(30.0)
    }

    pub fn font_ui(&self) -> f32 {
        13.0 * self.scale
    }

    pub fn font_ui_sm(&self) -> f32 {
        12.0 * self.scale
    }

    pub fn font_content(&self) -> f32 {
        16.0 * self.scale
    }

    pub fn font_heading(&self) -> f32 {
        22.0 * self.scale
    }

    pub fn font_hero(&self) -> f32 {
        32.0 * self.scale
    }

    pub fn line_ui(&self) -> f32 {
        18.0 * self.scale
    }

    pub fn line_content(&self) -> f32 {
        24.0 * self.scale
    }

    pub fn content_max_w(&self) -> i32 {
        self.s(2400.0)
    }

    pub fn content_min_w(&self) -> i32 {
        self.s(320.0)
    }

    pub fn content_pad(&self) -> i32 {
        self.s(48.0)
    }
}
