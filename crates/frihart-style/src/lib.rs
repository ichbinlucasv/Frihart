//! UA + user + author cascade for the first CSS subset.

#![forbid(unsafe_code)]

use frihart_css::{Combinator, Compound, Declaration, Selector, Simple, Stylesheet};

/// Root `em` size. `rem` is this many CSS pixels.
pub const REM_PX: f32 = 16.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    pub display: Display,
    pub color: u32,
    pub background: u32,
    pub font_size: f32,
    pub font_weight: u16,
    pub margin: f32,
    pub padding: f32,
    pub width: Option<f32>,
    pub max_width: Option<f32>,
    pub height: Option<f32>,
    pub border_width: f32,
    pub border_color: u32,
    pub text_align: Align,
    pub line_height_mult: Option<f32>,
    pub line_height_px: Option<f32>,
}

impl Computed {
    pub fn line_height(&self) -> f32 {
        if let Some(px) = self.line_height_px {
            px
        } else {
            self.font_size * self.line_height_mult.unwrap_or(1.4)
        }
    }
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
            font_size: REM_PX,
            font_weight: 400,
            margin: 0.0,
            padding: 0.0,
            width: None,
            max_width: None,
            height: None,
            border_width: 0.0,
            border_color: 0x00333333,
            text_align: Align::Start,
            line_height_mult: None,
            line_height_px: None,
        }
    }
}

/// Element identity used for selector matching.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Element {
    pub tag: String,
    pub id: String,
    pub classes: Vec<String>,
    /// Root → parent.
    pub ancestors: Vec<Element>,
}

impl Element {
    pub fn tag(tag: &str) -> Self {
        Self {
            tag: tag.to_ascii_lowercase(),
            ..Self::default()
        }
    }
}

