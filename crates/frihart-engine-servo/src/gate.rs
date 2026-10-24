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
//! `X-Requested-With`, `Referer`, `Ping-From`, `Ping-To`); a User-Agent other
//! than Frihart's frozen one; an oversized header list; a `Cookie` or
//! `Authorization` whose document host and request host are not the same
//! site; `Proxy-Authorization` always (that credential belongs to a proxy,
//! and the origin would see it inside the tunnel). The page Servo is
//! handed today is a `data:` URL, which has no host, so `Cookie` and
//! `Authorization` are refused. `Origin` may be `null` (the opaque
//! `data:` origin). Any other origin must be `https` on that document
//! host, default port only. A `.onion` or `.i2p` host on the document,
//! the request, or the origin is refused here: Servo never starts a Tor
//! or I2P load. Other special-use names, private addresses, rebinding and
//! blocker lists stay with the Compat proxy (an `.invalid` name still
//! reaches it, which is how the Servo feature test shows the proxy is on
//! the path).

use std::sync::atomic::{AtomicU64, Ordering};

/// Schemes served from memory, never from the network.
const LOCAL_SCHEMES: &[&str] = &["data", "blob", "about"];

/// Most request headers a Servo load may carry.
pub const MAX_REQUEST_HEADERS: usize = 64;

/// Header-name prefixes and names that identify the browser or the user.
const FORBIDDEN_PREFIXES: &[&str] = &["sec-ch-"];
const FORBIDDEN_NAMES: &[&str] = &[
    "x-client-data",
    "x-requested-with",
    "referer",
    "referrer",
    "ping-from",
    "ping-to",
];

/// Why a load was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotHttps,
    IdentifyingHeader,
    UserAgent,
    TooManyHeaders,
    /// A cookie the proxy cannot see, and that is not tied to the document site.
    Cookie,
    /// `Authorization` that is not same-site, or any `Proxy-Authorization`.
    Credential,
    /// `Origin` is not `null` and not `https` on the document host.
    Origin,
    /// Document, request, or origin host is Tor (`.onion`) or I2P (`.i2p`).
    HiddenNetwork,
}

impl Refusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotHttps => "not https",
            Self::IdentifyingHeader => "identifying header (client hints)",
            Self::UserAgent => "user agent is not the frozen one",
            Self::TooManyHeaders => "too many request headers",
            Self::Cookie => "cookie is not same-site with the document",
            Self::Credential => "authorization credential",
            Self::Origin => "origin is not the document site",
            Self::HiddenNetwork => "onion or i2p host",
        }
    }
}

/// True for a Tor (`.onion`) or I2P (`.i2p`) host.
///
/// Real names use [`frihart_core::hidden_net_host`], so this gate and the
/// rest of Frihart share one suffix rule (trailing DNS dot included). The
/// bare suffix (`onion`, `i2p`) is refused too, matching the Compat proxy.
/// A label that only contains those letters (`notonion.com`,
/// `onion.example.com`) does not: the suffix has to be its own label.
pub fn hidden_network_host(host: &str) -> bool {
    let h = host.trim();
    if frihart_core::hidden_net_host(h).is_some() {
        return true;
    }
    matches!(
        h.trim_end_matches('.').to_ascii_lowercase().as_str(),
        "onion" | "i2p"
    )
}

/// True when two hosts are equal, or one is a parent of the other.
///
/// `notevil.com` does not share a site with `evil.com`. Sibling names such
/// as `www.example.com` and `cdn.example.com` do not either: without a
/// public-suffix list, sharing only a parent label is not enough.
pub fn hosts_share_site(document_host: &str, request_host: &str) -> bool {
    fn label(host: &str) -> Option<String> {
        let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
        if h.is_empty() || h.bytes().any(|c| c == b':' || c == b'/' || c == b' ') {
            return None;
        }
        Some(h)
    }
    let (Some(a), Some(b)) = (label(document_host), label(request_host)) else {
        return false;
    };
    if a == b {
        return true;
    }
    let (longer, shorter) = if a.len() >= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    longer.len() > shorter.len()
        && longer.as_bytes()[longer.len() - shorter.len() - 1] == b'.'
        && longer.ends_with(shorter.as_str())
}

