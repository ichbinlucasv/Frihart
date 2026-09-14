use std::io::Read;
use std::time::Duration;

use frihart_blocker::FilterEngine;
use frihart_core::{
    CircuitKind, ContainerId, FrihartError, HiddenNet, Result, hidden_net, sanitize_error,
};
use frihart_privacy::{Policy, ResourceKind};

use crate::cookie::CookieJar;
use crate::headers::apply_identity_headers;
use crate::{HttpClient, Request, Response};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetFail {
    Tor,
    I2p,
    WrongNet,
    Tls,
    Blocked,
    Timeout,
    Other,
}

pub fn classify_error(err: &FrihartError) -> NetFail {
    let s = err.to_string().to_ascii_lowercase();
    if s.contains("wrong net") {
        NetFail::WrongNet
    } else if s.contains("tor") {
        NetFail::Tor
    } else if s.contains("i2p") {
        NetFail::I2p
    } else if s.contains("certificate") || s.contains("tls") || s.contains("ssl") {
        NetFail::Tls
    } else if s.contains("blocked") {
        NetFail::Blocked
    } else if s.contains("timed out") || s.contains("timeout") {
        NetFail::Timeout
    } else {
        NetFail::Other
    }
}

impl NetFail {
    pub fn title(self) -> &'static str {
        match self {
            Self::Tor => "Tor",
            Self::I2p => "I2P",
            Self::WrongNet => "Wrong network",
            Self::Tls => "Certificate",
            Self::Blocked => "Blocked",
            Self::Timeout => "Timeout",
            Self::Other => "Unavailable",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Tor => "SOCKS refused. No clearnet fallback. Start the system tor daemon.",
            Self::I2p => "SOCKS refused. No clearnet fallback. Start the system i2pd/I2P daemon.",
            Self::WrongNet => {
                ".onion is Tor only. .i2p is I2P only. Clearnet DNS is never used for those names."
            }
            Self::Tls => "TLS failed. Frihart will not click through a bad certificate.",
            Self::Blocked => "Policy or the native blocker stopped this request.",
            Self::Timeout => "The host did not answer in time.",
            Self::Other => "The request did not complete.",
        }
    }
}

const MAX_REDIRECTS: usize = 5;
const MAX_BODY: usize = 8 * 1024 * 1024;
const TIMEOUT_SECS: u64 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchMode {
    Direct,
    /// SOCKS5 only. Empty socks is a hard refuse — never clearnet.
    Tor {
        socks: String,
    },
    /// SOCKS5 only. Empty socks is a hard refuse — never clearnet.
    I2p {
        socks: String,
    },
}

#[derive(Clone, Debug)]
pub struct RustlsClient {
    agent: ureq::Agent,
}

impl Default for RustlsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RustlsClient {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .redirects(0)
            .user_agent("Frihart")
            .build();
        Self { agent }
    }
}

fn socks5_agent(socks: &str, kind: &str) -> Result<ureq::Agent> {
    let socks = socks.trim();
    if socks.is_empty() {
        return Err(FrihartError::network(format!("{kind} refused: no socks")));
    }
    if !socks.contains(':') {
        return Err(FrihartError::network(format!(
            "{kind} refused: socks host:port"
        )));
    }
    let proxy = ureq::Proxy::new(format!("socks5://{socks}"))
        .map_err(|_| FrihartError::network(format!("{kind} refused: socks invalid")))?;
    Ok(ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .redirects(0)
        .user_agent("Frihart")
        .proxy(proxy)
        .build())
}

/// No DNS, no socket: hidden names stay on the matching circuit.
pub fn refuse_wrong_net(mode: &FetchMode, url: &url::Url) -> Result<()> {
    match (mode, hidden_net(url)) {
        (FetchMode::Direct, Some(HiddenNet::Onion)) => Err(FrihartError::network(
            "wrong net: .onion stays on a Tor tab (no clearnet DNS)",
        )),
        (FetchMode::Direct, Some(HiddenNet::I2p)) => Err(FrihartError::network(
            "wrong net: .i2p stays on an I2P tab (no clearnet DNS)",
        )),
        (FetchMode::Tor { .. }, Some(HiddenNet::I2p)) => Err(FrihartError::network(
            "wrong net: .i2p is not reachable via Tor",
        )),
        (FetchMode::I2p { .. }, Some(HiddenNet::Onion)) => Err(FrihartError::network(
            "wrong net: .onion is not reachable via I2P",
        )),
        (FetchMode::I2p { .. }, None) => Err(FrihartError::network(
            "wrong net: I2P tabs are for .i2p names; use Tor for anonymous clearnet",
        )),
        (FetchMode::Tor { .. }, Some(HiddenNet::Onion) | None)
        | (FetchMode::I2p { .. }, Some(HiddenNet::I2p))
        | (FetchMode::Direct, None) => Ok(()),
    }
}

