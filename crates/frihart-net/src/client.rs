use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use frihart_blocker::FilterEngine;
use frihart_config::ClearnetDns;
use frihart_core::{
    CircuitKind, ContainerId, FrihartError, HiddenNet, Result, hidden_net, sanitize_error,
};
use frihart_privacy::{Policy, ResourceKind};

use crate::cookie::CookieJar;
use crate::dns::{LiveExchange, PlanResolver, SharedClearnetDns};
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
    plan: SharedClearnetDns,
    agent: ureq::Agent,
}

impl Default for RustlsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RustlsClient {
    /// Clearnet agent. Names go to Quad9 DoH. Nothing is sent to the
    /// operating-system resolver.
    pub fn new() -> Self {
        Self::with_plan(ClearnetDns::quad9())
    }

    /// Clearnet agent using `plan`. Tor and I2P fetches build a separate
    /// agent that refuses every name and sends it to SOCKS.
    pub fn with_plan(plan: ClearnetDns) -> Self {
        let plan = SharedClearnetDns::new(plan);
        let resolver = PlanResolver {
            plan: plan.clone(),
            exchange: Arc::new(LiveExchange::new()),
        };
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .redirects(0)
            .user_agent("Frihart")
            .resolver(resolver)
            .build();
        Self { plan, agent }
    }

    pub fn plan(&self) -> ClearnetDns {
        self.plan.get()
    }

    /// Settings changed. Later clearnet lookups use `plan`.
    pub fn set_plan(&self, plan: ClearnetDns) {
        self.plan.set(plan);
    }
}

/// Parse the SOCKS address without DNS. `localhost` maps to 127.0.0.1;
/// any other host must be an IP literal, so looking up the proxy itself
/// never asks a resolver.
fn socks_addr(socks: &str) -> Option<SocketAddr> {
    let (host, port) = socks.rsplit_once(':')?;
    let port: u16 = port.parse().ok().filter(|p| *p != 0)?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let ip = if host.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        host.parse::<IpAddr>().ok()?
    };
    Some(SocketAddr::new(ip, port))
}

/// Resolver for Tor and I2P agents. With a SOCKS5 proxy ureq sends host
/// names to the proxy, so the only lookups it should ask for are IP
/// literals (the proxy address, or a URL that already holds an IP). A
/// name reaching this resolver would be a DNS query outside the circuit;
/// it is refused instead of being passed to the system resolver.
struct NoDnsResolver;

