//! Documents, about: pages, session history.

#![forbid(unsafe_code)]

mod about;
mod document;
mod session;

use frihart_blocker::FilterEngine;
use frihart_core::{ContainerId, UrlKind, about_page, classify_url, safe_host};
use frihart_net::{CookieJar, FetchMode, HttpClient, Request, RustlsClient, decode_body};
use frihart_privacy::Policy;
use frihart_profile::Profile;
use url::Url;

pub use about::PrefToggle;
pub use document::{Block, Document, InternalPage};
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
            Document::Source {
                url: resp.final_url,
                title: host,
                text,
            }
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
}