impl HttpClient for RustlsClient {
    fn send(
        &self,
        mut request: Request,
        policy: &Policy,
        jar: &mut CookieJar,
        blocker: &FilterEngine,
        mode: FetchMode,
        container: ContainerId,
    ) -> Result<Response> {
        refuse_wrong_net(&mode, &request.url)?;
        let circuit = match &mode {
            FetchMode::Direct => CircuitKind::Direct,
            FetchMode::Tor { .. } => CircuitKind::Tor,
            FetchMode::I2p { .. } => CircuitKind::I2p,
        };
        let agent;
        let agent_ref: &ureq::Agent = match &mode {
            FetchMode::Direct => &self.agent,
            FetchMode::Tor { socks } => {
                agent = socks5_agent(socks, "tor")?;
                &agent
            }
            FetchMode::I2p { socks } => {
                agent = socks5_agent(socks, "i2p")?;
                &agent
            }
        };

        let mut current = if policy.strip_tracking() {
            crate::strip_tracking(&request.url)
        } else {
            request.url.clone()
        };
        let first_party = current.host_str().unwrap_or("").to_ascii_lowercase();

        for _ in 0..=MAX_REDIRECTS {
            if policy.strip_tracking() {
                current = crate::strip_tracking(&current);
            }
            if crate::private_redirect(&first_party, current.host_str().unwrap_or("")) {
                return Err(FrihartError::network("blocked private"));
            }
            // .onion / .i2p http is inside an encrypted circuit. HTTPS-only
            // must not force those names onto clearnet DNS.
            let https = current.scheme() == "https" || hidden_net(&current).is_some();
            if !policy
                .decide(ResourceKind::OutboundHttp { https })
                .allowed()
            {
                return Err(FrihartError::network("blocked"));
            }
            if blocker.decide(&current).blocked() {
                return Err(FrihartError::network("blocked"));
            }

            request.url = current.clone();
            apply_identity_headers(&mut request, policy);
            if let Some(cookie) =
                jar.header_for(&current, &first_party, container, circuit, policy, false)
            {
                request.headers.push(("Cookie".into(), cookie));
            }

            let mut ureq_req = agent_ref.request(request.method.as_str(), current.as_str());
            for (k, v) in &request.headers {
                ureq_req = ureq_req.set(k, v);
            }

            let resp = match &request.body {
                Some(body) if request.method.eq_ignore_ascii_case("post") => {
                    match ureq_req.send_bytes(body) {
                        Ok(r) => r,
                        Err(ureq::Error::Status(_, r)) => r,
                        Err(e) => {
                            return Err(FrihartError::network(sanitize_error(&e.to_string())));
                        }
                    }
                }
                _ => match ureq_req.call() {
                    Ok(r) => r,
                    Err(ureq::Error::Status(_, r)) => r,
                    Err(e) => return Err(FrihartError::network(sanitize_error(&e.to_string()))),
                },
            };

            let status = resp.status();
            let headers: Vec<(String, String)> = resp
                .headers_names()
                .into_iter()
                .filter_map(|name| {
                    let value = resp.header(&name)?.to_string();
                    Some((name, value))
                })
                .collect();

            for (name, value) in &headers {
                if name.eq_ignore_ascii_case("set-cookie") {
                    let third = current.host_str().unwrap_or("") != first_party;
                    jar.store(
                        value,
                        &current,
                        &first_party,
                        container,
                        circuit,
                        policy,
                        third,
                    );
                }
            }

            if (300..400).contains(&status) {
                if let Some(loc) = headers
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case("location"))
                    .map(|(_, v)| v.clone())
                {
                    let next = current
                        .join(&loc)
                        .map_err(|_| FrihartError::network("redirect"))?;
                    current = next;
                    request.headers.retain(|(n, _)| {
                        let l = n.to_ascii_lowercase();
                        l != "cookie" && l != "user-agent" && l != "sec-gpc" && l != "dnt"
                    });
                    continue;
                }
            }

            let mut body = Vec::new();
            resp.into_reader()
                .take(MAX_BODY as u64 + 1)
                .read_to_end(&mut body)
                .map_err(|e| FrihartError::network(sanitize_error(&e.to_string())))?;
            if body.len() > MAX_BODY {
                body.truncate(MAX_BODY);
            }

            return Ok(Response {
                status,
                headers,
                body,
                final_url: current,
            });
        }

