//! Paint chrome + internal pages, and collect hit regions.

use cosmic_text::Weight;

use frihart_content::{Block, Document, PageItem};
use frihart_core::display_url;
use frihart_gfx::DisplayOp;
use frihart_pipeline::layout_html;
use frihart_privacy::letterbox_size;

use crate::raster::{Framebuffer, Rect};
use crate::state::{Browser, Circuit, Hit};
use crate::text::{DrawText, TextEngine};
use crate::theme::{
    ACCENT, ACCENT_DIM, BAD, BG_CHROME, BG_CONTENT, BG_FIND, BG_NOTE, BG_STATUS, BG_TAB_ACTIVE,
    BG_TAB_HOVER, BG_TOGGLE_OFF, BG_TOGGLE_ON, BG_TOOLBAR, BG_URL, BG_URL_FOCUS, GOOD, HAIRLINE,
    Metrics, NET_DIRECT, NET_I2P, NET_TOR, PRIVATE, TEXT_CHROME, TEXT_CONTENT, TEXT_CONTENT_MUTED,
    TEXT_LINK, TEXT_MUTED, WARN,
};

pub struct HitRegion {
    pub rect: Rect,
    pub hit: Hit,
}

fn css_weight(weight: u16, link: bool) -> Weight {
    if weight >= 600 {
        Weight(weight)
    } else if link {
        Weight::MEDIUM
    } else {
        Weight(weight.max(1))
    }
}

pub fn paint(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    scale: f32,
) -> Vec<HitRegion> {
    let m = Metrics::new(scale);
    let mut hits = Vec::new();
    let w = fb.width as i32;
    let h = fb.height as i32;

    fb.fill(BG_CHROME);

    let tab_h = m.tab_h();
    let toolbar_h = m.toolbar_h();
    let status_h = if browser.profile.prefs().general.show_status_bar {
        m.status_h()
    } else {
        0
    };
    let find_h = if browser.find_open { m.find_h() } else { 0 };

    let content = Rect::new(
        0,
        tab_h + toolbar_h,
        w,
        (h - tab_h - toolbar_h - status_h - find_h).max(0),
    );
    let outer = content;
    let mut letterbox = None;
    let content = if browser.profile.prefs().privacy.letterboxing {
        let (iw, ih) = letterbox_size(content.w, content.h);
        let x = content.x + (content.w - iw) / 2;
        let y = content.y + (content.h - ih) / 2;
        fb.fill_rect(content, BG_CHROME);
        letterbox = Some((iw, ih));
        Rect::new(x, y, iw, ih)
    } else {
        content
    };
    paint_content(fb, text, browser, &m, content, &mut hits);
    if letterbox.is_some() && (content.w < outer.w || content.h < outer.h) {
        // Clip page overflow into the letterbox margin, then draw the frame.
        fb.fill_rect(
            Rect::new(outer.x, outer.y, outer.w, content.y - outer.y),
            BG_CHROME,
        );
        fb.fill_rect(
            Rect::new(
                outer.x,
                content.bottom(),
                outer.w,
                outer.bottom() - content.bottom(),
            ),
            BG_CHROME,
        );
        fb.fill_rect(
            Rect::new(outer.x, content.y, content.x - outer.x, content.h),
            BG_CHROME,
        );
        fb.fill_rect(
            Rect::new(
                content.right(),
                content.y,
                outer.right() - content.right(),
                content.h,
            ),
            BG_CHROME,
        );
        paint_letterbox_outline(fb, text, &m, content);
    }
    paint_fail_closed_card(fb, text, &m, browser.active_tab().circuit, content);
    // Chrome after content: page overflow can never paint over the tab strip / toolbar.
    paint_tabs(fb, text, browser, &m, w, tab_h, &mut hits);
    paint_toolbar(
        fb,
        text,
        browser,
        &m,
        Rect::new(0, tab_h, w, toolbar_h),
        &mut hits,
    );
    if browser.shields_open {
        paint_shields_panel(fb, text, browser, &m, outer, letterbox, &mut hits);
    }

    if find_h > 0 {
        paint_find(
            fb,
            text,
            browser,
            &m,
            Rect::new(0, h - status_h - find_h, w, find_h),
            &mut hits,
        );
    }

    if status_h > 0 {
        paint_status(
            fb,
            text,
            browser,
            &m,
            Rect::new(0, h - status_h, w, status_h),
        );
    }

    if browser.is_private() {
        fb.fill_rect(Rect::new(0, 0, m.s(4.0), h), PRIVATE);
    }

    hits
}

fn paint_tabs(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    width: i32,
    tab_h: i32,
    hits: &mut Vec<HitRegion>,
) {
    fb.fill_rect(Rect::new(0, 0, width, tab_h), BG_CHROME);
    let pad = m.pad();
    let mut x = pad;
    let plus_w = m.btn();

    let available = width - pad * 2 - plus_w - 8;
    let n = browser.tabs.len().max(1);
    let tab_w = (available / n as i32).clamp(m.s(88.0), m.s(220.0));

    for (i, tab) in browser.tabs.iter().enumerate() {
        let rect = Rect::new(x, 4, tab_w - 4, tab_h - 4);
        let active = i == browser.active;
        let hovered = matches!(browser.hover, Some(Hit::Tab(idx)) if idx == i);
        let bg = if active {
            BG_TAB_ACTIVE
        } else if hovered {
            BG_TAB_HOVER
        } else {
            BG_CHROME
        };
        fb.fill_round_rect(rect, m.s(8.0), bg);
        // Square off the bottom so the active tab joins the toolbar.
        fb.fill_rect(
            Rect::new(rect.x, rect.bottom() - m.s(8.0), rect.w, m.s(8.0)),
            bg,
        );
        if active {
            fb.fill_rect(
                Rect::new(rect.x + 6, rect.bottom() - 2, rect.w - 12, 2),
                ACCENT,
            );
        }
        let stripe = browser
            .profile
            .container(tab.container)
            .map(|c| c.color)
            .unwrap_or(ACCENT);
        fb.fill_rect(Rect::new(rect.x, rect.y + 8, m.s(3.0), rect.h - 16), stripe);

        let pill_w = paint_net_pill(fb, text, m, tab.circuit, rect.x + 10, rect.y + rect.h / 2);
        let close_w = m.s(18.0);
        let label_x = rect.x + 10 + pill_w + 6;
        let label_w = (rect.right() - close_w - 10 - label_x).max(8) as f32;
        text.draw(
            fb,
            &tab.title(),
            DrawText {
                x: label_x,
                y: rect.y + (rect.h - m.line_ui() as i32) / 2,
                max_width: label_w,
                font_size: m.font_ui(),
                line_height: m.line_ui(),
                color: if active { TEXT_CHROME } else { TEXT_MUTED },
                weight: if active {
                    Weight::MEDIUM
                } else {
                    Weight::NORMAL
                },
                ellipsis: true,
                wrap: false,
            },
        );

        let close = Rect::new(
            rect.right() - close_w - 6,
            rect.y + (rect.h - close_w) / 2,
            close_w,
            close_w,
        );
        if matches!(browser.hover, Some(Hit::CloseTab(idx)) if idx == i) {
            fb.fill_rect(close, BAD);
        }
        text.draw(
            fb,
            "×",
            DrawText {
                x: close.x + 4,
                y: close.y,
                max_width: close.w as f32,
                font_size: m.font_ui(),
                line_height: m.line_ui(),
                color: TEXT_CHROME,
                weight: Weight::NORMAL,
                ellipsis: false,
                wrap: false,
            },
        );
        hits.push(HitRegion {
            rect: close,
            hit: Hit::CloseTab(i),
        });
        hits.push(HitRegion {
            rect,
            hit: Hit::Tab(i),
        });
        x += tab_w;
        if x > width - plus_w - pad {
            break;
        }
    }

    let plus = Rect::new(width - pad - plus_w, (tab_h - plus_w) / 2, plus_w, plus_w);
    fb.fill_rect(
        plus,
        if matches!(browser.hover, Some(Hit::NewTab)) {
            BG_TAB_HOVER
        } else {
            BG_CHROME
        },
    );
    text.draw(
        fb,
        "+",
        DrawText {
            x: plus.x + 8,
            y: plus.y + 2,
            max_width: plus.w as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: TEXT_CHROME,
            weight: Weight::MEDIUM,
            ellipsis: false,
            wrap: false,
        },
    );
    hits.push(HitRegion {
        rect: plus,
        hit: Hit::NewTab,
    });
    fb.fill_rect(Rect::new(0, tab_h - 1, width, 1), HAIRLINE);
}

