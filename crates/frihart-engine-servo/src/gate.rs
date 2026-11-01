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
//! `X-Requested-With`, `Referer`, `Ping-From`, `Ping-To`, legacy client
//! hints, Network Information, `Sec-Purpose`); a User-Agent other
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
//!
//! Every decision is also kept as an [`ObservedLoad`] (scheme, host,
//! header names, verdict — no header values and no URL path). [`audit_loads`]
//! fails closed if a load that was allowed breaks that policy. The log is
//! capped; a truncated log is itself a failure.

use std::sync::Mutex;
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
    // Legacy client hints (the `Sec-CH-*` forms are covered by the prefix).
    "dpr",
    "device-memory",
    "viewport-width",
    // Network Information API. `width` is intentionally not here: cancelling
    // a request because of a header with that common name would drop images
    // if Servo ever sent it, and it is not a default request header.
    "downlink",
    "rtt",
    "ect",
    "save-data",
    // Speculative prefetch (`prefetch`, `prerender`). Compat does not prefetch.
    "sec-purpose",
    "purpose",
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
            Self::IdentifyingHeader => "identifying header",
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

/// How the User-Agent on one load compared to Frihart's frozen value.
/// The header text itself is not stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UaCheck {
    /// The request had no User-Agent header.
    Absent,
    /// The request carried [`frihart_core::FROZEN_USER_AGENT`].
    Frozen,
    /// The request carried some other User-Agent.
    Foreign,
}

/// One load the gate saw. Header values and the URL path are not kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedLoad {
    pub scheme: String,
    pub host: Option<String>,
    pub header_names: Vec<String>,
    pub user_agent: UaCheck,
    pub origin: Option<String>,
    pub document_host: Option<String>,
    pub verdict: Result<(), Refusal>,
}

/// Why a transcript cannot be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditFail {
    /// More loads arrived than the log keeps. Treat that as a leak: the
    /// missing ones were not checked.
    Truncated,
    /// A load was allowed even though policy refuses it.
    Allowed(Refusal),
}

/// Most loads kept for one Servo instance. Past this the audit fails closed.
pub const MAX_OBSERVED_LOADS: usize = 64;

/// Longest `Origin` value copied into the log. Longer values are stored as
/// a marker the origin check refuses, so a huge header cannot hide.
const MAX_STORED_ORIGIN: usize = 256;

/// Re-run policy on a recorded load. An allowed load that policy refuses
/// is a leak. A refused load is not: the request never left Servo.
pub fn audit_loads(loads: &[ObservedLoad]) -> Result<(), AuditFail> {
    for load in loads {
        if load.verdict.is_err() {
            continue;
        }
        let ua = match load.user_agent {
            UaCheck::Absent => None,
            UaCheck::Frozen => Some(frihart_core::FROZEN_USER_AGENT),
            UaCheck::Foreign => Some("foreign"),
        };
        if let Err(reason) = check_request(
            &load.scheme,
            load.header_names.iter().map(String::as_str),
            ua,
            load.document_host.as_deref(),
            load.host.as_deref(),
            load.origin.as_deref(),
        ) {
            return Err(AuditFail::Allowed(reason));
        }
    }
    Ok(())
}

#[derive(Debug, Default)]
struct GateLog {
    loads: Vec<ObservedLoad>,
    truncated: bool,
}

/// Counters and the leak transcript for one Servo instance (shared with
/// Servo's delegate).
#[derive(Debug, Default)]
pub struct GateStats {
    pub allowed: AtomicU64,
    pub refused: AtomicU64,
    log: Mutex<GateLog>,
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