        Err(FrihartError::network("redirect"))
    }
}

pub fn decode_body(resp: &Response) -> String {
    let charset_utf8 = resp.headers.iter().any(|(n, v)| {
        n.eq_ignore_ascii_case("content-type") && v.to_ascii_lowercase().contains("utf-8")
    });
    if charset_utf8 {
        return String::from_utf8_lossy(&resp.body).into_owned();
    }
    String::from_utf8_lossy(&resp.body).into_owned()
}

pub fn content_type(resp: &Response) -> String {
    resp.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "application/octet-stream".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_blocker::FilterEngine;
    use frihart_config::Prefs;
    use frihart_privacy::Policy;
    use url::Url;

    #[test]
    fn https_only_blocks_cleartext() {
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::new(true);
        let req = Request::get(Url::parse("http://example.com").unwrap());
        let err = client
            .send(
                req,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Direct,
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(err.to_string().contains("blocked"));
    }

    #[test]
    fn tor_never_uses_clearnet() {
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::new(true);
        let req = Request::get(Url::parse("https://example.com").unwrap());
        let empty = client
            .send(
                req.clone(),
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Tor {
                    socks: String::new(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(empty.to_string().contains("tor"));
        let bad = client
            .send(
                req,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Tor {
                    socks: "noport".into(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(bad.to_string().contains("tor"));
    }

    #[test]
    fn i2p_never_uses_clearnet() {
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::new(true);
        let req = Request::get(Url::parse("https://example.com").unwrap());
        let err = client
            .send(
                req,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::I2p {
                    socks: "127.0.0.1:4447".into(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(err.to_string().contains("wrong net"));
        assert_eq!(classify_error(&err), NetFail::WrongNet);
        let hidden = Request::get(Url::parse("http://zzz.i2p/").unwrap());
        let empty = client
            .send(
                hidden,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::I2p {
                    socks: String::new(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(empty.to_string().contains("i2p"));
        assert_eq!(
            classify_error(&FrihartError::network("i2p refused: no socks")),
            NetFail::I2p
        );
    }

    #[test]
    fn onion_never_hits_clearnet_dns() {
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::new(true);
        let onion = Request::get(Url::parse("http://www.example.onion/").unwrap());
        let direct = client
            .send(
                onion.clone(),
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Direct,
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(direct.to_string().contains("wrong net"));
        assert!(direct.to_string().contains("onion"));
        let i2p_tab = client
            .send(
                onion,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::I2p {
                    socks: "127.0.0.1:4447".into(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(i2p_tab.to_string().contains("wrong net"));
        let i2p_name = Request::get(Url::parse("http://zzz.i2p/").unwrap());
        let on_tor = client
            .send(
                i2p_name,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Tor {
                    socks: "127.0.0.1:9050".into(),
                },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(on_tor.to_string().contains(".i2p"));
    }

    #[test]
    fn blocker_stops_tracker_host() {
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::builtin();
        let req = Request::get(Url::parse("https://doubleclick.net/").unwrap());
        let err = client
            .send(
                req,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Direct,
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        assert!(err.to_string().contains("blocked"));
    }

    #[test]
    fn classifies_tor_and_block() {
        assert_eq!(
            classify_error(&FrihartError::network("tor refused: no socks")),
            NetFail::Tor
        );
        assert_eq!(
            classify_error(&FrihartError::network("blocked")),
            NetFail::Blocked
        );
    }
}
