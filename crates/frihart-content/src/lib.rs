//! Documents, about: pages, session history.

#![forbid(unsafe_code)]

mod about;
mod document;
mod session;
mod sites;

use frihart_autofill::{FieldKind, classify};
use frihart_blocker::FilterEngine;
use frihart_core::{ContainerId, UrlKind, about_page, classify_url, is_script_scheme, safe_host};
use frihart_html::{document_title, parse, visible_blocks};
use frihart_net::{
    CookieJar, DownloadLog, DownloadRecord, FetchMode, HttpClient, Request, RustlsClient,
    classify_error, content_type, decode_body, save_download, should_save,
};
use frihart_privacy::Policy;
use frihart_profile::Profile;
use url::Url;

pub use about::PrefToggle;
pub use document::{Block, Document, InternalPage, Page, PageItem};
pub use session::{SessionEntry, SessionHistory};

pub fn load(url: &Url, profile: &Profile) -> Document {
    let prefs = profile.prefs();
    match classify_url(url) {
        UrlKind::About(name) => about::page(&name, url, prefs, profile),
        UrlKind::Https | UrlKind::Http => https_only_or_empty(url, prefs),
        UrlKind::File => Document::unavailable(url.clone(), "file scheme disabled"),
        UrlKind::Other => {
            if is_script_scheme(url.scheme()) {
                Document::unavailable(url.clone(), "javascript: refused")
            } else if url.scheme() == "view-source" {
                Document::unavailable(url.clone(), "view-source")
            } else {
                Document::unavailable(url.clone(), "scheme refused")
            }
        }
    }
}

fn https_only_or_empty(url: &Url, prefs: &frihart_config::Prefs) -> Document {
    let policy = Policy::from_prefs(prefs);
    if url.scheme() != "https" && policy.https_only() {
        return Document::internal(InternalPage {
            title: "HTTPS-only".into(),
            url: url.clone(),
            blocks: vec![
                Block::Hero {
                    title: "HTTPS-only".into(),
                    subtitle: safe_host(url),
                },
                Block::Link {
                    label: "settings".into(),
                    href: "about:settings".into(),
                },
            ],
        });
    }
    Document::unavailable(url.clone(), "offline")
}

pub struct FetchRequest<'a> {
    pub url: &'a Url,
    pub profile: &'a Profile,
    pub client: &'a RustlsClient,
    pub jar: &'a mut CookieJar,
    pub blocker: &'a FilterEngine,
    pub container: ContainerId,
    pub mode: FetchMode,
}

pub fn fetch(req: FetchRequest<'_>) -> Document {
    if req.url.scheme() == "about" {
        return load(req.url, req.profile);
    }
    let policy = Policy::from_prefs(req.profile.prefs());
    if req.url.scheme() != "https" && policy.https_only() {
        return https_only_or_empty(req.url, req.profile.prefs());
    }
    match req.client.send(
        Request::get(req.url.clone()),
        &policy,
        req.jar,
        req.blocker,
        req.mode.clone(),
        req.container,
    ) {
        Ok(resp) => {
            let text = decode_body(&resp);
            let host = safe_host(&resp.final_url);
            let ct = content_type(&resp);
            if should_save(&ct, &resp.headers) && !is_html(&ct, &text) {
                return saved_download(req, &resp);
            }
            if is_html(&ct, &text) {
                Document::Page(page_from_html(resp.final_url, host, &text))
            } else {
                Document::Source {
                    url: resp.final_url,
                    title: host,
                    text,
                }
            }
        }
        Err(err) => network_error(req.url, err),
    }
}

fn network_error(url: &url::Url, err: frihart_core::FrihartError) -> Document {
    let kind = classify_error(&err);
    Document::internal(InternalPage {
        title: kind.title().into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: kind.title().into(),
                subtitle: safe_host(url),
            },
            Block::Paragraph(kind.hint().into()),
            Block::Note(err.to_string()),
            Block::Link {
                label: "Tor".into(),
                href: "about:tor".into(),
            },
            Block::Link {
                label: "Privacy".into(),
                href: "about:privacy".into(),
            },
        ],
    })
}

fn is_html(content_type: &str, body: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("html") {
        return true;
    }
    let t = body.trim_start();
    t.starts_with("<!doctype") || t.starts_with("<!DOCTYPE") || t.starts_with("<html")
}