    /// Decide one load, count it, and append it to the transcript.
    ///
    /// `header_names` are stored as sent (the check itself is
    /// case-insensitive). `origin` longer than [`MAX_STORED_ORIGIN`] is
    /// replaced with a marker that does not parse as a document origin.
    pub fn observe<'a>(
        &self,
        scheme: &str,
        header_names: impl IntoIterator<Item = &'a str>,
        user_agent: Option<&str>,
        document_host: Option<&str>,
        request_host: Option<&str>,
        origin: Option<&str>,
    ) -> Result<(), Refusal> {
        let header_names: Vec<String> = header_names.into_iter().map(str::to_string).collect();
        let verdict = check_request(
            scheme,
            header_names.iter().map(String::as_str),
            user_agent,
            document_host,
            request_host,
            origin,
        );
        let user_agent = match user_agent {
            None => UaCheck::Absent,
            Some(ua) if ua == frihart_core::FROZEN_USER_AGENT => UaCheck::Frozen,
            Some(_) => UaCheck::Foreign,
        };
        let origin = origin.map(|o| {
            if o.len() <= MAX_STORED_ORIGIN {
                o.to_string()
            } else {
                "too-long".to_string()
            }
        });
        let observed = ObservedLoad {
            scheme: scheme.to_string(),
            host: request_host.map(str::to_string),
            header_names,
            user_agent,
            origin,
            document_host: document_host.map(str::to_string),
            verdict,
        };
        {
            let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
            if log.loads.len() >= MAX_OBSERVED_LOADS {
                log.truncated = true;
            } else {
                log.loads.push(observed);
            }
        }
        self.record(verdict)
    }

    pub fn counts(&self) -> (u64, u64) {
        (
            self.allowed.load(Ordering::Relaxed),
            self.refused.load(Ordering::Relaxed),
        )
    }

    /// Copy of the transcript so far, oldest first.
    pub fn observed(&self) -> Vec<ObservedLoad> {
        self.log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .loads
            .clone()
    }

    /// Fail closed when the transcript was truncated or an allowed load
    /// breaks policy.
    pub fn audit(&self) -> Result<(), AuditFail> {
        let log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        if log.truncated {
            return Err(AuditFail::Truncated);
        }
        audit_loads(&log.loads)
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
            "DPR",
            "Device-Memory",
            "Viewport-Width",
            "Downlink",
            "RTT",
            "ECT",
            "Save-Data",
            "Sec-Purpose",
            "Purpose",
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

    fn observed(
        scheme: &str,
        host: Option<&str>,
        headers: &[&str],
        ua: UaCheck,
        origin: Option<&str>,
        document: Option<&str>,
        verdict: Result<(), Refusal>,
    ) -> ObservedLoad {
        ObservedLoad {
            scheme: scheme.to_string(),
            host: host.map(str::to_string),
            header_names: headers.iter().map(|h| (*h).to_string()).collect(),
            user_agent: ua,
            origin: origin.map(str::to_string),
            document_host: document.map(str::to_string),
            verdict,
        }
    }

    #[test]
    fn audit_accepts_refusals_and_a_clean_https_load() {
        let loads = [
            observed(
                "http",
                Some("127.0.0.1"),
                &[],
                UaCheck::Frozen,
                None,
                None,
                Err(Refusal::NotHttps),
            ),
            observed(
                "https",
                Some("example.onion"),
                &[],
                UaCheck::Frozen,
                None,
                Some("example.com"),
                Err(Refusal::HiddenNetwork),
            ),
            observed(
                "https",
                Some("frihart-test.invalid"),
                &["Accept", "User-Agent"],
                UaCheck::Frozen,
                Some("null"),
                None,
                Ok(()),
            ),
            observed(
                "data",
                Some("example.onion"),
                &["Sec-CH-UA"],
                UaCheck::Foreign,
                None,
                None,
                Ok(()),
            ),
        ];
        assert_eq!(audit_loads(&loads), Ok(()));
    }

    #[test]
    fn audit_fails_closed_on_an_allowed_leak() {
        let onion = observed(
            "https",
            Some("secret.onion"),
            &["Accept"],
            UaCheck::Frozen,
            None,
            Some("example.com"),
            Ok(()),
        );
        assert_eq!(
            audit_loads(&[onion]),
            Err(AuditFail::Allowed(Refusal::HiddenNetwork))
        );
        let hinted = observed(
            "https",
            Some("example.com"),
            &["Accept", "Save-Data"],
            UaCheck::Frozen,
            None,
            Some("example.com"),
            Ok(()),
        );
        assert_eq!(
            audit_loads(&[hinted]),
            Err(AuditFail::Allowed(Refusal::IdentifyingHeader))
        );
        let foreign = observed(
            "https",
            Some("example.com"),
            &["User-Agent"],
            UaCheck::Foreign,
            None,
            Some("example.com"),
            Ok(()),
        );
        assert_eq!(
            audit_loads(&[foreign]),
            Err(AuditFail::Allowed(Refusal::UserAgent))
        );
        let cookie = observed(
            "https",
            Some("cdn.example.com"),
            &["Cookie"],
            UaCheck::Frozen,
            None,
            Some("www.example.com"),
            Ok(()),
        );
        assert_eq!(
            audit_loads(&[cookie]),
            Err(AuditFail::Allowed(Refusal::Cookie))
        );
    }

    #[test]
    fn observe_logs_the_verdict_and_drops_header_values() {
        let s = GateStats::default();
        let verdict = s.observe(
            "https",
            ["Accept", "Save-Data"],
            Some(UA),
            None,
            Some("example.com"),
            Some("null"),
        );
        assert_eq!(verdict, Err(Refusal::IdentifyingHeader));
        assert_eq!(s.counts(), (0, 1));
        let log = s.observed();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].header_names, ["Accept", "Save-Data"]);
        assert_eq!(log[0].user_agent, UaCheck::Frozen);
        assert_eq!(log[0].verdict, Err(Refusal::IdentifyingHeader));
        assert_eq!(s.audit(), Ok(()));

        let _ = s.observe(
            "https",
            ["User-Agent"],
            Some("Mozilla/5.0 somewhere"),
            Some("example.com"),
            Some("example.com"),
            None,
        );
        assert_eq!(s.observed()[1].user_agent, UaCheck::Foreign);
        assert!(s.audit().is_ok(), "a refused foreign UA is not a leak");
    }

    #[test]
    fn a_full_transcript_fails_the_audit() {
        let s = GateStats::default();
        for _ in 0..=MAX_OBSERVED_LOADS {
            let _ = s.observe(
                "https",
                ["Accept"],
                Some(UA),
                Some("example.com"),
                Some("example.com"),
                None,
            );
        }
        assert_eq!(s.observed().len(), MAX_OBSERVED_LOADS);
        assert_eq!(s.audit(), Err(AuditFail::Truncated));
        assert_eq!(s.counts().0, MAX_OBSERVED_LOADS as u64 + 1);
    }
}
