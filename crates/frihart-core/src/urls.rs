use url::Url;

use crate::{FrihartError, Result};

/// How Frihart treats a URL. Classification is cheap and has no I/O.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrlKind {
    /// Internal page. The string is the page name (`home`, `settings`, …).
    About(String),
    Https,
    Http,
    File,
    Other,
}

/// Parse what the user typed into the URL bar.
pub fn parse_user_input(input: &str) -> Result<Url> {
    try_parse_user_input(input).ok_or_else(|| FrihartError::InvalidUrl(input.to_string()))
}

/// True when the input should be opened as a location, not searched.
pub fn looks_like_destination(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.contains(' ') {
        return false;
    }
    if trimmed.contains("://") {
        return true;
    }
    if trimmed.starts_with("about:") || trimmed.starts_with("file:") {
        return true;
    }
    if let Ok(url) = Url::parse(trimmed) {
        if is_script_scheme(url.scheme()) || is_known_scheme(url.scheme()) {
            return true;
        }
    }
    if let Ok(url) = Url::parse(&format!("https://{trimmed}")) {
        if url
            .host_str()
            .is_some_and(|h| h.contains('.') || h == "localhost")
        {
            return true;
        }
    }
    false
}

/// Fallible parse that returns `None` instead of an error.
pub fn try_parse_user_input(input: &str) -> Option<Url> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Url::parse("about:blank").ok();
    }

    if let Ok(url) = Url::parse(trimmed) {
        // Script schemes must parse so load() can refuse them. They
        // must never become a search query (`javascript:…` to Swisscows).
        if is_script_scheme(url.scheme()) {
            return Some(url);
        }
        if url.scheme() != "http"
            || trimmed.contains("://")
            || trimmed.starts_with("http:")
            || trimmed.starts_with("https:")
        {
            // `url` crate treats some bare words as `scheme:path`. Accept
            // only real schemes we know, plus anything with `://`.
            if is_known_scheme(url.scheme()) || trimmed.contains("://") {
                return Some(url);
            }
        }
    }

    if let Ok(url) = Url::parse(&format!("https://{trimmed}")) {
        if url
            .host_str()
            .is_some_and(|h| h.contains('.') || h == "localhost")
        {
            return Some(url);
        }
        // Bare words such as "settings" become about: pages if they match,
        // otherwise still https://word — the content crate will explain
        // that the network stack is not ready.
        return Some(url);
    }

    None
}

/// `javascript:` / `vbscript:` — parseable, never executed.
pub fn is_script_scheme(scheme: &str) -> bool {
    matches!(scheme, "javascript" | "vbscript")
}

fn is_known_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "about"
            | "https"
            | "http"
            | "file"
            | "data"
            | "blob"
            | "frihart"
            | "container"
            | "view-source"
    )
}

/// Classify a parsed URL.
pub fn classify_url(url: &Url) -> UrlKind {
    match url.scheme() {
        "about" => {
            let name = about_page(url);
            UrlKind::About(name)
        }
        "https" => UrlKind::Https,
        "http" => UrlKind::Http,
        "file" => UrlKind::File,
        s if is_script_scheme(s) => UrlKind::Other,
        _ => UrlKind::Other,
    }
}

/// Page name for an `about:` URL. `about:home` → `home`, `about:` → `blank`.
pub fn about_page(url: &Url) -> String {
    let host = url.host_str().unwrap_or("");
    let path = url.path().trim_matches('/');
    let name = if !host.is_empty() {
        host
    } else if !path.is_empty() {
        path
    } else {
        "blank"
    };
    name.to_ascii_lowercase()
}

/// Compact display form for the URL bar.
pub fn display_url(url: &Url) -> String {
    if url.scheme() == "about" {
        let page = about_page(url);
        if page == "blank" && url.as_str() == "about:blank" {
            return "about:blank".to_string();
        }
        return format!("about:{page}");
    }
    url.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_blank() {
        let url = parse_user_input("").unwrap();
        assert_eq!(url.as_str(), "about:blank");
    }

    #[test]
    fn about_home() {
        let url = parse_user_input("about:home").unwrap();
        assert_eq!(classify_url(&url), UrlKind::About("home".into()));
        assert_eq!(display_url(&url), "about:home");
    }

    #[test]
    fn https_passthrough() {
        let url = parse_user_input("https://example.com/a").unwrap();
        assert_eq!(classify_url(&url), UrlKind::Https);
    }

    #[test]
    fn bare_domain_becomes_https() {
        let url = parse_user_input("example.com").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn localhost_becomes_https() {
        let url = parse_user_input("localhost:8000").unwrap();
        assert_eq!(url.host_str(), Some("localhost"));
        assert_eq!(url.port(), Some(8000));
    }

    #[test]
    fn words_are_search_not_hosts() {
        assert!(!looks_like_destination("cachyos privacy"));
        assert!(!looks_like_destination("librewolf"));
        assert!(looks_like_destination("example.com"));
        assert!(looks_like_destination("about:home"));
    }

    #[test]
    fn javascript_is_a_destination_not_search() {
        assert!(looks_like_destination("javascript:alert(1)"));
        assert!(looks_like_destination("javascript://host/%0Aalert(1)"));
        let url = parse_user_input("javascript:alert(1)").unwrap();
        assert!(is_script_scheme(url.scheme()));
        assert_eq!(classify_url(&url), UrlKind::Other);
        let bypass = parse_user_input("javascript://x.test/%0Aalert(1)").unwrap();
        assert!(is_script_scheme(bypass.scheme()));
    }
}
