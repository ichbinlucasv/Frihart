//! Documents, about: pages, session history.

#![forbid(unsafe_code)]

mod about;
mod document;
mod session;

use frihart_autofill::{FieldKind, classify};
use frihart_blocker::FilterEngine;
use frihart_core::{ContainerId, UrlKind, about_page, classify_url, safe_host};
use frihart_html::{document_title, parse, visible_blocks};
use frihart_net::{
    CookieJar, FetchMode, HttpClient, Request, RustlsClient, content_type, decode_body,
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
            if url.scheme() == "view-source" {
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
        req.mode,
        req.container,
    ) {
        Ok(resp) => {
            let text = decode_body(&resp);
            let host = safe_host(&resp.final_url);
            let ct = content_type(&resp);
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
        Err(err) => Document::unavailable(req.url.clone(), err.to_string()),
    }
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
            frihart_html::Block::Text(t) => items.push(PageItem::Text(t)),
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
    Page { url, title, items }
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
}