impl ureq::Resolver for NoDnsResolver {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        netloc.parse::<SocketAddr>().map(|a| vec![a]).map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "no local DNS on a hidden circuit",
            )
        })
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
    let addr = socks_addr(socks).ok_or_else(|| {
        FrihartError::network(format!("{kind} refused: socks host must be an IP address"))
    })?;
    let proxy = ureq::Proxy::new(format!("socks5://{addr}"))
        .map_err(|_| FrihartError::network(format!("{kind} refused: socks invalid")))?;
    Ok(ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .redirects(0)
        .user_agent("Frihart")
        .resolver(NoDnsResolver)
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

/// Resolve a redirect `Location` and keep it on the tab's circuit.
///
/// Every hop goes through [`refuse_wrong_net`], not only the first URL: a
/// clearnet page must not bounce a Direct tab to a `.onion` (that name would
/// reach clearnet DNS), and an `.i2p` site must not bounce an I2P tab
/// to clearnet through an outproxy.
pub fn redirect_target(mode: &FetchMode, current: &url::Url, location: &str) -> Result<url::Url> {
    let next = current
        .join(location)
        .map_err(|_| FrihartError::network("redirect"))?;
    refuse_wrong_net(mode, &next)?;
    Ok(next)
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
            apply_identity_headers(&mut request, policy, circuit);
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

            if (300..400).contains(&status)
                && let Some(loc) = headers
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case("location"))
                    .map(|(_, v)| v.clone())
            {
                current = redirect_target(&mode, &current, &loc)?;
                continue;
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
    fn clearnet_client_defaults_to_quad9() {
        let client = RustlsClient::new();
        assert_eq!(client.plan(), frihart_config::ClearnetDns::quad9());
        assert!(!client.plan().allows_system_resolver());
        client.set_plan(frihart_config::ClearnetDns::System);
        assert!(client.plan().allows_system_resolver());
    }

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
    fn socks_address_needs_no_dns() {
        assert_eq!(
            socks_addr("127.0.0.1:9050"),
            Some("127.0.0.1:9050".parse().unwrap())
        );
        assert_eq!(
            socks_addr("localhost:4447"),
            Some("127.0.0.1:4447".parse().unwrap())
        );
        assert_eq!(
            socks_addr("[::1]:9050"),
            Some("[::1]:9050".parse().unwrap())
        );
        assert_eq!(socks_addr("tor.example:9050"), None);
        assert_eq!(socks_addr("127.0.0.1:0"), None);
        assert_eq!(socks_addr("127.0.0.1:x"), None);
        let err = socks5_agent("tor.example:9050", "tor").unwrap_err();
        assert!(err.to_string().contains("IP address"));
    }

    #[test]
    fn hidden_resolver_refuses_names() {
        use ureq::Resolver;
        assert!(NoDnsResolver.resolve("example.com:443").is_err());
        assert!(NoDnsResolver.resolve("www.example.onion:80").is_err());
        assert_eq!(
            NoDnsResolver.resolve("127.0.0.1:9050").unwrap(),
            vec!["127.0.0.1:9050".parse().unwrap()]
        );
        assert!(NoDnsResolver.resolve("[2001:db8::1]:443").is_ok());
    }

    /// Fake SOCKS5 proxy: records the CONNECT address type and host, then
    /// refuses the connection.
    fn fake_socks() -> (String, std::thread::JoinHandle<(u8, Vec<u8>)>) {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let h = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut greet = [0u8; 2];
            s.read_exact(&mut greet).unwrap();
            let mut methods = vec![0u8; greet[1] as usize];
            s.read_exact(&mut methods).unwrap();
            s.write_all(&[5, 0]).unwrap();
            let mut head = [0u8; 4];
            s.read_exact(&mut head).unwrap();
            let host = match head[3] {
                3 => {
                    let mut len = [0u8; 1];
                    s.read_exact(&mut len).unwrap();
                    let mut name = vec![0u8; len[0] as usize];
                    s.read_exact(&mut name).unwrap();
                    name
                }
                1 => {
                    let mut ip = vec![0u8; 4];
                    s.read_exact(&mut ip).unwrap();
                    ip
                }
                _ => Vec::new(),
            };
            // 0x05 = connection refused.
            let _ = s.write_all(&[5, 5, 0, 1, 0, 0, 0, 0, 0, 0]);
            (head[3], host)
        });
        (addr, h)
    }

    #[test]
    fn tor_tab_sends_host_name_to_the_proxy() {
        let (socks, h) = fake_socks();
        let client = RustlsClient::new();
        let policy = Policy::new(Prefs::default());
        let mut jar = CookieJar::default();
        let blocker = FilterEngine::new(true);
        let req = Request::get(Url::parse("https://dns-leak-check.example/").unwrap());
        let err = client
            .send(
                req,
                &policy,
                &mut jar,
                &blocker,
                FetchMode::Tor { socks },
                ContainerId::PERSONAL,
            )
            .unwrap_err();
        let (atyp, host) = h.join().unwrap();
        assert_eq!(atyp, 3, "CONNECT must carry the domain, not a local lookup");
        assert_eq!(host, b"dns-leak-check.example");
        assert!(!err.to_string().is_empty());
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
    fn redirect_hops_stay_on_the_circuit() {
        let tor = FetchMode::Tor {
            socks: "127.0.0.1:9050".into(),
        };
        let i2p = FetchMode::I2p {
            socks: "127.0.0.1:4447".into(),
        };
        let clear = Url::parse("https://example.org/a").unwrap();
        let eepsite = Url::parse("http://zzz.i2p/").unwrap();
        // Direct tab: clearnet hop fine, .onion / .i2p hops refused (no DNS).
        assert!(redirect_target(&FetchMode::Direct, &clear, "/b").is_ok());
        for loc in ["http://www.example.onion/", "http://zzz.i2p/x"] {
            let e = redirect_target(&FetchMode::Direct, &clear, loc).unwrap_err();
            assert!(e.to_string().contains("wrong net"), "{loc}");
        }
        // I2P tab: stays on .i2p; a bounce to clearnet or .onion is refused.
        assert!(redirect_target(&i2p, &eepsite, "/forum").is_ok());
        assert!(redirect_target(&i2p, &eepsite, "https://example.org/").is_err());
        assert!(redirect_target(&i2p, &eepsite, "http://www.example.onion/").is_err());
        // Tor tab: clearnet and .onion fine, .i2p refused.
        assert!(redirect_target(&tor, &clear, "http://www.example.onion/").is_ok());
        assert!(redirect_target(&tor, &clear, "http://zzz.i2p/").is_err());
        // Protocol-relative bounce keeps the check.
        assert!(redirect_target(&FetchMode::Direct, &clear, "//evil.onion/").is_err());
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