/// Decide one Servo load. `header_names` are the request's header names
/// (any case); `user_agent` is its User-Agent value when present.
///
/// `document_host` is the page's host. `request_host` is this load's host.
/// A `Cookie` or `Authorization` is allowed only when those two share a
/// site. `None` refuses both: the current Compat document is a `data:` URL
/// and has no host. `origin` is the `Origin` header value when Servo sent
/// one. `null` is allowed. Anything else must be `https` on `document_host`
/// (exact host, port 443 or omitted). A document, request, or origin host
/// on `.onion` or `.i2p` refuses the load.
pub fn check_request<'a>(
    scheme: &str,
    header_names: impl IntoIterator<Item = &'a str>,
    user_agent: Option<&str>,
    document_host: Option<&str>,
    request_host: Option<&str>,
    origin: Option<&str>,
) -> Result<(), Refusal> {
    // In-memory documents (the page Frihart hands Servo is a `data:` URL)
    // never touch the network.
    if LOCAL_SCHEMES.iter().any(|s| scheme.eq_ignore_ascii_case(s)) {
        return Ok(());
    }
    if !scheme.eq_ignore_ascii_case("https") {
        return Err(Refusal::NotHttps);
    }
    // Servo is clearnet Compat only. Refuse before header checks so a
    // hidden-network load is cancelled even when the rest of the request
    // would have passed.
    if document_host.is_some_and(hidden_network_host)
        || request_host.is_some_and(hidden_network_host)
    {
        return Err(Refusal::HiddenNetwork);
    }
    let mut n = 0usize;
    let mut saw_cookie = false;
    let mut saw_authorization = false;
    for name in header_names {
        n += 1;
        if n > MAX_REQUEST_HEADERS {
            return Err(Refusal::TooManyHeaders);
        }
        let lower = name.to_ascii_lowercase();
        if lower == "cookie" {
            saw_cookie = true;
        }
        if lower == "authorization" {
            saw_authorization = true;
        }
        // The proxy never asks for this, and the origin sits on the other
        // side of the tunnel, so it must not see a proxy credential.
        if lower == "proxy-authorization" {
            return Err(Refusal::Credential);
        }
        if FORBIDDEN_PREFIXES.iter().any(|p| lower.starts_with(p))
            || FORBIDDEN_NAMES.contains(&lower.as_str())
        {
            return Err(Refusal::IdentifyingHeader);
        }
    }
    let same_site = match (document_host, request_host) {
        (Some(document), Some(request)) => hosts_share_site(document, request),
        _ => false,
    };
    if saw_cookie && !same_site {
        return Err(Refusal::Cookie);
    }
    if saw_authorization && !same_site {
        return Err(Refusal::Credential);
    }
    if let Some(origin) = origin {
        match classify_origin(origin, document_host) {
            OriginCheck::Allow => {}
            OriginCheck::Hidden => return Err(Refusal::HiddenNetwork),
            OriginCheck::Refuse => return Err(Refusal::Origin),
        }
    }
    if let Some(ua) = user_agent
        && ua != frihart_core::FROZEN_USER_AGENT
    {
        return Err(Refusal::UserAgent);
    }
    Ok(())
}

/// What an `Origin` header means for this load.
enum OriginCheck {
    Allow,
    /// The origin host is `.onion` or `.i2p`.
    Hidden,
    Refuse,
}

/// `null` is the opaque origin of a `data:` document. Any other value has
/// to be `https` on the document host itself (not a parent or a sibling):
/// that is the origin the page actually has. An explicit port is only
/// allowed when it is 443.
fn classify_origin(origin: &str, document_host: Option<&str>) -> OriginCheck {
    let o = origin.trim();
    if o.eq_ignore_ascii_case("null") {
        return OriginCheck::Allow;
    }
    let Some(rest) = o
        .get(.."https://".len())
        .filter(|prefix| prefix.eq_ignore_ascii_case("https://"))
        .map(|_| &o["https://".len()..])
    else {
        return OriginCheck::Refuse;
    };
    if rest.is_empty()
        || rest
            .bytes()
            .any(|b| matches!(b, b'/' | b'?' | b'#' | b'@' | b' ' | b'\\'))
    {
        return OriginCheck::Refuse;
    }
    let Some((host, port)) = split_origin_host(rest) else {
        return OriginCheck::Refuse;
    };
    if port.is_some_and(|p| p != "443") {
        return OriginCheck::Refuse;
    }
    if hidden_network_host(host) {
        return OriginCheck::Hidden;
    }
    let Some(document) = document_host else {
        return OriginCheck::Refuse;
    };
    if hosts_equal(document, host) {
        OriginCheck::Allow
    } else {
        OriginCheck::Refuse
    }
}

