//! Chrome metrics and the Ember palette.

/// Packed 0x00RRGGBB, which matches softbuffer on little-endian.
pub type Rgb = u32;

pub const BG_CHROME: Rgb = 0x001A1714;
pub const BG_TAB_ACTIVE: Rgb = 0x002A241F;
pub const BG_TAB_HOVER: Rgb = 0x00231E1A;
pub const BG_TOOLBAR: Rgb = 0x00211C18;
pub const BG_URL: Rgb = 0x0012100E;
pub const BG_URL_FOCUS: Rgb = 0x0014100C;
pub const BG_CONTENT: Rgb = 0x00F4EFE6;
pub const BG_STATUS: Rgb = 0x0016110E;
pub const BG_NOTE: Rgb = 0x00E8DFD0;
pub const BG_TOGGLE_OFF: Rgb = 0x00C9BBA8;
pub const BG_TOGGLE_ON: Rgb = 0x00C47A3A;

pub const ACCENT: Rgb = 0x00C47A3A;
pub const ACCENT_DIM: Rgb = 0x008A5A2E;
pub const TEXT_CHROME: Rgb = 0x00E8E0D4;
pub const TEXT_MUTED: Rgb = 0x009A8F82;
pub const TEXT_CONTENT: Rgb = 0x00241E18;
pub const TEXT_CONTENT_MUTED: Rgb = 0x00685E54;
pub const TEXT_LINK: Rgb = 0x008A4B1A;
pub const HAIRLINE: Rgb = 0x00362E27;
pub const GOOD: Rgb = 0x006B9B6E;
pub const WARN: Rgb = 0x00C4A03A;
pub const BAD: Rgb = 0x00B05A4A;

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
        self.s(720.0)
    }

    pub fn content_pad(&self) -> i32 {
        self.s(48.0)
    }
}