fn paint_toolbar(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    rect: Rect,
    hits: &mut Vec<HitRegion>,
) {
    fb.fill_rect(rect, BG_TOOLBAR);
    let pad = m.pad();
    let btn = m.btn();
    let y = rect.y + (rect.h - btn) / 2;
    let mut x = pad;

    let tab = browser.active_tab();
    let buttons = [
        ("‹", Hit::Back, tab.session.can_go_back()),
        ("›", Hit::Forward, tab.session.can_go_forward()),
        ("↻", Hit::Reload, true),
        (
            container_glyph(browser),
            Hit::ContainerBadge,
            browser.profile.prefs().privacy.containers,
        ),
    ];
    for (label, hit, enabled) in buttons {
        let r = Rect::new(x, y, btn, btn);
        let hovered = matches!(
            (&hit, &browser.hover),
            (Hit::Back, Some(Hit::Back))
                | (Hit::Forward, Some(Hit::Forward))
                | (Hit::Reload, Some(Hit::Reload))
                | (Hit::ContainerBadge, Some(Hit::ContainerBadge))
        );
        if hovered && enabled {
            fb.fill_rect(r, BG_TAB_HOVER);
        }
        text.draw(
            fb,
            label,
            DrawText {
                x: r.x + 6,
                y: r.y + 4,
                max_width: r.w as f32,
                font_size: m.font_ui(),
                line_height: m.line_ui(),
                color: if enabled { TEXT_CHROME } else { TEXT_MUTED },
                weight: Weight::NORMAL,
                ellipsis: false,
                wrap: false,
            },
        );
        if enabled {
            hits.push(HitRegion { rect: r, hit });
        }
        x += btn + 4;
    }

    let badge = Rect::new(rect.right() - pad - btn, y, btn, btn);
    let (dot, _) = privacy_dot(browser);
    let d = badge.inset(8);
    fb.fill_round_rect(d, d.w / 2, dot);
    hits.push(HitRegion {
        rect: badge,
        hit: Hit::PrivacyBadge,
    });

    // Campaign S M2 polish: Reader / Compat cue (Shift+E cycles).
    let mode_w = m.s(28.0);
    let mode_rect = Rect::new(
        badge.x - 6 - mode_w,
        y + (btn - m.s(18.0)) / 2,
        mode_w,
        m.s(18.0),
    );
    let (mode_label, mode_good) = match browser.active_tab().mode {
        frihart_engine::TabMode::Reader => ("R", true),
        frihart_engine::TabMode::Compat => ("C", frihart_engine_servo::AVAILABLE),
    };
    let (mbg, mfg) = if mode_good {
        (ACCENT_DIM, TEXT_CHROME)
    } else {
        (BAD, TEXT_CONTENT)
    };
    fb.fill_round_rect(mode_rect, mode_rect.h / 2, mbg);
    let mfs = 10.0 * m.scale;
    let mtw = text.measure_width(mode_label, mfs, 14.0 * m.scale, Weight::BOLD) as i32;
    label(
        fb,
        text,
        mode_label,
        mode_rect.x + (mode_rect.w - mtw) / 2,
        mode_rect.y + 1,
        mtw + 2,
        mfs,
        mfg,
        Weight::BOLD,
    );

    let url_x = x + 6;
    let url_w = (mode_rect.x - 8 - url_x).max(40);
    let url_h = m.url_h();
    let url_y = rect.y + (rect.h - url_h) / 2;
    let url_rect = Rect::new(url_x, url_y, url_w, url_h);
    let focused = browser.url_focused;
    let radius = m.s(8.0);
    fb.fill_round_rect(url_rect, radius, if focused { ACCENT } else { HAIRLINE });
    fb.fill_round_rect(
        url_rect.inset(1),
        radius - 1,
        if focused { BG_URL_FOCUS } else { BG_URL },
    );
    let url_pill = paint_net_pill(
        fb,
        text,
        m,
        tab.circuit,
        url_rect.x + 8,
        url_rect.y + url_rect.h / 2,
    );
    let url_rect = Rect::new(
        url_rect.x + url_pill + 6,
        url_rect.y,
        url_rect.w - url_pill - 6,
        url_rect.h,
    );

    let prefix = &browser.url_text[..browser.url_cursor.min(browser.url_text.len())];
    let caret_x = text.measure_width(prefix, m.font_ui(), m.line_ui(), Weight::NORMAL);

    text.draw(
        fb,
        &browser.url_text,
        DrawText {
            x: url_rect.x + 10,
            y: url_rect.y + (url_rect.h - m.line_ui() as i32) / 2,
            max_width: (url_rect.w - 20) as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: TEXT_CHROME,
            weight: Weight::NORMAL,
            ellipsis: true,
            wrap: false,
        },
    );
    if focused {
        let cx = url_rect.x + 10 + caret_x as i32;
        fb.fill_rect(Rect::new(cx, url_rect.y + 6, 1, url_rect.h - 12), ACCENT);
    }

    hits.push(HitRegion {
        rect: url_rect,
        hit: Hit::UrlBar,
    });
}

fn privacy_dot(browser: &Browser) -> (u32, &'static str) {
    let p = &browser.profile.prefs().privacy;
    if !p.https_only || p.third_party_cookies || !p.resist_fingerprinting {
        (WARN, "protection weakened")
    } else if p.javascript || p.webrtc {
        (WARN, "attack surface opened")
    } else {
        (GOOD, "defaults hold")
    }
}

fn paint_content(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    rect: Rect,
    hits: &mut Vec<HitRegion>,
) {
    fb.fill_rect(rect, BG_CONTENT);
    let tab = browser.active_tab();
    match &tab.document {
        Document::Blank => {
            text.draw(
                fb,
                "about:blank",
                DrawText {
                    x: rect.x + m.content_pad(),
                    y: rect.y + m.content_pad(),
                    max_width: rect.w as f32,
                    font_size: m.font_content(),
                    line_height: m.line_content(),
                    color: TEXT_CONTENT_MUTED,
                    weight: Weight::NORMAL,
                    ellipsis: false,
                    wrap: false,
                },
            );
        }
        Document::Source { text: body, .. } => {
            draw_wrapped(
                fb,
                text,
                body,
                (
                    rect.x + m.content_pad(),
                    rect.y + m.pad() - tab.scroll_y as i32,
                ),
                (rect.w - m.content_pad() * 2).max(40),
                m,
                TEXT_CONTENT,
            );
        }
        Document::Unavailable { url, reason } => {
            paint_internal_blocks(
                fb,
                text,
                browser,
                m,
                rect,
                &[
                    Block::Hero {
                        title: "Unavailable".into(),
                        subtitle: display_url(url),
                    },
                    Block::Paragraph(reason.clone()),
                ],
                hits,
            );
        }
        Document::Internal(page) => {
            paint_internal_blocks(fb, text, browser, m, rect, &page.blocks, hits);
        }
        Document::Page(page) => {
            paint_page(fb, text, browser, m, rect, page, hits);
        }
    }
}