/// Host and optional port from the part after `https://`.
///
/// IPv6 is accepted only in brackets (`[2001:db8::1]`, `[::1]:443`).
fn split_origin_host(rest: &str) -> Option<(&str, Option<&str>)> {
    if let Some(body) = rest.strip_prefix('[') {
        let (addr, after) = body.split_once(']')?;
        if addr.is_empty() {
            return None;
        }
        let port = match after {
            "" => None,
            p if p.starts_with(':') => Some(&p[1..]),
            _ => return None,
        };
        return Some((addr, port));
    }
    if let Some((host, port)) = rest.rsplit_once(':') {
        if host.is_empty() || port.is_empty() || host.contains(':') {
            return None;
        }
        return Some((host, Some(port)));
    }
    Some((rest, None))
}

fn hosts_equal(a: &str, b: &str) -> bool {
    let norm = |host: &str| host.trim().trim_end_matches('.').to_ascii_lowercase();
    let (a, b) = (norm(a), norm(b));
    // Colons stay: an IPv6 host is already split from its port.
    !a.is_empty()
        && a == b
        && !a
            .bytes()
            .any(|c| matches!(c, b'/' | b' ' | b'@' | b'\\' | b'?' | b'#' | b'[' | b']'))
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
        assert_eq!(
            check_request(
                "https",
                h,
                Some(UA),
                Some("example.com"),
                Some("example.com"),
                None
            ),
            Ok(())
        );
        assert_eq!(check_request("HTTPS", [], None, None, None, None), Ok(()));
    }

    #[test]
    fn other_schemes_are_refused() {
        for s in ["http", "ws", "wss", "ftp", "file", ""] {
            assert_eq!(
                check_request(s, [], Some(UA), None, None, None),
                Err(Refusal::NotHttps)
            );
        }
        for s in ["data", "blob", "about"] {
            assert_eq!(
                check_request(s, ["Sec-CH-UA"], None, None, None, None),
                Ok(()),
                "{s}"
            );
        }
    }

    #[test]
    fn client_hints_and_identifying_headers_are_refused() {
        for h in [
            "Sec-CH-UA",
            "sec-ch-ua-platform",
            "X-Client-Data",
            "X-Requested-With",
            "Referer",
            "Referrer",
            "Ping-From",
            "Ping-To",
        ] {
            assert_eq!(
                check_request("https", ["Accept", h], Some(UA), None, None, None),
                Err(Refusal::IdentifyingHeader),
                "{h}"
            );
        }
        // Fetch metadata is not identifying.
        assert_eq!(
            check_request(
                "https",
                ["Sec-Fetch-Mode", "Sec-Fetch-Site"],
                Some(UA),
                None,
                None,
                None
            ),
            Ok(())
        );
    }

    #[test]
    fn cookie_requires_a_same_site_document() {
        let cookie = ["Cookie"];
        assert_eq!(
            check_request("https", cookie, Some(UA), None, Some("example.com"), None),
            Err(Refusal::Cookie)
        );
        assert_eq!(
            check_request(
                "https",
                cookie,
                Some(UA),
                Some("www.example.com"),
                Some("example.com"),
                None
            ),
            Ok(())
        );
        assert_eq!(
            check_request(
                "https",
                cookie,
                Some(UA),
                Some("www.example.com"),
                Some("cdn.example.com"),
                None
            ),
            Err(Refusal::Cookie)
        );
        assert_eq!(
            check_request(
                "https",
                cookie,
                Some(UA),
                Some("notevil.com"),
                Some("evil.com"),
                None
            ),
            Err(Refusal::Cookie)
        );
        assert!(!hosts_share_site("notevil.com", "evil.com"));
        assert!(hosts_share_site("www.example.com", "example.com"));
    }

    #[test]
    fn foreign_user_agent_and_header_floods_are_refused() {
        assert_eq!(
            check_request(
                "https",
                ["User-Agent"],
                Some("Mozilla/5.0 Servo/0.7"),
                None,
                None,
                None
            ),
            Err(Refusal::UserAgent)
        );
        let many = vec!["X-A"; MAX_REQUEST_HEADERS + 1];
        assert_eq!(
            check_request("https", many, Some(UA), None, None, None),
            Err(Refusal::TooManyHeaders)
        );
    }

    #[test]
    fn onion_and_i2p_hosts_are_refused() {
        for host in [
            "example.onion",
            "example.onion.",
            "EXAMPLE.ONION",
            "onion",
            "www.example.i2p",
            "I2P",
            "a.b.i2p.",
        ] {
            assert_eq!(
                check_request(
                    "https",
                    ["Accept"],
                    Some(UA),
                    Some("example.com"),
                    Some(host),
                    None
                ),
                Err(Refusal::HiddenNetwork),
                "{host}"
            );
            assert!(hidden_network_host(host), "{host}");
        }
        assert!(hidden_network_host("  x.onion  "));
        assert!(!hidden_network_host(""));
        assert!(!hidden_network_host("..."));
        // The hidden-network name is the reason, even if a header would
        // also have been refused.
        assert_eq!(
            check_request("https", ["Referer"], Some(UA), None, Some("x.onion"), None),
            Err(Refusal::HiddenNetwork)
        );
        // The page itself is on a hidden network: its clearnet subresources
        // are refused too. Servo must not be rendering that page.
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                Some("foo.onion"),
                Some("example.com"),
                None
            ),
            Err(Refusal::HiddenNetwork)
        );
        // Not hidden-network names. `.invalid` and private addresses stay
        // the proxy's job, so they pass this gate.
        for host in [
            "example.com",
            "notonion.com",
            "onion.example.com",
            "i2p.example.com",
            "frihart-test.invalid",
            "10.1.2.3",
        ] {
            assert_eq!(
                check_request("https", [], Some(UA), Some("example.com"), Some(host), None),
                Ok(()),
                "{host}"
            );
            assert!(!hidden_network_host(host), "{host}");
        }
        // In-memory loads are not fetches of the host.
        assert_eq!(
            check_request("data", [], None, None, Some("x.onion"), None),
            Ok(())
        );
    }

    #[test]
    fn origin_and_authorization_stay_on_the_document() {
        let doc = Some("www.example.com");
        let same = Some("www.example.com");
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                doc,
                Some("cdn.example.com"),
                Some("https://www.example.com")
            ),
            Ok(())
        );
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                doc,
                same,
                Some(" HTTPS://WWW.EXAMPLE.com:443 ")
            ),
            Ok(())
        );
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                doc,
                same,
                Some("https://[2001:db8::1]")
            ),
            Err(Refusal::Origin)
        );
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                Some("2001:db8::1"),
                Some("2001:db8::1"),
                Some("https://[2001:db8::1]:443")
            ),
            Ok(())
        );
        for origin in [
            "https://example.com",
            "https://cdn.example.com",
            "http://www.example.com",
            "https://www.example.com/path",
            "https://www.example.com:8443",
            "https://user@www.example.com",
            "https://notevil.com",
            "",
        ] {
            assert_eq!(
                check_request("https", [], Some(UA), doc, same, Some(origin)),
                Err(Refusal::Origin),
                "{origin}"
            );
        }
        // No document host: a concrete origin would invent a site. `null`
        // is the opaque origin of the data: page Servo is handed.
        assert_eq!(
            check_request(
                "https",
                [],
                Some(UA),
                None,
                Some("example.com"),
                Some("https://example.com")
            ),
            Err(Refusal::Origin)
        );
        for origin in ["null", "NULL", " null "] {
            assert_eq!(
                check_request(
                    "https",
                    [],
                    Some(UA),
                    None,
                    Some("example.com"),
                    Some(origin)
                ),
                Ok(()),
                "{origin}"
            );
        }
        // An onion origin is a hidden-network load even when the request
        // host is clearnet. That beats a plain origin mismatch.
        assert_eq!(
            check_request(
                "https",
                ["Accept"],
                Some(UA),
                doc,
                Some("example.com"),
                Some("https://secret.onion")
            ),
            Err(Refusal::HiddenNetwork)
        );
        assert_eq!(
            check_request(
                "https",
                ["Accept"],
                Some(UA),
                doc,
                Some("example.com"),
                Some("https://[foo.i2p]")
            ),
            Err(Refusal::HiddenNetwork)
        );

        let auth = ["Authorization"];
        assert_eq!(
            check_request("https", auth, Some(UA), None, Some("example.com"), None),
            Err(Refusal::Credential)
        );
        assert_eq!(
            check_request("https", auth, Some(UA), doc, Some("example.com"), None),
            Ok(())
        );
        assert_eq!(
            check_request("https", auth, Some(UA), doc, Some("cdn.example.com"), None),
            Err(Refusal::Credential)
        );
        // Same host still must not send a proxy credential to the origin.
        assert_eq!(
            check_request("https", ["Proxy-Authorization"], Some(UA), doc, same, None),
            Err(Refusal::Credential)
        );
        // A bad header name is the reason, ahead of a bad origin.
        assert_eq!(
            check_request(
                "https",
                ["Referer"],
                Some(UA),
                doc,
                same,
                Some("https://evil.com")
            ),
            Err(Refusal::IdentifyingHeader)
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
