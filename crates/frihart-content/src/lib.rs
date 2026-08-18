//! Documents, internal pages, and session history.

#![forbid(unsafe_code)]

mod about;
mod document;
mod session;

use frihart_config::Prefs;
use frihart_core::{UrlKind, about_page, classify_url};
use frihart_privacy::Policy;
use frihart_profile::Profile;
use url::Url;

pub use about::PrefToggle;
pub use document::{Block, Document, InternalPage};
pub use session::{SessionEntry, SessionHistory};

/// Load a URL into a document. `about:` never touches the network.
pub fn load(url: &Url, profile: &Profile) -> Document {
    let prefs = profile.prefs();
    match classify_url(url) {
        UrlKind::About(name) => about::page(&name, url, prefs, profile),
        UrlKind::Https | UrlKind::Http => phase_two_placeholder(url, prefs),
        UrlKind::File => Document::unavailable(
            url.clone(),
            "local files are not opened yet. That lands with the document engine.",
        ),
        UrlKind::Other => Document::unavailable(
            url.clone(),
            format!("the {} scheme is not supported.", url.scheme()),
        ),
    }
}

fn phase_two_placeholder(url: &Url, prefs: &Prefs) -> Document {
    let policy = Policy::from_prefs(prefs);
    let https = url.scheme() == "https";
    if !https && policy.https_only() {
        return Document::internal(InternalPage {
            title: "HTTPS-only".into(),
            url: url.clone(),
            blocks: vec![
                Block::Hero {
                    title: "Blocked by HTTPS-only mode".into(),
                    subtitle: url.to_string(),
                },
                Block::Paragraph(
                    "Frihart refuses cleartext HTTP by default. The network stack \
                     itself is Phase 2, so there is no exception button yet. When \
                     it exists, exceptions will be per-site and local."
                        .into(),
                ),
                Block::Link {
                    label: "Open settings".into(),
                    href: "about:settings".into(),
                },
            ],
        });
    }

    Document::internal(InternalPage {
        title: "Network is Phase 2".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "The web is not wired up yet".into(),
                subtitle: url.to_string(),
            },
            Block::Paragraph(
                "Frihart will not pretend to be a finished browser. The chrome, \
                 profile, and privacy policy are real. Fetching this URL requires \
                 the Phase 2 network stack (rustls, first-party cookies, HTTPS-only)."
                    .into(),
            ),
            Block::Paragraph(
                "Until then, stay on about: pages. They are local, they do not \
                 phone home, and they are where you learn the product."
                    .into(),
            ),
            Block::List(vec![
                "Phase 2 — sovereign network stack".into(),
                "Phase 3 — HTML and DOM".into(),
                "Phase 4 — CSS, layout, and paint".into(),
            ]),
            Block::Link {
                label: "Read the roadmap".into(),
                href: "about:roadmap".into(),
            },
            Block::Link {
                label: "Back to home".into(),
                href: "about:home".into(),
            },
        ],
    })
}

/// Convenience: parse `about:name` without going through user input.
pub fn about_url(name: &str) -> Url {
    Url::parse(&format!("about:{name}"))
        .unwrap_or_else(|_| Url::parse("about:blank").expect("about:blank is a valid URL"))
}

/// True when this URL is an internal page we know how to build.
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
    fn https_is_honest_placeholder() {
        let profile = Profile::ephemeral().unwrap();
        let url = Url::parse("https://example.com").unwrap();
        let doc = load(&url, &profile);
        assert!(doc.title().contains("Phase 2") || doc.title().contains("Network"));
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