fn paint_page(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    viewport: Rect,
    page: &frihart_content::Page,
    hits: &mut Vec<HitRegion>,
) {
    let pad = m.content_pad();
    let max_w = m.content_max_w().min(viewport.w - pad * 2);
    let x = viewport.x + ((viewport.w - max_w) / 2).max(pad);
    let mut y = viewport.y + pad - browser.active_tab().scroll_y as i32;
    let fill = Rect::new(x, y, max_w, m.s(28.0));
    text.draw(
        fb,
        &frihart_i18n::t("en", "fill-identity"),
        DrawText {
            x: fill.x,
            y,
            max_width: (max_w / 2) as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: TEXT_LINK,
            weight: Weight::MEDIUM,
            ellipsis: true,
            wrap: false,
        },
    );
    hits.push(HitRegion {
        rect: fill,
        hit: Hit::Autofill,
    });
    let pass = Rect::new(x + max_w / 2, y, max_w / 2, m.s(28.0));
    text.draw(
        fb,
        &frihart_i18n::t("en", "password-manager"),
        DrawText {
            x: pass.x,
            y,
            max_width: pass.w as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: TEXT_LINK,
            weight: Weight::MEDIUM,
            ellipsis: true,
            wrap: false,
        },
    );
    hits.push(HitRegion {
        rect: pass,
        hit: Hit::PassLaunch,
    });
    y += m.s(36.0);
    if let Some(frame) = &browser.active_tab().frame {
        y = paint_display_list(fb, text, frame, x, y, viewport, m, hits, browser, page);
    } else if !page.html.is_empty() {
        let extra = browser.profile.user_css();
        let frame = layout_html(&page.html, &extra, max_w as f32);
        y = paint_display_list(
            fb,
            text,
            &frame.display,
            x,
            y,
            viewport,
            m,
            hits,
            browser,
            page,
        );
    }
    if browser.active_tab().frame.is_some() || !page.html.is_empty() {
        return;
    }
    for (i, item) in page.items.iter().enumerate() {
        if y > viewport.bottom() {
            break;
        }
        match item {
            PageItem::Heading(level, t) => {
                let size = if *level <= 1 {
                    m.font_hero()
                } else {
                    m.font_heading()
                };
                let h = text.draw(
                    fb,
                    t,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: size,
                        line_height: size + 6.0,
                        color: TEXT_CONTENT,
                        weight: Weight::SEMIBOLD,
                        ellipsis: false,
                        wrap: false,
                    },
                );
                y += h.1 + m.s(10.0);
            }
            PageItem::Text(t) => {
                let h = draw_wrapped(fb, text, t, (x, y), max_w, m, TEXT_CONTENT);
                y += h + m.s(10.0);
            }
            PageItem::Link { text: label, href } => {
                let h = text.draw(
                    fb,
                    label,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_content(),
                        line_height: m.line_content(),
                        color: TEXT_LINK,
                        weight: Weight::MEDIUM,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                hits.push(HitRegion {
                    rect: Rect::new(x, y, max_w, h.1 + 4),
                    hit: Hit::ContentLink(href.clone()),
                });
                y += h.1 + m.s(8.0);
            }
            PageItem::Field {
                label,
                value,
                secret,
                ..
            } => {
                text.draw(
                    fb,
                    label,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_ui_sm(),
                        line_height: m.line_ui(),
                        color: TEXT_CONTENT_MUTED,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                y += m.s(18.0);
                let box_h = m.s(28.0);
                let boxr = Rect::new(x, y, max_w, box_h);
                fb.fill_rect(boxr, BG_URL);
                let focused = browser.field_focus == Some(i);
                fb.stroke_rect(boxr, if focused { ACCENT } else { HAIRLINE });
                let shown = if *secret {
                    if value.is_empty() {
                        ""
                    } else {
                        "••••••••"
                    }
                } else {
                    value.as_str()
                };
                text.draw(
                    fb,
                    shown,
                    DrawText {
                        x: boxr.x + 8,
                        y: boxr.y + 4,
                        max_width: (boxr.w - 16) as f32,
                        font_size: m.font_ui(),
                        line_height: m.line_ui(),
                        color: TEXT_CONTENT,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                hits.push(HitRegion {
                    rect: boxr,
                    hit: if *secret {
                        Hit::PassLaunch
                    } else {
                        Hit::Field(i)
                    },
                });
                y += box_h + m.s(12.0);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_display_list(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    list: &frihart_gfx::DisplayList,
    origin_x: i32,
    origin_y: i32,
    viewport: Rect,
    m: &Metrics,
    hits: &mut Vec<HitRegion>,
    browser: &Browser,
    page: &frihart_content::Page,
) -> i32 {
    let mut bottom = origin_y;
    for op in &list.ops {
        match op {
            DisplayOp::Image {
                x,
                y,
                width,
                height,
                rgba,
            } => {
                let ox = origin_x + *x as i32;
                let oy = origin_y + *y as i32;
                if op.image_is_well_formed() {
                    fb.blit_rgba(ox, oy, *width, *height, rgba, viewport);
                    bottom = bottom.max(oy + *height as i32);
                }
            }
            DisplayOp::Fill { x, y, w, h, color } => {
                let rect = Rect::new(
                    origin_x + *x as i32,
                    origin_y + *y as i32,
                    (*w as i32).max(1),
                    (*h as i32).max(1),
                );
                if rect.bottom() >= viewport.y && rect.y <= viewport.bottom() {
                    fb.fill_rect(rect, *color);
                }
                bottom = bottom.max(rect.bottom());
            }
            DisplayOp::Text {
                x,
                y,
                color,
                size,
                weight,
                family,
                text: body,
                href,
                max_width,
                wrap,
            } => {
                let px = origin_x + *x as i32;
                let py = origin_y + *y as i32;
                let drawn = text.draw_family(
                    fb,
                    body,
                    DrawText {
                        x: px,
                        y: py,
                        max_width: *max_width,
                        font_size: *size,
                        line_height: *size * 1.4,
                        color: *color,
                        weight: css_weight(*weight, href.is_some()),
                        ellipsis: false,
                        wrap: *wrap,
                    },
                    *family,
                );
                if let Some(href) = href {
                    hits.push(HitRegion {
                        rect: Rect::new(px, py, drawn.0.max(*max_width as i32), drawn.1.max(8)),
                        hit: Hit::ContentLink(href.clone()),
                    });
                }
                bottom = bottom.max(py + drawn.1);
            }
            DisplayOp::Field {
                x,
                y,
                w,
                h,
                index,
                secret,
                label,
            } => {
                let px = origin_x + *x as i32;
                let py = origin_y + *y as i32;
                text.draw(
                    fb,
                    label,
                    DrawText {
                        x: px,
                        y: py,
                        max_width: *w,
                        font_size: m.font_ui_sm(),
                        line_height: m.line_ui(),
                        color: TEXT_CONTENT_MUTED,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                let box_h = m.s(28.0);
                let boxr = Rect::new(px, py + m.s(18.0), (*w as i32).max(8), box_h);
                fb.fill_rect(boxr, BG_URL);
                fb.stroke_rect(
                    boxr,
                    if browser.field_focus == Some(*index) {
                        ACCENT
                    } else {
                        HAIRLINE
                    },
                );
                let value = match page.items.get(*index) {
                    Some(PageItem::Field { value, .. }) => value.as_str(),
                    _ => "",
                };
                let shown = if *secret {
                    if value.is_empty() {
                        ""
                    } else {
                        "••••••••"
                    }
                } else {
                    value
                };
                text.draw(
                    fb,
                    shown,
                    DrawText {
                        x: boxr.x + 8,
                        y: boxr.y + 4,
                        max_width: (boxr.w - 16) as f32,
                        font_size: m.font_ui(),
                        line_height: m.line_ui(),
                        color: TEXT_CONTENT,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                hits.push(HitRegion {
                    rect: boxr,
                    hit: if *secret {
                        Hit::PassLaunch
                    } else {
                        Hit::Field(*index)
                    },
                });
                let _ = h;
                bottom = bottom.max(boxr.bottom());
            }
        }
    }
    bottom + m.s(8.0)
}

fn paint_internal_blocks(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    viewport: Rect,
    blocks: &[Block],
    hits: &mut Vec<HitRegion>,
) {
    let here = browser.active_tab().url_display();
    if matches!(here.as_str(), "about:home" | "about:newtab") {
        paint_home(fb, text, browser, m, viewport, blocks, hits);
        return;
    }
    let pad = m.content_pad();
    let max_w = m.content_max_w().min(viewport.w - pad * 2);
    let mut locked_off = false;
    let x = viewport.x + ((viewport.w - max_w) / 2).max(pad);
    let mut y = viewport.y + pad - browser.active_tab().scroll_y as i32;

    for block in blocks {
        if y > viewport.bottom() {
            break;
        }
        if !matches!(block, Block::KeyValue { .. }) {
            locked_off = matches!(block, Block::Heading(h) if h == "Locked off");
        }
        match block {
            Block::Hero { title, subtitle } => {
                let h = text.draw(
                    fb,
                    title,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_hero(),
                        line_height: m.font_hero() + 8.0,
                        color: TEXT_CONTENT,
                        weight: Weight::SEMIBOLD,
                        ellipsis: false,
                        wrap: false,
                    },
                );
                y += h.1 + m.s(8.0);
                let h = text.draw(
                    fb,
                    subtitle,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_content(),
                        line_height: m.line_content(),
                        color: TEXT_CONTENT_MUTED,
                        weight: Weight::NORMAL,
                        ellipsis: false,
                        wrap: false,
                    },
                );
                y += h.1 + m.s(20.0);
            }
            Block::Heading(s) => {
                y += m.s(12.0);
                let h = text.draw(
                    fb,
                    s,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_heading(),
                        line_height: m.font_heading() + 6.0,
                        color: if locked_off { BAD } else { TEXT_CONTENT },
                        weight: Weight::SEMIBOLD,
                        ellipsis: false,
                        wrap: false,
                    },
                );
                y += h.1 + m.s(10.0);
            }
            Block::Paragraph(s) => {
                let h = draw_wrapped(fb, text, s, (x, y), max_w, m, TEXT_CONTENT);
                y += h + m.s(12.0);
            }
            Block::Note(s) => {
                let box_h = estimate_wrap_h(s, max_w, m) + m.s(20.0);
                let box_rect = Rect::new(x, y, max_w, box_h);
                fb.fill_rect(box_rect, BG_NOTE);
                draw_wrapped(
                    fb,
                    text,
                    s,
                    (x + m.s(14.0), y + m.s(10.0)),
                    max_w - m.s(28.0),
                    m,
                    TEXT_CONTENT_MUTED,
                );
                y += box_h + m.s(14.0);
            }
            Block::Divider => {
                fb.fill_rect(Rect::new(x, y + 6, max_w, 1), HAIRLINE);
                y += m.s(18.0);
            }
            Block::KeyValue { key, value } => {
                if locked_off {
                    // Red "forced off — not a setting" rows (DESIGN.md).
                    let rh = m.line_content() as i32 + m.s(6.0);
                    fb.fill_rect(
                        Rect::new(x - m.s(10.0), y - m.s(3.0), max_w + m.s(20.0), rh),
                        0x001A0C0C,
                    );
                    fb.fill_rect(Rect::new(x - m.s(10.0), y - m.s(3.0), m.s(3.0), rh), BAD);
                }
                text.draw(
                    fb,
                    key,
                    DrawText {
                        x,
                        y,
                        max_width: (max_w / 3) as f32,
                        font_size: m.font_ui(),
                        line_height: m.line_content(),
                        color: if locked_off { BAD } else { TEXT_CONTENT_MUTED },
                        weight: Weight::MEDIUM,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                let h = text.draw(
                    fb,
                    value,
                    DrawText {
                        x: x + max_w / 3,
                        y,
                        max_width: (max_w * 2 / 3) as f32,
                        font_size: m.font_ui(),
                        line_height: m.line_content(),
                        color: TEXT_CONTENT,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                y += h.1.max(m.line_content() as i32) + m.s(6.0);
            }
            Block::Toggle {
                id,
                label,
                description,
                value,
            } => {
                let row_h = m.s(64.0);
                let row = Rect::new(x, y, max_w, row_h);
                fb.fill_rect(Rect::new(x, y + row_h - 1, max_w, 1), HAIRLINE);
                text.draw(
                    fb,
                    label,
                    DrawText {
                        x,
                        y: y + m.s(8.0),
                        max_width: (max_w - m.s(70.0)) as f32,
                        font_size: m.font_content(),
                        line_height: m.line_content(),
                        color: TEXT_CONTENT,
                        weight: Weight::MEDIUM,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                let desc = format!("{}  ·  {description}", id.pref_key());
                text.draw(
                    fb,
                    &desc,
                    DrawText {
                        x,
                        y: y + m.s(30.0),
                        max_width: (max_w - m.s(70.0)) as f32,
                        font_size: m.font_ui_sm(),
                        line_height: m.line_ui(),
                        color: TEXT_CONTENT_MUTED,
                        weight: Weight::NORMAL,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                let sw = Rect::new(x + max_w - m.s(52.0), y + m.s(18.0), m.s(44.0), m.s(24.0));
                fb.fill_round_rect(
                    sw,
                    sw.h / 2,
                    if *value { BG_TOGGLE_ON } else { BG_TOGGLE_OFF },
                );
                let knob_x = if *value {
                    sw.right() - m.s(20.0)
                } else {
                    sw.x + m.s(4.0)
                };
                fb.fill_round_rect(
                    Rect::new(knob_x, sw.y + m.s(4.0), m.s(16.0), m.s(16.0)),
                    m.s(8.0),
                    if *value {
                        BG_CONTENT
                    } else {
                        TEXT_CONTENT_MUTED
                    },
                );
                hits.push(HitRegion {
                    rect: row,
                    hit: Hit::ContentToggle(*id),
                });
                y += row_h;
            }
            Block::Link { label, href } => {
                let h = text.draw(
                    fb,
                    label,
                    DrawText {
                        x,
                        y,
                        max_width: max_w as f32,
                        font_size: m.font_content(),
                        line_height: m.line_content(),
                        color: TEXT_LINK,
                        weight: Weight::MEDIUM,
                        ellipsis: true,
                        wrap: false,
                    },
                );
                fb.fill_rect(Rect::new(x, y + h.1 - 2, h.0.min(max_w), 1), ACCENT_DIM);
                hits.push(HitRegion {
                    rect: Rect::new(x, y, max_w, h.1 + 4),
                    hit: Hit::ContentLink(href.clone()),
                });
                y += h.1 + m.s(8.0);
            }
            Block::List(items) => {
                for item in items {
                    let bullet = format!("·  {item}");
                    let h = draw_wrapped(fb, text, &bullet, (x, y), max_w, m, TEXT_CONTENT);
                    y += h + m.s(6.0);
                }
                y += m.s(8.0);
            }
        }
    }

    let _ = y;
}

fn draw_wrapped(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    body: &str,
    origin: (i32, i32),
    max_w: i32,
    m: &Metrics,
    color: u32,
) -> i32 {
    let (x, y) = origin;
    let lines = wrap_words(body, max_w, m, text);
    let mut yy = y;
    for line in lines {
        let h = text.draw(
            fb,
            &line,
            DrawText {
                x,
                y: yy,
                max_width: max_w as f32,
                font_size: m.font_content(),
                line_height: m.line_content(),
                color,
                weight: Weight::NORMAL,
                ellipsis: false,
                wrap: false,
            },
        );
        yy += h.1.max(m.line_content() as i32);
    }
    yy - y
}

fn estimate_wrap_h(body: &str, max_w: i32, m: &Metrics) -> i32 {
    let avg = (m.font_content() * 0.5).max(1.0);
    let chars_per_line = ((max_w as f32) / avg).max(8.0) as usize;
    let lines = body.len().div_ceil(chars_per_line).max(1);
    lines as i32 * m.line_content() as i32
}

fn wrap_words(body: &str, max_w: i32, m: &Metrics, text: &mut TextEngine) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in body.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        let w = text.measure_width(
            &candidate,
            m.font_content(),
            m.line_content(),
            Weight::NORMAL,
        );
        if w > max_w as f32 && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn paint_status(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    rect: Rect,
) {
    fb.fill_rect(rect, BG_STATUS);
    fb.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1), HAIRLINE);
    let mut label = browser.status.clone();
    if browser.is_private() {
        label = format!("Private.  {label}");
    }
    if let Some(c) = browser.profile.container(browser.active_tab().container) {
        label = format!("{}  ·  {label}", c.name);
    }
    match browser.active_tab().circuit {
        Circuit::Tor => label = format!("Tor · fail-closed · DNS inside Tor  ·  {label}"),
        Circuit::I2p => label = format!("I2P · fail-closed · no clearnet  ·  {label}"),
        Circuit::Private => label = format!("{}  ·  {label}", Circuit::Private.label()),
        Circuit::Direct => {}
    }
    text.draw(
        fb,
        &label,
        DrawText {
            x: rect.x + m.pad(),
            y: rect.y + 3,
            max_width: (rect.w / 2) as f32,
            font_size: m.font_ui_sm(),
            line_height: m.line_ui(),
            color: TEXT_MUTED,
            weight: Weight::NORMAL,
            ellipsis: true,
            wrap: false,
        },
    );
    let url = browser.active_tab().url_display();
    text.draw(
        fb,
        &url,
        DrawText {
            x: rect.x + rect.w / 2,
            y: rect.y + 3,
            max_width: (rect.w / 2 - m.pad()) as f32,
            font_size: m.font_ui_sm(),
            line_height: m.line_ui(),
            color: TEXT_MUTED,
            weight: Weight::NORMAL,
            ellipsis: true,
            wrap: false,
        },
    );
}

/// Network pill text and colour (DESIGN.md: DIRECT / TOR / I2P on every tab and the URL field).
pub(crate) fn net_pill(circuit: Circuit) -> (&'static str, u32, u32) {
    match circuit {
        Circuit::Tor => ("TOR", NET_TOR, BG_CHROME),
        Circuit::I2p => ("I2P", NET_I2P, BG_CHROME),
        Circuit::Private => ("PRIVATE", ACCENT, BG_CHROME),
        Circuit::Direct => ("DIRECT", NET_DIRECT, TEXT_CONTENT_MUTED),
    }
}

/// Draw a pill vertically centred on `cy`; returns its width.
fn paint_net_pill(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    m: &Metrics,
    circuit: Circuit,
    x: i32,
    cy: i32,
) -> i32 {
    let (label, bg, fg) = net_pill(circuit);
    let fs = 10.0 * m.scale;
    let lh = 14.0 * m.scale;
    let tw = text.measure_width(label, fs, lh, Weight::BOLD) as i32;
    let h = m.s(16.0);
    let w = tw + m.s(12.0);
    fb.fill_round_rect(Rect::new(x, cy - h / 2, w, h), h / 2, bg);
    text.draw(
        fb,
        label,
        DrawText {
            x: x + m.s(6.0),
            y: cy - lh as i32 / 2,
            max_width: (tw + 4) as f32,
            font_size: fs,
            line_height: lh,
            color: fg,
            weight: Weight::BOLD,
            ellipsis: false,
            wrap: false,
        },
    );
    w
}

fn container_glyph(browser: &Browser) -> &'static str {
    match browser.active_tab().container.0 {
        1 => "P",
        2 => "W",
        3 => "B",
        4 => "S",
        _ => "C",
    }
}

fn paint_find(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    rect: Rect,
    hits: &mut Vec<HitRegion>,
) {
    fb.fill_rect(rect, BG_FIND);
    fb.fill_rect(Rect::new(rect.x, rect.y, rect.w, 1), ACCENT_DIM);
    text.draw(
        fb,
        "Find",
        DrawText {
            x: rect.x + m.pad(),
            y: rect.y + 6,
            max_width: m.s(48.0) as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: TEXT_CHROME,
            weight: Weight::MEDIUM,
            ellipsis: false,
            wrap: false,
        },
    );
    let field = Rect::new(
        rect.x + m.s(64.0),
        rect.y + 4,
        (rect.w / 2).max(80),
        rect.h - 8,
    );
    fb.fill_rect(field, BG_URL);
    fb.stroke_rect(
        field,
        if browser.find_focused {
            ACCENT
        } else {
            HAIRLINE
        },
    );
    let shown = if browser.find_text.is_empty() {
        "type to search this page"
    } else {
        browser.find_text.as_str()
    };
    text.draw(
        fb,
        shown,
        DrawText {
            x: field.x + 8,
            y: field.y + 2,
            max_width: (field.w - 16) as f32,
            font_size: m.font_ui(),
            line_height: m.line_ui(),
            color: if browser.find_text.is_empty() {
                TEXT_MUTED
            } else {
                TEXT_CONTENT
            },
            weight: Weight::NORMAL,
            ellipsis: true,
            wrap: false,
        },
    );
    text.draw(
        fb,
        &browser.find_status,
        DrawText {
            x: field.right() + 12,
            y: rect.y + 6,
            max_width: (rect.right() - field.right() - 20) as f32,
            font_size: m.font_ui_sm(),
            line_height: m.line_ui(),
            color: TEXT_MUTED,
            weight: Weight::NORMAL,
            ellipsis: true,
            wrap: false,
        },
    );
    hits.push(HitRegion {
        rect: field,
        hit: Hit::FindBar,
    });
}

pub fn hit_test(hits: &[HitRegion], x: i32, y: i32) -> Option<Hit> {
    hits.iter()
        .rev()
        .find(|h| h.rect.contains(x, y))
        .map(|h| h.hit.clone())
}

/// about:home / about:newtab — centered wordmark, search box, tiles (DESIGN.md concept).
fn paint_home(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    vp: Rect,
    blocks: &[Block],
    hits: &mut Vec<HitRegion>,
) {
    let cx = vp.x + vp.w / 2;
    let mut y = vp.y + m.s(70.0).min(vp.h / 6) - browser.active_tab().scroll_y as i32;
    let (title, tagline) = match blocks.first() {
        Some(Block::Hero { title, subtitle }) => (title.as_str(), subtitle.as_str()),
        _ => ("Frihart", ""),
    };
    // Wordmark: first half white, second half accent ("Fri" + "hart").
    let fs = 56.0 * m.scale;
    let split = title.char_indices().nth(3).map_or(title.len(), |(i, _)| i);
    let (a, b) = title.split_at(split);
    let wa = text.measure_width(a, fs, fs + 8.0, Weight::BOLD) as i32;
    let wb = text.measure_width(b, fs, fs + 8.0, Weight::BOLD) as i32;
    let x0 = cx - (wa + wb) / 2;
    label(fb, text, a, x0, y, wa + 8, fs, TEXT_CONTENT, Weight::BOLD);
    label(fb, text, b, x0 + wa, y, wb + 8, fs, ACCENT, Weight::BOLD);
    y += m.s(76.0);
    let tw = text.measure_width(tagline, m.font_ui() + 1.0, m.line_ui(), Weight::NORMAL) as i32;
    label(
        fb,
        text,
        tagline,
        cx - tw / 2,
        y,
        tw + 8,
        m.font_ui() + 1.0,
        ACCENT,
        Weight::NORMAL,
    );
    y += m.s(36.0);

    // Search box: focuses the URL bar (no suggestions are sent unless enabled).
    let sw = m.s(440.0).min(vp.w - m.s(40.0));
    let sr = Rect::new(cx - sw / 2, y, sw, m.s(36.0));
    fb.fill_round_rect(sr, m.s(10.0), HAIRLINE);
    fb.fill_round_rect(sr.inset(1), m.s(9.0), BG_CHROME);
    label(
        fb,
        text,
        "Search privately or enter address",
        sr.x + m.s(14.0),
        sr.y + m.s(9.0),
        sw - m.s(160.0),
        m.font_ui(),
        TEXT_CONTENT_MUTED,
        Weight::NORMAL,
    );
    let hint = if browser.profile.prefs().privacy.search_suggestions {
        "suggestions on"
    } else {
        "no suggestions sent"
    };
    let hw = text.measure_width(hint, 10.5 * m.scale, 16.0, Weight::NORMAL) as i32;
    label(
        fb,
        text,
        hint,
        sr.right() - hw - m.s(14.0),
        sr.y + m.s(11.0),
        hw + 6,
        10.5 * m.scale,
        TEXT_MUTED,
        Weight::NORMAL,
    );
    hits.push(HitRegion {
        rect: sr,
        hit: Hit::UrlBar,
    });
    y += m.s(60.0);

    // Featured tiles (real about: pages).
    let tiles = [
        ("◎", "Tor", "about:tor", "fail-closed"),
        ("◇", "I2P", "about:i2p", "no clearnet"),
        ("▣", "Containers", "about:containers", "isolated"),
        ("⌫", "Shred site data", "about:shred", "wipe this profile"),
    ];
    let gap = m.s(12.0);
    let tile_w = ((sw + m.s(260.0)).min(vp.w - m.s(40.0)) - gap * 3) / 4;
    let row_w = tile_w * 4 + gap * 3;
    let mut tx = cx - row_w / 2;
    let tile_h = m.s(76.0);
    for (glyph, name, href, sub) in tiles {
        let r = Rect::new(tx, y, tile_w, tile_h);
        let hovered = matches!(&browser.hover, Some(Hit::ContentLink(h)) if h == href);
        fb.fill_round_rect(r, m.s(12.0), if hovered { ACCENT_DIM } else { HAIRLINE });
        fb.fill_round_rect(r.inset(1), m.s(11.0), BG_TAB_ACTIVE);
        label(
            fb,
            text,
            glyph,
            r.x + m.s(14.0),
            r.y + m.s(10.0),
            m.s(24.0),
            m.font_ui() + 2.0,
            ACCENT,
            Weight::BOLD,
        );
        label(
            fb,
            text,
            name,
            r.x + m.s(14.0),
            r.y + m.s(32.0),
            tile_w - m.s(20.0),
            m.font_ui(),
            TEXT_CONTENT,
            Weight::BOLD,
        );
        label(
            fb,
            text,
            &format!("{href} · {sub}"),
            r.x + m.s(14.0),
            r.y + m.s(52.0),
            tile_w - m.s(20.0),
            10.0 * m.scale,
            TEXT_MUTED,
            Weight::NORMAL,
        );
        hits.push(HitRegion {
            rect: r,
            hit: Hit::ContentLink(href.into()),
        });
        tx += tile_w + gap;
    }
    y += tile_h + m.s(28.0);

    // All other pages as compact chips.
    let links: Vec<(&str, &str)> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::Link { label, href } => Some((label.as_str(), href.as_str())),
            _ => None,
        })
        .filter(|(_, h)| !tiles.iter().any(|t| t.2 == *h) && !h.ends_with("home"))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let links: Vec<(&str, &str)> = links.into_iter().filter(|(_, h)| seen.insert(*h)).collect();
    let lx0 = cx - row_w / 2;
    label(
        fb,
        text,
        "All pages",
        lx0,
        y,
        row_w,
        m.font_ui_sm(),
        TEXT_MUTED,
        Weight::BOLD,
    );
    y += m.s(22.0);
    let chip_h = m.s(26.0);
    let mut lx = lx0;
    for (name, href) in links {
        let cw = text.measure_width(name, m.font_ui_sm(), m.line_ui(), Weight::MEDIUM) as i32
            + m.s(20.0);
        if lx + cw > lx0 + row_w {
            lx = lx0;
            y += chip_h + m.s(8.0);
        }
        if y > vp.bottom() {
            break;
        }
        let r = Rect::new(lx, y, cw, chip_h);
        let hovered = matches!(&browser.hover, Some(Hit::ContentLink(h)) if h == href);
        fb.fill_round_rect(r, chip_h / 2, if hovered { ACCENT_DIM } else { HAIRLINE });
        fb.fill_round_rect(r.inset(1), chip_h / 2 - 1, BG_CONTENT);
        label(
            fb,
            text,
            name,
            r.x + m.s(10.0),
            r.y + m.s(4.0),
            cw,
            m.font_ui_sm(),
            TEXT_LINK,
            Weight::MEDIUM,
        );
        hits.push(HitRegion {
            rect: r,
            hit: Hit::ContentLink(href.into()),
        });
        lx += cw + m.s(8.0);
    }
    y += chip_h + m.s(26.0);
    if let Some(Block::Paragraph(p)) = blocks.get(1) {
        let pw = text.measure_width(p, 10.5 * m.scale, 16.0, Weight::NORMAL) as i32;
        label(
            fb,
            text,
            p,
            cx - pw.min(row_w) / 2,
            y,
            row_w,
            10.5 * m.scale,
            TEXT_MUTED,
            Weight::NORMAL,
        );
        y += m.s(20.0);
    }
    let foot = "no sponsored tiles · no telemetry · no account";
    let fw = text.measure_width(foot, 10.5 * m.scale, 16.0, Weight::NORMAL) as i32;
    label(
        fb,
        text,
        foot,
        cx - fw / 2,
        y,
        fw + 6,
        10.5 * m.scale,
        TEXT_MUTED,
        Weight::NORMAL,
    );
}

/// Small single-line text helper for chrome overlays (presentation only).
#[allow(clippy::too_many_arguments)]
fn label(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    s: &str,
    x: i32,
    y: i32,
    max_w: i32,
    size: f32,
    color: u32,
    weight: Weight,
) -> (i32, i32) {
    text.draw(
        fb,
        s,
        DrawText {
            x,
            y,
            max_width: max_w.max(2) as f32,
            font_size: size,
            line_height: size + 6.0,
            color,
            weight,
            ellipsis: true,
            wrap: false,
        },
    )
}

/// Dashed outline (letterbox frame).
fn dashed_rect(fb: &mut Framebuffer, r: Rect, dash: i32, color: u32) {
    let mut x = r.x;
    while x < r.right() {
        let w = dash.min(r.right() - x);
        fb.fill_rect(Rect::new(x, r.y, w, 1), color);
        fb.fill_rect(Rect::new(x, r.bottom() - 1, w, 1), color);
        x += dash * 2;
    }
    let mut y = r.y;
    while y < r.bottom() {
        let h = dash.min(r.bottom() - y);
        fb.fill_rect(Rect::new(r.x, y, 1, h), color);
        fb.fill_rect(Rect::new(r.right() - 1, y, 1, h), color);
        y += dash * 2;
    }
}

/// Letterbox frame: dashed outline + the bucket size the page sees.
fn paint_letterbox_outline(fb: &mut Framebuffer, text: &mut TextEngine, m: &Metrics, inner: Rect) {
    let o = Rect::new(inner.x - 1, inner.y - 1, inner.w + 2, inner.h + 2);
    dashed_rect(fb, o, m.s(6.0), ACCENT_DIM);
    let tag = format!("letterbox {}×{}", inner.w, inner.h);
    label(
        fb,
        text,
        &tag,
        inner.right() - m.s(110.0),
        inner.y - m.s(16.0),
        m.s(150.0),
        10.0 * m.scale,
        ACCENT_DIM,
        Weight::NORMAL,
    );
}

/// Fail-closed card on Tor / I2P tabs (DESIGN.md). Display only.
fn paint_fail_closed_card(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    m: &Metrics,
    circuit: Circuit,
    content: Rect,
) {
    let (net, col, dns) = match circuit {
        Circuit::Tor => ("Tor", NET_TOR, "DNS inside Tor."),
        Circuit::I2p => ("I2P", NET_I2P, "Names resolve inside I2P."),
        _ => return,
    };
    let w = m.s(420.0).min(content.w - m.s(24.0));
    let h = m.s(58.0);
    let r = Rect::new(
        content.x + m.s(12.0),
        content.bottom() - h - m.s(12.0),
        w,
        h,
    );
    if w < m.s(120.0) || r.y < content.y {
        return;
    }
    fb.fill_round_rect(r, m.s(12.0), col);
    fb.fill_round_rect(r.inset(1), m.s(11.0), 0x00101014);
    fb.fill_round_rect(
        Rect::new(r.x + m.s(12.0), r.y + m.s(14.0), m.s(8.0), m.s(8.0)),
        m.s(4.0),
        col,
    );
    label(
        fb,
        text,
        &format!("Fail-closed · {net}"),
        r.x + m.s(28.0),
        r.y + m.s(8.0),
        w - m.s(40.0),
        m.font_ui(),
        col,
        Weight::BOLD,
    );
    label(
        fb,
        text,
        &format!("If {net} drops, this tab stops. No clearnet fallback. {dns}"),
        r.x + m.s(12.0),
        r.y + m.s(32.0),
        w - m.s(24.0),
        m.font_ui_sm(),
        TEXT_CONTENT_MUTED,
        Weight::NORMAL,
    );
}

/// Shields side panel, opened by the privacy dot. Reads prefs only.
fn paint_shields_panel(
    fb: &mut Framebuffer,
    text: &mut TextEngine,
    browser: &Browser,
    m: &Metrics,
    area: Rect,
    letterbox: Option<(i32, i32)>,
    hits: &mut Vec<HitRegion>,
) {
    let w = m.s(330.0).min(area.w);
    let r = Rect::new(area.right() - w, area.y, w, area.h);
    fb.fill_rect(r, BG_TAB_ACTIVE);
    fb.fill_rect(Rect::new(r.x, r.y, 1, r.h), HAIRLINE);
    hits.push(HitRegion {
        rect: r,
        hit: Hit::ShieldsPanel,
    });
    let p = &browser.profile.prefs().privacy;
    let circuit = browser.active_tab().circuit;
    let pad = m.s(18.0);
    let x = r.x + pad;
    let iw = w - pad * 2;
    let mut y = r.y + pad;
    let (dot, verdict) = privacy_dot(browser);
    label(
        fb,
        text,
        "Shields",
        x,
        y,
        iw,
        m.font_heading() - 4.0,
        TEXT_CONTENT,
        Weight::BOLD,
    );
    let close = Rect::new(r.right() - pad - m.s(16.0), y, m.s(20.0), m.s(22.0));
    label(
        fb,
        text,
        "×",
        close.x,
        close.y,
        close.w,
        m.font_ui() + 3.0,
        TEXT_MUTED,
        Weight::NORMAL,
    );
    hits.push(HitRegion {
        rect: close,
        hit: Hit::PrivacyBadge,
    });
    y += m.s(30.0);
    fb.fill_round_rect(
        Rect::new(x, y + m.s(5.0), m.s(8.0), m.s(8.0)),
        m.s(4.0),
        dot,
    );
    label(
        fb,
        text,
        verdict,
        x + m.s(14.0),
        y,
        iw,
        m.font_ui(),
        dot,
        Weight::MEDIUM,
    );
    y += m.s(30.0);

    let uniform = matches!(circuit, Circuit::Tor | Circuit::I2p);
    let lb = match (p.letterboxing, letterbox) {
        (true, Some((lw, lh))) => format!("on · {lw}×{lh}"),
        (true, None) => "on".to_string(),
        (false, _) => "off".to_string(),
    };
    let rows: [(&str, &str, String, bool); 7] = [
        (
            "Native blocker",
            "privacy.blocker",
            on_off(p.blocker),
            p.blocker,
        ),
        (
            "Uniform identity headers",
            "Tor / I2P tabs",
            if uniform {
                "on".into()
            } else {
                "DIRECT tab".into()
            },
            uniform,
        ),
        ("Letterboxing", "privacy.letterboxing", lb, p.letterboxing),
        (
            "JavaScript",
            "privacy.javascript",
            if p.javascript {
                "on (no engine)".into()
            } else {
                "off · no engine".into()
            },
            !p.javascript,
        ),
        (
            "Third-party cookies",
            "privacy.third_party_cookies",
            if p.third_party_cookies {
                "allowed".into()
            } else {
                "blocked".into()
            },
            !p.third_party_cookies,
        ),
        (
            "Strip tracking params",
            "privacy.strip_tracking",
            on_off(p.strip_tracking),
            p.strip_tracking,
        ),
        (
            "HTTPS-only",
            "privacy.https_only",
            on_off(p.https_only),
            p.https_only,
        ),
    ];
    for (name, key, state, good) in rows {
        let row_h = m.s(46.0);
        label(
            fb,
            text,
            name,
            x,
            y + m.s(4.0),
            iw - m.s(110.0),
            m.font_ui(),
            TEXT_CONTENT,
            Weight::MEDIUM,
        );
        label(
            fb,
            text,
            key,
            x,
            y + m.s(23.0),
            iw - m.s(110.0),
            10.5 * m.scale,
            TEXT_MUTED,
            Weight::NORMAL,
        );
        let fs = 10.5 * m.scale;
        let tw = text.measure_width(&state, fs, 14.0 * m.scale, Weight::BOLD) as i32;
        let pw = tw + m.s(14.0);
        let pr = Rect::new(r.right() - pad - pw, y + m.s(10.0), pw, m.s(18.0));
        let (bg, fg) = if good {
            (ACCENT, BG_CHROME)
        } else {
            (BAD, TEXT_CONTENT)
        };
        fb.fill_round_rect(pr, pr.h / 2, bg);
        label(
            fb,
            text,
            &state,
            pr.x + m.s(7.0),
            pr.y + 1,
            tw + 4,
            fs,
            fg,
            Weight::BOLD,
        );
        fb.fill_rect(Rect::new(x, y + row_h - 1, iw, 1), HAIRLINE);
        y += row_h;
    }

    // Campaign S M4 — per-site JS override (interactive).
    {
        let host = browser.active_host();
        let torish = matches!(circuit, Circuit::Tor | Circuit::I2p);
        let mode = browser.active_tab().mode;
        let (state, good, clickable) = if torish {
            ("off · unavailable".to_string(), true, false)
        } else if host.is_empty() {
            ("no host".to_string(), true, false)
        } else {
            let on = browser.site_js_allowed();
            let label = if on {
                match mode {
                    frihart_engine::TabMode::Compat if frihart_engine_servo::AVAILABLE => {
                        "on · Compat".into()
                    }
                    frihart_engine::TabMode::Compat => "on · needs servo".into(),
                    frihart_engine::TabMode::Reader => "on · needs Compat".into(),
                }
            } else {
                "off".into()
            };
            (label, !on, true)
        };
        let row_h = m.s(46.0);
        let row_rect = Rect::new(x, y, iw, row_h);
        label(
            fb,
            text,
            "JavaScript (this site)",
            x,
            y + m.s(4.0),
            iw - m.s(110.0),
            m.font_ui(),
            TEXT_CONTENT,
            Weight::MEDIUM,
        );
        let sub = if host.is_empty() {
            "first-party host".to_string()
        } else {
            host.clone()
        };
        label(
            fb,
            text,
            &sub,
            x,
            y + m.s(23.0),
            iw - m.s(110.0),
            10.5 * m.scale,
            TEXT_MUTED,
            Weight::NORMAL,
        );
        let fs = 10.5 * m.scale;
        let tw = text.measure_width(&state, fs, 14.0 * m.scale, Weight::BOLD) as i32;
        let pw = tw + m.s(14.0);
        let pr = Rect::new(r.right() - pad - pw, y + m.s(10.0), pw, m.s(18.0));
        let (bg, fg) = if good {
            (ACCENT, BG_CHROME)
        } else {
            (BAD, TEXT_CONTENT)
        };
        fb.fill_round_rect(pr, pr.h / 2, bg);
        label(
            fb,
            text,
            &state,
            pr.x + m.s(7.0),
            pr.y + 1,
            tw + 4,
            fs,
            fg,
            Weight::BOLD,
        );
        fb.fill_rect(Rect::new(x, y + row_h - 1, iw, 1), HAIRLINE);
        if clickable {
            hits.push(HitRegion {
                rect: row_rect,
                hit: Hit::ShieldsSiteJs,
            });
        }
        y += row_h;
    }

    y += m.s(14.0);
    for (txt, href) in [
        ("Blocker details", "about:blocker"),
        ("All privacy defaults", "about:privacy"),
    ] {
        let h = label(
            fb,
            text,
            &format!("{txt}  ›"),
            x,
            y,
            iw,
            m.font_ui(),
            TEXT_LINK,
            Weight::MEDIUM,
        );
        hits.push(HitRegion {
            rect: Rect::new(x, y, iw, h.1),
            hit: Hit::ContentLink(href.into()),
        });
        y += h.1 + m.s(8.0);
    }
    let _ = y;
    label(
        fb,
        text,
        "no telemetry · no account",
        x,
        r.bottom() - pad - m.s(14.0),
        iw,
        m.font_ui_sm(),
        TEXT_MUTED,
        Weight::NORMAL,
    );
}

fn on_off(b: bool) -> String {
    if b { "on".into() } else { "off".into() }
}

#[cfg(test)]
mod design_tests {
    use super::*;
    use crate::state::Browser;
    use frihart_profile::Profile;

    fn browser() -> Browser {
        let profile = Profile::ephemeral().expect("ephemeral profile");
        let mut b = Browser::new(profile, Some("about:home".into()), false, false);
        b.active_tab_mut().circuit = Circuit::Direct;
        b.new_tor_tab();
        b.new_i2p_tab();
        b.active = 1;
        b
    }

    /// Writes a PPM when FRIHART_SNAPSHOT_DIR is set (docs/screenshots/real).
    fn dump(fb: &Framebuffer, name: &str) {
        let Ok(dir) = std::env::var("FRIHART_SNAPSHOT_DIR") else {
            return;
        };
        let mut out = format!("P6\n{} {}\n255\n", fb.width, fb.height).into_bytes();
        for p in &fb.pixels {
            out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(format!("{dir}/{name}.ppm"), out).expect("write snapshot");
    }

    fn render(b: &Browser, name: &str) -> Framebuffer {
        let mut fb = Framebuffer::new(1280, 800);
        let mut text = TextEngine::new();
        let hits = paint(&mut fb, &mut text, b, 1.0);
        assert!(!hits.is_empty());
        dump(&fb, name);
        fb
    }

    #[test]
    fn net_pills_match_design_tokens() {
        assert_eq!(net_pill(Circuit::Tor).1, NET_TOR);
        assert_eq!(net_pill(Circuit::I2p).1, NET_I2P);
        assert_eq!(net_pill(Circuit::Direct).0, "DIRECT");
    }

    #[test]
    fn tor_tab_paints_tor_pill() {
        let b = browser();
        let fb = render(&b, "frihart-tor-tab");
        assert!(fb.pixels.contains(&NET_TOR));
        assert!(fb.pixels.contains(&NET_I2P));
    }

    #[test]
    fn new_tab_page_renders() {
        let mut b = browser();
        b.active = 0;
        render(&b, "frihart-new-tab");
    }
    #[test]
    fn shields_panel_opens_from_privacy_dot() {
        let mut b = browser();
        b.shields_open = true;
        let mut fb = Framebuffer::new(1280, 800);
        let mut text = TextEngine::new();
        let hits = paint(&mut fb, &mut text, &b, 1.0);
        dump(&fb, "frihart-shields");
        assert!(hits.iter().any(|h| matches!(h.hit, Hit::ShieldsPanel)));
        assert!(
            hits.iter()
                .any(|h| matches!(&h.hit, Hit::ContentLink(u) if u == "about:blocker"))
        );
        // Tor tab: site JS row is locked (no ShieldsSiteJs hit).
        assert!(
            !hits.iter().any(|h| matches!(h.hit, Hit::ShieldsSiteJs)),
            "Tor must not expose a site-JS toggle"
        );
    }

    #[test]
    fn shields_site_js_hit_on_clearnet_and_toggles() {
        let profile = Profile::ephemeral().expect("ephemeral");
        let mut b = Browser::new(profile, Some("https://example.com/".into()), false, false);
        b.shields_open = true;
        let mut fb = Framebuffer::new(1280, 800);
        let mut text = TextEngine::new();
        let hits = paint(&mut fb, &mut text, &b, 1.0);
        assert!(
            hits.iter().any(|h| matches!(h.hit, Hit::ShieldsSiteJs)),
            "clearnet site must expose per-site JS toggle"
        );
        assert!(!b.site_js_allowed());
        b.toggle_site_js();
        assert!(b.site_js_allowed());
        assert_eq!(b.profile.site_js().override_for("example.com"), Some(true));
        b.toggle_site_js();
        assert!(!b.site_js_allowed());
    }

    #[test]
    fn tor_tab_has_fail_closed_card_and_letterbox_outline() {
        let b = browser();
        let fb = render(&b, "frihart-tor-tab");
        assert!(fb.pixels.contains(&0x00101014), "fail-closed card");
        assert!(fb.pixels.contains(&ACCENT_DIM), "letterbox outline");
    }

    #[test]
    fn settings_page_renders_pref_keys_and_forced_off() {
        let profile = Profile::ephemeral().expect("ephemeral profile");
        let b = Browser::new(profile, Some("about:settings".into()), false, false);
        let fb = render(&b, "frihart-settings");
        assert!(fb.pixels.contains(&BG_TOGGLE_ON));
        assert_eq!(
            frihart_content::PrefToggle::HttpsOnly.pref_key(),
            "privacy.https_only"
        );
        let mut b = b;
        b.active_tab_mut().scroll_y = 2400.0;
        let fb = render(&b, "frihart-settings-forced-off");
        assert!(fb.pixels.contains(&BAD), "forced-off rows are red");
    }
}
