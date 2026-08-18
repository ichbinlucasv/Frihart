//! UA + author cascade for the first CSS subset.

#![forbid(unsafe_code)]

use frihart_css::{Declaration, Stylesheet};

#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    pub display: Display,
    pub color: u32,
    pub background: u32,
    pub font_size: f32,
    pub margin: f32,
    pub padding: f32,
    pub width: Option<f32>,
    pub text_align: Align,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

impl Default for Computed {
    fn default() -> Self {
        Self {
            display: Display::Block,
            color: 0x00F2F2F2,
            background: 0,
            font_size: 16.0,
            margin: 0.0,
            padding: 0.0,
            width: None,
            text_align: Align::Start,
        }
    }
}

pub fn ua_style(tag: &str) -> Computed {
    let mut c = Computed::default();
    match tag {
        "h1" => {
            c.font_size = 32.0;
            c.margin = 8.0;
        }
        "h2" => {
            c.font_size = 24.0;
            c.margin = 6.0;
        }
        "h3" => {
            c.font_size = 20.0;
            c.margin = 4.0;
        }
        "a" => {
            c.display = Display::Inline;
            c.color = 0x00F5C400;
        }
        "span" | "em" | "strong" => c.display = Display::Inline,
        "script" | "style" | "head" => c.display = Display::None,
        _ => {}
    }
    c
}

pub fn apply(computed: &mut Computed, decls: &[Declaration]) {
    for d in decls {
        match d.name.as_str() {
            "color" => {
                if let Some(c) = parse_color(&d.value) {
                    computed.color = c;
                }
            }
            "background-color" | "background" => {
                if let Some(c) = parse_color(&d.value) {
                    computed.background = c;
                }
            }
            "display" if d.value == "none" => computed.display = Display::None,
            "display" if d.value == "inline" => computed.display = Display::Inline,
            "display" if d.value == "block" => computed.display = Display::Block,
            "font-size" => {
                if let Some(n) = parse_px(&d.value) {
                    computed.font_size = n;
                }
            }
            "margin" | "margin-top" | "margin-bottom" => {
                if let Some(n) = parse_px(&d.value) {
                    computed.margin = n;
                }
            }
            "padding" => {
                if let Some(n) = parse_px(&d.value) {
                    computed.padding = n;
                }
            }
            "width" => computed.width = parse_px(&d.value),
            "text-align" => {
                computed.text_align = match d.value.as_str() {
                    "center" => Align::Center,
                    "right" | "end" => Align::End,
                    _ => Align::Start,
                };
            }
            _ => {}
        }
    }
}

pub fn style_tag(tag: &str, author: &Stylesheet) -> Computed {
    let mut c = ua_style(tag);
    for rule in &author.rules {
        if selector_matches(&rule.selector, tag) {
            apply(&mut c, &rule.declarations);
        }
    }
    c
}

fn selector_matches(selector: &str, tag: &str) -> bool {
    let sel = selector.trim();
    sel == "*" || sel == tag
}

pub fn parse_color(value: &str) -> Option<u32> {
    let v = value.trim().to_ascii_lowercase();
    match v.as_str() {
        "red" => Some(0x00C04040),
        "black" => Some(0x00000000),
        "white" => Some(0x00F2F2F2),
        "yellow" => Some(0x00F5C400),
        "transparent" => Some(0),
        hex if hex.starts_with('#') => parse_hex(&hex[1..]),
        _ => None,
    }
}

fn parse_hex(hex: &str) -> Option<u32> {
    match hex.len() {
        3 => {
            let r = u32::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u32::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u32::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r << 16) | (g << 8) | b)
        }
        6 => u32::from_str_radix(hex, 16).ok(),
        _ => None,
    }
}

fn parse_px(value: &str) -> Option<f32> {
    let v = value.trim();
    if let Some(n) = v.strip_suffix("px") {
        return n.trim().parse().ok();
    }
    if v == "0" {
        return Some(0.0);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_css::parse_stylesheet;

    #[test]
    fn author_overrides_ua() {
        let sheet = parse_stylesheet("p { font-size: 20px; color: #112233 }");
        let c = style_tag("p", &sheet);
        assert_eq!(c.font_size, 20.0);
        assert_eq!(c.color, 0x00112233);
    }

    #[test]
    fn star_selector_applies() {
        let sheet = parse_stylesheet("* { padding: 4px }");
        let c = style_tag("div", &sheet);
        assert_eq!(c.padding, 4.0);
    }
}
