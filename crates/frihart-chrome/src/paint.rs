//! Paint chrome + internal pages, and collect hit regions.

use cosmic_text::Weight;

use frihart_content::{Block, Document, PageItem};
use frihart_core::display_url;
use frihart_gfx::DisplayOp;
use frihart_pipeline::layout_html;

use crate::raster::{Framebuffer, Rect};
use crate::state::{Browser, Hit};
use crate::text::{DrawText, TextEngine};
use crate::theme::{
    ACCENT, ACCENT_DIM, BAD, BG_CHROME, BG_CONTENT, BG_FIND, BG_NOTE, BG_STATUS, BG_TAB_ACTIVE,
    BG_TAB_HOVER, BG_TOGGLE_OFF, BG_TOGGLE_ON, BG_TOOLBAR, BG_URL, BG_URL_FOCUS, GOOD, HAIRLINE,
    Metrics, PRIVATE, TEXT_CHROME, TEXT_CONTENT, TEXT_CONTENT_MUTED, TEXT_LINK, TEXT_MUTED, WARN,
};

pub struct HitRegion {
    pub rect: Rect,
    pub hit: Hit,
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

    paint_tabs(fb, text, browser, &m, w, tab_h, &mut hits);
    paint_toolbar(
        fb,
        text,
        browser,
        &m,
        Rect::new(0, tab_h, w, toolbar_h),
        &mut hits,
    );

    let content = Rect::new(
        0,
        tab_h + toolbar_h,
        w,
        (h - tab_h - toolbar_h - status_h - find_h).max(0),
    );
    paint_content(fb, text, browser, &m, content, &mut hits);

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
        fb.fill_rect(rect, bg);
        if active {
            fb.fill_rect(Rect::new(rect.x, rect.bottom() - 2, rect.w, 2), ACCENT);
        }
        let stripe = browser
            .profile
            .container(tab.container)
            .map(|c| c.color)
            .unwrap_or(ACCENT);
        fb.fill_rect(Rect::new(rect.x, rect.y, m.s(4.0), rect.h), stripe);

        let close_w = m.s(18.0);
        let label_w = (rect.w - close_w - 16).max(8) as f32;
        text.draw(
            fb,
            &tab.title(),
            DrawText {
                x: rect.x + 10,
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
    fb.fill_rect(badge.inset(8), dot);
    hits.push(HitRegion {
        rect: badge,
        hit: Hit::PrivacyBadge,
    });

    let url_x = x + 6;
    let url_w = (badge.x - 8 - url_x).max(40);
    let url_h = m.url_h();
    let url_y = rect.y + (rect.h - url_h) / 2;
    let url_rect = Rect::new(url_x, url_y, url_w, url_h);
    let focused = browser.url_focused;
    fb.fill_rect(url_rect, if focused { BG_URL_FOCUS } else { BG_URL });
    fb.stroke_rect(url_rect, if focused { ACCENT } else { HAIRLINE });

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
    if !page.html.is_empty() {
        let extra = browser.profile.user_css();
        let frame = layout_html(&page.html, &extra, max_w as f32);
        y = paint_display_list(fb, text, &frame.display, x, y, viewport, m, hits);
    }
    for (i, item) in page.items.iter().enumerate() {
        if y > viewport.bottom() {
            break;
        }
        if !page.html.is_empty() {
            match item {
                PageItem::Heading(_, _) | PageItem::Text(_) | PageItem::Link { .. } => continue,
                PageItem::Field { .. } => {}
            }
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
) -> i32 {
    let mut bottom = origin_y;
    for op in &list.ops {
        match op {
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
                text: body,
                href,
                max_width,
                wrap,
            } => {
                let px = origin_x + *x as i32;
                let py = origin_y + *y as i32;
                let drawn = text.draw(
                    fb,
                    body,
                    DrawText {
                        x: px,
                        y: py,
                        max_width: *max_width,
                        font_size: *size,
                        line_height: *size * 1.4,
                        color: *color,
                        weight: if href.is_some() {
                            Weight::MEDIUM
                        } else {
                            Weight::NORMAL
                        },
                        ellipsis: false,
                        wrap: *wrap,
                    },
                );
                if let Some(href) = href {
                    hits.push(HitRegion {
                        rect: Rect::new(px, py, drawn.0.max(*max_width as i32), drawn.1.max(8)),
                        hit: Hit::ContentLink(href.clone()),
                    });
                }
                bottom = bottom.max(py + drawn.1);
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
    let pad = m.content_pad();
    let max_w = m.content_max_w().min(viewport.w - pad * 2);
    let x = viewport.x + ((viewport.w - max_w) / 2).max(pad);
    let mut y = viewport.y + pad - browser.active_tab().scroll_y as i32;

    for block in blocks {
        if y > viewport.bottom() {
            break;
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
                        color: TEXT_CONTENT,
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
                fb.fill_rect(Rect::new(x, y + 6, max_w, 1), 0x00D9D0C4);
                y += m.s(18.0);
            }
            Block::KeyValue { key, value } => {
                text.draw(
                    fb,
                    key,
                    DrawText {
                        x,
                        y,
                        max_width: (max_w / 3) as f32,
                        font_size: m.font_ui(),
                        line_height: m.line_content(),
                        color: TEXT_CONTENT_MUTED,
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
                fb.fill_rect(Rect::new(x, y + row_h - 1, max_w, 1), 0x00E4DBCE);
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
                text.draw(
                    fb,
                    description,
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
                fb.fill_rect(sw, if *value { BG_TOGGLE_ON } else { BG_TOGGLE_OFF });
                let knob_x = if *value {
                    sw.right() - m.s(20.0)
                } else {
                    sw.x + m.s(4.0)
                };
                fb.fill_rect(
                    Rect::new(knob_x, sw.y + m.s(4.0), m.s(16.0), m.s(16.0)),
                    BG_CONTENT,
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
    if browser.active_tab().circuit != crate::state::Circuit::Direct {
        label = format!("{}  ·  {label}", browser.active_tab().circuit.label());
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