fn page_from_html(url: url::Url, host: String, html: &str) -> Page {
    let root = parse(html);
    let title = document_title(&root);
    let title = if title.is_empty() { host } else { title };
    let mut items = Vec::new();
    for block in visible_blocks(&root) {
        match block {
            frihart_html::Block::Heading(l, t) => items.push(PageItem::Heading(l, t)),
            frihart_html::Block::Text(t)
            | frihart_html::Block::Pre(t)
            | frihart_html::Block::Quote(t)
            | frihart_html::Block::Caption(t) => items.push(PageItem::Text(t)),
            frihart_html::Block::Rule => {}
            frihart_html::Block::ListItem { text, .. } => items.push(PageItem::Text(text)),
            frihart_html::Block::Image { alt, src } => {
                items.push(PageItem::Text(if alt.is_empty() { src } else { alt }));
            }
            frihart_html::Block::TableRow { cells } => {
                items.push(PageItem::Text(cells.join("  ·  ")));
            }
            frihart_html::Block::Link { text, href } => items.push(PageItem::Link { text, href }),
            frihart_html::Block::Field(f) => {
                let kind = classify(&f);
                let secret = kind == FieldKind::Password;
                let label = if f.label.is_empty() {
                    if f.name.is_empty() {
                        f.input_type.clone()
                    } else {
                        f.name.clone()
                    }
                } else {
                    f.label
                };
                items.push(PageItem::Field {
                    kind,
                    label,
                    value: String::new(),
                    secret,
                });
            }
        }
    }
    let (form_action, form_method) = frihart_html::first_form(&root);
    Page {
        url,
        title,
        items,
        form_action,
        form_method,
        html: html.to_string(),
    }
}

fn saved_download(req: FetchRequest<'_>, resp: &frihart_net::Response) -> Document {
    let dir = frihart_platform::downloads_dir();
    match save_download(&dir, &resp.final_url, &resp.headers, &resp.body) {
        Ok(saved) => {
            if !req.profile.is_ephemeral() {
                let log_path = req.profile.root().join("downloads.json");
                let mut log = DownloadLog::load(&log_path).unwrap_or_default();
                log.push(DownloadRecord {
                    url: resp.final_url.as_str().into(),
                    dest: saved.dest.display().to_string(),
                    bytes: saved.bytes,
                });
                let _ = log.save(&log_path);
            }
            Document::internal(crate::document::InternalPage {
                title: "Saved".into(),
                url: resp.final_url.clone(),
                blocks: vec![
                    crate::document::Block::Hero {
                        title: "Saved".into(),
                        subtitle: saved.dest.display().to_string(),
                    },
                    crate::document::Block::Paragraph(format!(
                        "{} bytes. Not executed.",
                        saved.bytes
                    )),
                    crate::document::Block::Link {
                        label: "Downloads".into(),
                        href: "about:downloads".into(),
                    },
                ],
            })
        }
        Err(err) => Document::unavailable(req.url.clone(), err.to_string()),
    }
}

pub fn about_url(name: &str) -> Url {
    Url::parse(&format!("about:{name}"))
        .unwrap_or_else(|_| Url::parse("about:blank").expect("about:blank"))
}

pub fn is_known_about(url: &Url) -> bool {
    if !matches!(classify_url(url), UrlKind::About(_)) {
        return false;
    }
    about::is_known(&about_page(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_home_loads() {
        let profile = Profile::ephemeral().unwrap();
        let doc = load(&about_url("home"), &profile);
        assert_eq!(doc.title(), "Home");
        assert!(matches!(doc, Document::Internal(_)));
    }

    #[test]
    fn http_blocked_when_https_only() {
        let profile = Profile::ephemeral().unwrap();
        let url = Url::parse("http://example.com").unwrap();
        let doc = load(&url, &profile);
        assert_eq!(doc.title(), "HTTPS-only");
    }

    #[test]
    fn about_blank_is_blank() {
        let profile = Profile::ephemeral().unwrap();
        let doc = load(&about_url("blank"), &profile);
        assert!(matches!(doc, Document::Blank));
    }

    #[test]
    fn engine_and_process_pages_exist() {
        let profile = Profile::ephemeral().unwrap();
        assert_eq!(load(&about_url("engine"), &profile).title(), "Engine");
        assert_eq!(load(&about_url("processes"), &profile).title(), "Processes");
        assert_eq!(load(&about_url("print"), &profile).title(), "Print");
        assert_eq!(load(&about_url("downloads"), &profile).title(), "Downloads");
        assert_eq!(load(&about_url("linux"), &profile).title(), "Linux");
        assert_eq!(load(&about_url("campaigns"), &profile).title(), "Campaigns");
        assert_eq!(load(&about_url("script"), &profile).title(), "Script");
        assert_eq!(load(&about_url("sites"), &profile).title(), "Sites");
    }

    #[test]
    fn javascript_url_is_refused() {
        let profile = Profile::ephemeral().unwrap();
        let url = Url::parse("javascript:alert(1)").unwrap();
        let doc = load(&url, &profile);
        assert_eq!(doc.title(), "Unavailable");
        assert!(doc.searchable_text().contains("javascript: refused"));
        let bypass = Url::parse("javascript://x.test/%0Aalert(1)").unwrap();
        assert!(
            load(&bypass, &profile)
                .searchable_text()
                .contains("javascript: refused")
        );
    }
}
