//! Privacy-focused search catalog. Swisscows first, DuckDuckGo second.

#![forbid(unsafe_code)]

use url::Url;

/// A search engine Frihart is willing to offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchEngine {
    pub id: &'static str,
    pub name: &'static str,
    pub region: &'static str,
    /// `{q}` is replaced with a percent-encoded query.
    pub template: &'static str,
}

pub const SWISSCOWS: SearchEngine = SearchEngine {
    id: "swisscows",
    name: "Swisscows",
    region: "Switzerland",
    template: "https://swisscows.com/web?query={q}",
};

pub const DUCKDUCKGO: SearchEngine = SearchEngine {
    id: "duckduckgo",
    name: "DuckDuckGo",
    region: "USA",
    template: "https://duckduckgo.com/?q={q}",
};

pub const STARTPAGE: SearchEngine = SearchEngine {
    id: "startpage",
    name: "Startpage",
    region: "Netherlands",
    template: "https://www.startpage.com/sp/search?query={q}",
};

pub const MOJEEK: SearchEngine = SearchEngine {
    id: "mojeek",
    name: "Mojeek",
    region: "UK",
    template: "https://www.mojeek.com/search?q={q}",
};

pub const QWANT: SearchEngine = SearchEngine {
    id: "qwant",
    name: "Qwant",
    region: "France",
    template: "https://www.qwant.com/?q={q}",
};

pub const METAGER: SearchEngine = SearchEngine {
    id: "metager",
    name: "MetaGer",
    region: "Germany",
    template: "https://metager.org/meta/meta.ger3?eingabe={q}",
};

pub const BRAVE: SearchEngine = SearchEngine {
    id: "brave",
    name: "Brave Search",
    region: "USA",
    template: "https://search.brave.com/search?q={q}",
};

/// Primary, then secondary, then the rest. Order is product policy.
pub fn catalog() -> &'static [SearchEngine] {
    &[
        SWISSCOWS, DUCKDUCKGO, STARTPAGE, MOJEEK, QWANT, METAGER, BRAVE,
    ]
}

pub fn primary() -> SearchEngine {
    SWISSCOWS
}

pub fn secondary() -> SearchEngine {
    DUCKDUCKGO
}

pub fn by_id(id: &str) -> Option<SearchEngine> {
    catalog().iter().copied().find(|e| e.id == id)
}

pub fn resolve(engine: SearchEngine, query: &str) -> Option<Url> {
    let encoded = urlencoding(query);
    let raw = engine.template.replace("{q}", &encoded);
    Url::parse(&raw).ok()
}

/// Minimal query encoder. Spaces become `+`.
fn urlencoding(query: &str) -> String {
    let mut out = String::new();
    for b in query.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push('+'),
            c => out.push_str(&format!("%{c:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swisscows_is_primary() {
        assert_eq!(primary().id, "swisscows");
        assert_eq!(secondary().id, "duckduckgo");
    }

    #[test]
    fn builds_swisscows_url() {
        let url = resolve(SWISSCOWS, "cachyos privacy").unwrap();
        assert_eq!(url.host_str(), Some("swisscows.com"));
        assert!(url.as_str().contains("cachyos+privacy"));
    }

    #[test]
    fn unknown_id() {
        assert!(by_id("google").is_none());
    }
}