pub fn ua_style(tag: &str) -> Computed {
    let mut c = Computed::default();
    match tag {
        "h1" => {
            c.font_size = 32.0;
            c.font_weight = 700;
            c.margin = 8.0;
        }
        "h2" => {
            c.font_size = 24.0;
            c.font_weight = 700;
            c.margin = 6.0;
        }
        "h3" => {
            c.font_size = 20.0;
            c.font_weight = 700;
            c.margin = 4.0;
        }
        "blockquote" => {
            c.margin = 12.0;
            c.padding = 8.0;
            c.color = 0x00C8C8C8;
        }
        "pre" | "code" => {
            c.font_size = 14.0;
            c.padding = 8.0;
            c.background = 0x00181818;
        }
        "li" => {
            c.margin = 2.0;
            c.padding = 4.0;
        }
        "img" => {
            c.margin = 8.0;
            c.background = 0x00202020;
        }
        "hr" => {
            c.margin = 8.0;
            c.background = 0x00333333;
        }
        "caption" => {
            c.font_size = 14.0;
            c.margin = 4.0;
            c.text_align = Align::Center;
        }
        "a" => {
            c.display = Display::Inline;
            c.color = 0x00F5C400;
        }
        "span" | "em" => c.display = Display::Inline,
        "strong" | "b" => {
            c.display = Display::Inline;
            c.font_weight = 700;
        }
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
                if let Some(n) =
                    parse_length(&d.value, computed.font_size, Some(computed.font_size))
                {
                    computed.font_size = n.max(1.0);
                }
            }
            "font-weight" => {
                if let Some(w) = parse_weight(&d.value) {
                    computed.font_weight = w;
                }
            }
            "margin" | "margin-top" | "margin-bottom" => {
                if let Some(n) = parse_length(&d.value, computed.font_size, None) {
                    computed.margin = n;
                }
            }
            "padding" => {
                if let Some(n) = parse_length(&d.value, computed.font_size, None) {
                    computed.padding = n;
                }
            }
            "width" => computed.width = parse_length(&d.value, computed.font_size, None),
            "max-width" => computed.max_width = parse_length(&d.value, computed.font_size, None),
            "height" => computed.height = parse_length(&d.value, computed.font_size, None),
            "border" => apply_border(computed, &d.value),
            "border-width" => {
                if let Some(n) = parse_length(&d.value, computed.font_size, None) {
                    computed.border_width = n;
                }
            }
            "border-color" => {
                if let Some(c) = parse_color(&d.value) {
                    computed.border_color = c;
                }
            }
            "line-height" => apply_line_height(computed, &d.value),
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

fn apply_line_height(computed: &mut Computed, value: &str) {
    let v = value.trim();
    if v.eq_ignore_ascii_case("normal") {
        computed.line_height_mult = None;
        computed.line_height_px = None;
        return;
    }
    if let Some(px) = parse_length(v, computed.font_size, None) {
        computed.line_height_px = Some(px);
        computed.line_height_mult = None;
        return;
    }
    if let Ok(n) = v.parse::<f32>() {
        if n > 0.0 {
            computed.line_height_mult = Some(n);
            computed.line_height_px = None;
        }
    }
}

pub fn style_tag(tag: &str, author: &Stylesheet) -> Computed {
    style_element(&Element::tag(tag), &Stylesheet::default(), author)
}

pub fn style_element(el: &Element, user: &Stylesheet, author: &Stylesheet) -> Computed {
    let mut c = ua_style(&el.tag);
    apply_matching(&mut c, user, el);
    apply_matching(&mut c, author, el);
    c
}

fn apply_matching(computed: &mut Computed, sheet: &Stylesheet, el: &Element) {
    for rule in &sheet.rules {
        if rule_matches(rule, el) {
            apply(computed, &rule.declarations);
        }
    }
}

fn rule_matches(rule: &frihart_css::Rule, el: &Element) -> bool {
    if let Some(sel) = &rule.parsed {
        return selector_matches(sel, el);
    }
    let s = rule.selector.trim();
    s == "*" || s.eq_ignore_ascii_case(&el.tag)
}

pub fn selector_matches(sel: &Selector, el: &Element) -> bool {
    if sel.parts.is_empty() {
        return false;
    }
    let last = sel.parts.last().expect("parts");
    if !compound_matches(&last.1, el) {
        return false;
    }
    let mut current_parent = el.ancestors.as_slice();
    for (comb, compound) in sel.parts[..sel.parts.len() - 1].iter().rev() {
        match comb.unwrap_or(Combinator::Descendant) {
            Combinator::Child => {
                let Some((parent, rest)) = current_parent.split_last() else {
                    return false;
                };
                if !compound_matches(compound, parent) {
                    return false;
                }
                current_parent = rest;
            }
            Combinator::Descendant => {
                let mut found = None;
                for (i, anc) in current_parent.iter().enumerate().rev() {
                    if compound_matches(compound, anc) {
                        found = Some(i);
                        break;
                    }
                }
                let Some(i) = found else {
                    return false;
                };
                current_parent = &current_parent[..i];
            }
        }
    }
    true
}

fn compound_matches(compound: &Compound, el: &Element) -> bool {
    compound.simples.iter().all(|s| simple_matches(s, el))
}

fn simple_matches(simple: &Simple, el: &Element) -> bool {
    match simple {
        Simple::Universal => true,
        Simple::Type(t) => el.tag.eq_ignore_ascii_case(t),
        Simple::Class(c) => el.classes.iter().any(|have| have.eq_ignore_ascii_case(c)),
        Simple::Id(id) => el.id.eq_ignore_ascii_case(id),
    }
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

/// `px`, `em`, `rem`, `0`. `%` only when `percent_of` is set (font-size).
pub fn parse_length(value: &str, em: f32, percent_of: Option<f32>) -> Option<f32> {
    let v = value.trim();
    if v == "0" {
        return Some(0.0);
    }
    if let Some(n) = v.strip_suffix("rem") {
        return n.trim().parse::<f32>().ok().map(|x| x * REM_PX);
    }
    if let Some(n) = v.strip_suffix("em") {
        return n.trim().parse::<f32>().ok().map(|x| x * em);
    }
    if let Some(n) = v.strip_suffix("px") {
        return n.trim().parse().ok();
    }
    if let Some(n) = v.strip_suffix('%') {
        let p = n.trim().parse::<f32>().ok()?;
        return percent_of.map(|base| base * p / 100.0);
    }
    None
}

fn parse_weight(value: &str) -> Option<u16> {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" | "lighter" => Some(400),
        "bold" | "bolder" => Some(700),
        n => {
            let w: u16 = n.parse().ok()?;
            if (100..=900).contains(&w) && w % 100 == 0 {
                Some(w)
            } else {
                None
            }
        }
    }
}

fn apply_border(computed: &mut Computed, value: &str) {
    let v = value.trim();
    if v.eq_ignore_ascii_case("none") || v == "0" {
        computed.border_width = 0.0;
        return;
    }
    for token in v.split_whitespace() {
        if matches!(
            token,
            "solid" | "dashed" | "dotted" | "double" | "none" | "hidden"
        ) {
            if token == "none" || token == "hidden" {
                computed.border_width = 0.0;
            }
            continue;
        }
        if let Some(n) = parse_length(token, computed.font_size, None) {
            computed.border_width = n;
            continue;
        }
        if let Some(c) = parse_color(token) {
            computed.border_color = c;
        }
    }
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

    #[test]
    fn class_id_descendant_and_user_origin() {
        let user = parse_stylesheet("p { font-size: 10px }");
        let author = parse_stylesheet(
            "article #x p.lead { font-size: 22px; max-width: 400px; line-height: 1.5 }",
        );
        let el = Element {
            tag: "p".into(),
            id: String::new(),
            classes: vec!["lead".into()],
            ancestors: vec![
                Element {
                    tag: "article".into(),
                    ..Element::default()
                },
                Element {
                    tag: "div".into(),
                    id: "x".into(),
                    ..Element::default()
                },
            ],
        };
        let c = style_element(&el, &user, &author);
        assert_eq!(c.font_size, 22.0);
        assert_eq!(c.max_width, Some(400.0));
        assert!((c.line_height() - 33.0).abs() < 0.01);
    }

    #[test]
    fn ua_hr_and_caption() {
        let hr = ua_style("hr");
        assert_eq!(hr.background, 0x00333333);
        let cap = ua_style("caption");
        assert_eq!(cap.text_align, Align::Center);
        assert_eq!(cap.font_size, 14.0);
        assert_eq!(ua_style("h1").font_weight, 700);
    }

    #[test]
    fn em_rem_weight_border() {
        let sheet = parse_stylesheet(
            "p { font-size: 2em; font-weight: bold; margin: 1rem; border: 1px solid #334455 }",
        );
        let c = style_tag("p", &sheet);
        assert_eq!(c.font_size, 32.0);
        assert_eq!(c.font_weight, 700);
        assert_eq!(c.margin, 16.0);
        assert_eq!(c.border_width, 1.0);
        assert_eq!(c.border_color, 0x00334455);
        let pct = parse_stylesheet("p { font-size: 150% }");
        assert_eq!(style_tag("p", &pct).font_size, 24.0);
    }
}
