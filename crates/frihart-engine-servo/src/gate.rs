//! Per-request gate for Servo loads (Campaign S M3, in-tunnel audit).
//!
//! The Compat proxy sees only `CONNECT host:443`; what Servo puts inside
//! the TLS tunnel (path, headers) never reaches it. Servo hands every HTTP
//! load to the embedder first (`WebViewDelegate::load_web_resource`), so
//! Frihart checks each request there before it leaves Servo and cancels the
//! ones that break policy. The check is kept free of Servo types so it is
//! tested in the default build.
//!
//! Allowed without checks: in-memory `data:`, `blob:` and `about:` loads.
//! Refused: any other scheme than `https` (`http`, `ws`, `file`, …); User-Agent Client Hints
//! (`Sec-CH-*`) and other identifying headers (`X-Client-Data`,
//! `X-Requested-With`); a User-Agent other than Frihart's frozen one; an
//! oversized header list. Host policy (special-use names, private
//! addresses, rebinding, blocker lists) stays with the Compat proxy.

use std::sync::atomic::{AtomicU64, Ordering};

/// Schemes served from memory, never from the network.
const LOCAL_SCHEMES: &[&str] = &["data", "blob", "about"];

/// Most request headers a Servo load may carry.
pub const MAX_REQUEST_HEADERS: usize = 64;

/// Header-name prefixes and names that identify the browser or the user.
const FORBIDDEN_PREFIXES: &[&str] = &["sec-ch-"];
const FORBIDDEN_NAMES: &[&str] = &["x-client-data", "x-requested-with"];

/// Why a load was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotHttps,
    IdentifyingHeader,
    UserAgent,
    TooManyHeaders,
}

impl Refusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotHttps => "not https",
            Self::IdentifyingHeader => "identifying header (client hints)",
            Self::UserAgent => "user agent is not the frozen one",
            Self::TooManyHeaders => "too many request headers",
        }
    }
}

/// Decide one Servo load. `header_names` are the request's header names
/// (any case); `user_agent` is its User-Agent value when present.
pub fn check_request<'a>(
    scheme: &str,
    header_names: impl IntoIterator<Item = &'a str>,
    user_agent: Option<&str>,
) -> Result<(), Refusal> {
    // In-memory documents (the page Frihart hands Servo is a `data:` URL)
    // never touch the network.
    if LOCAL_SCHEMES.iter().any(|s| scheme.eq_ignore_ascii_case(s)) {
        return Ok(());
    }
    if !scheme.eq_ignore_ascii_case("https") {
        return Err(Refusal::NotHttps);
    }
    let mut n = 0usize;
    for name in header_names {
        n += 1;
        if n > MAX_REQUEST_HEADERS {
            return Err(Refusal::TooManyHeaders);
        }
        let lower = name.to_ascii_lowercase();
        if FORBIDDEN_PREFIXES.iter().any(|p| lower.starts_with(p))
            || FORBIDDEN_NAMES.contains(&lower.as_str())
        {
            return Err(Refusal::IdentifyingHeader);
        }
    }
    if let Some(ua) = user_agent
        && ua != frihart_core::FROZEN_USER_AGENT
    {
        return Err(Refusal::UserAgent);
    }
    Ok(())
}

/// Counters for one Servo instance (shared with Servo's delegate).
#[derive(Debug, Default)]
pub struct GateStats {
    pub allowed: AtomicU64,
    pub refused: AtomicU64,
}

impl GateStats {
    /// Record a decision and pass it through.
    pub fn record(&self, r: Result<(), Refusal>) -> Result<(), Refusal> {
        let c = if r.is_ok() {
            &self.allowed
        } else {
            &self.refused
        };
        c.fetch_add(1, Ordering::Relaxed);
        r
    }

    pub fn counts(&self) -> (u64, u64) {
        (
            self.allowed.load(Ordering::Relaxed),
            self.refused.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UA: &str = frihart_core::FROZEN_USER_AGENT;

    #[test]
    fn plain_https_with_frozen_ua_passes() {
        let h = ["Host", "User-Agent", "Accept", "Accept-Language", "Cookie"];
        assert_eq!(check_request("https", h, Some(UA)), Ok(()));
        assert_eq!(check_request("HTTPS", [], None), Ok(()));
    }

    #[test]
    fn other_schemes_are_refused() {
        for s in ["http", "ws", "wss", "ftp", "file", ""] {
            assert_eq!(check_request(s, [], Some(UA)), Err(Refusal::NotHttps));
        }
        for s in ["data", "blob", "about"] {
            assert_eq!(check_request(s, ["Sec-CH-UA"], None), Ok(()), "{s}");
        }
    }

    #[test]
    fn client_hints_and_identifying_headers_are_refused() {
        for h in [
            "Sec-CH-UA",
            "sec-ch-ua-platform",
            "X-Client-Data",
            "X-Requested-With",
        ] {
            assert_eq!(
                check_request("https", ["Accept", h], Some(UA)),
                Err(Refusal::IdentifyingHeader),
                "{h}"
            );
        }
        // Fetch metadata is not identifying.
        assert_eq!(
            check_request("https", ["Sec-Fetch-Mode", "Sec-Fetch-Site"], Some(UA)),
            Ok(())
        );
    }

    #[test]
    fn foreign_user_agent_and_header_floods_are_refused() {
        assert_eq!(
            check_request("https", ["User-Agent"], Some("Mozilla/5.0 Servo/0.7")),
            Err(Refusal::UserAgent)
        );
        let many = vec!["X-A"; MAX_REQUEST_HEADERS + 1];
        assert_eq!(
            check_request("https", many, Some(UA)),
            Err(Refusal::TooManyHeaders)
        );
    }

    #[test]
    fn stats_count_both_ways() {
        let s = GateStats::default();
        let _ = s.record(Ok(()));
        let _ = s.record(Err(Refusal::NotHttps));
        let _ = s.record(Err(Refusal::UserAgent));
        assert_eq!(s.counts(), (1, 2));
    }
}
