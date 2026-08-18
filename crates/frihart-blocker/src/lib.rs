//! Native content blocker. Not uBlock Origin. Lists stay local.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use url::Url;

/// Why a request was blocked (or not).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockDecision {
    Allow,
    Block { host: String, list: &'static str },
}

impl BlockDecision {
    pub fn blocked(&self) -> bool {
        matches!(self, Self::Block { .. })
    }
}

/// Host-level filter engine. Cosmetic filtering lands with the HTML engine.
#[derive(Clone, Debug)]
pub struct FilterEngine {
    hosts: HashSet<String>,
    enabled: bool,
}

impl FilterEngine {
    pub fn new(enabled: bool) -> Self {
        Self {
            hosts: builtin_hosts(),
            enabled,
        }
    }

    pub fn builtin() -> Self {
        Self::new(true)
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn rule_count(&self) -> usize {
        self.hosts.len()
    }

    pub fn add_host(&mut self, host: impl Into<String>) {
        self.hosts.insert(normalize_host(&host.into()));
    }

    /// True when `url`'s host or a parent domain is on a block list.
    pub fn decide(&self, url: &Url) -> BlockDecision {
        if !self.enabled {
            return BlockDecision::Allow;
        }
        let Some(host) = url.host_str() else {
            return BlockDecision::Allow;
        };
        let host = normalize_host(host);
        if let Some(matched) = matching_host(&self.hosts, &host) {
            return BlockDecision::Block {
                host: matched,
                list: "frihart-builtin",
            };
        }
        BlockDecision::Allow
    }

    pub fn sample(&self, n: usize) -> Vec<String> {
        let mut hosts: Vec<_> = self.hosts.iter().cloned().collect();
        hosts.sort();
        hosts.into_iter().take(n).collect()
    }
}

fn normalize_host(host: &str) -> String {
    host.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn matching_host(set: &HashSet<String>, host: &str) -> Option<String> {
    let mut cur = host;
    loop {
        if set.contains(cur) {
            return Some(cur.to_string());
        }
        match cur.split_once('.') {
            Some((_, rest)) if rest.contains('.') => cur = rest,
            _ => return None,
        }
    }
}

/// Built-in seed of well-known advertising, analytics, and beacon hosts.
/// This is a *seed*, not a claim of uBO list parity.
fn builtin_hosts() -> HashSet<String> {
    const HOSTS: &[&str] = &[
        "doubleclick.net",
        "googleadservices.com",
        "googlesyndication.com",
        "google-analytics.com",
        "googletagmanager.com",
        "googletagservices.com",
        "adservice.google.com",
        "pagead2.googlesyndication.com",
        "stats.g.doubleclick.net",
        "facebook.net",
        "connect.facebook.net",
        "pixel.facebook.com",
        "an.facebook.com",
        "ads-twitter.com",
        "analytics.twitter.com",
        "static.ads-twitter.com",
        "t.co",
        "scorecardresearch.com",
        "quantserve.com",
        "quantcast.com",
        "hotjar.com",
        "hotjar.io",
        "fullstory.com",
        "mouseflow.com",
        "clarity.ms",
        "bat.bing.com",
        "ads.linkedin.com",
        "snap.licdn.com",
        "ads.yahoo.com",
        "analytics.yahoo.com",
        "gemini.yahoo.com",
        "taboola.com",
        "outbrain.com",
        "criteo.com",
        "criteo.net",
        "casalemedia.com",
        "openx.net",
        "pubmatic.com",
        "rubiconproject.com",
        "2mdn.net",
        "adnxs.com",
        "adsafeprotected.com",
        "advertising.com",
        "adform.net",
        "adroll.com",
        "adsrvr.org",
        "adsymptotic.com",
        "amazon-adsystem.com",
        "media-amazon.com",
        "serving-sys.com",
        "smartadserver.com",
        "moatads.com",
        "moatpixel.com",
        "newrelic.com",
        "nr-data.net",
        "segment.io",
        "segment.com",
        "mixpanel.com",
        "amplitude.com",
        "sentry.io",
        "bugsnag.com",
        "branch.io",
        "appsflyer.com",
        "adjust.com",
        "kochava.com",
        "chartbeat.com",
        "chartbeat.net",
        "parsely.com",
        "parse.ly",
        "krxd.net",
        "bluekai.com",
        "exelator.com",
        "demdex.net",
        "omtrdc.net",
        "everesttech.net",
        "mathtag.com",
        "rlcdn.com",
        "tapad.com",
        "agkn.com",
        "addthis.com",
        "addthisedge.com",
        "sharethis.com",
        "optimizely.com",
        "crazyegg.com",
        "inspectlet.com",
        "luckyorange.com",
        "mc.yandex.ru",
        "an.yandex.ru",
        "ads.tiktok.com",
        "analytics.tiktok.com",
        "ads-api.twitter.com",
        "tr.snapchat.com",
        "sc-static.net",
        "ct.pinterest.com",
        "log.pinterest.com",
        "ads.pinterest.com",
    ];
    HOSTS.iter().map(|h| (*h).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_tracker_and_subdomain() {
        let engine = FilterEngine::builtin();
        let url = Url::parse("https://pagead2.googlesyndication.com/pagead/js").unwrap();
        assert!(engine.decide(&url).blocked());
        let sub = Url::parse("https://stats.g.doubleclick.net/g/collect").unwrap();
        assert!(engine.decide(&sub).blocked());
    }

    #[test]
    fn allows_first_party() {
        let engine = FilterEngine::builtin();
        let url = Url::parse("https://example.com/index.html").unwrap();
        assert!(!engine.decide(&url).blocked());
    }

    #[test]
    fn can_be_disabled() {
        let mut engine = FilterEngine::builtin();
        engine.set_enabled(false);
        let url = Url::parse("https://doubleclick.net/").unwrap();
        assert!(!engine.decide(&url).blocked());
    }

    #[test]
    fn seed_is_nontrivial() {
        assert!(FilterEngine::builtin().rule_count() >= 80);
    }
}
