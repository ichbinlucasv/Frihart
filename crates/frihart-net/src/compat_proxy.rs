//! Loopback HTTPS proxy for the Servo Compat engine (Campaign S, M3).
//!
//! Servo is pointed at this proxy for every HTTPS load, so Frihart, not
//! Servo, decides which hosts a Compat tab may reach and does the DNS
//! lookup. Policy, fail closed:
//!
//! - only `CONNECT host:443` is served; plain HTTP is refused (Compat tabs
//!   are HTTPS-only, and TLS stays end to end between Servo and the site);
//! - `.onion`, `.i2p` and other special-use names are refused: Compat is
//!   clearnet-only and Servo never serves Tor or I2P;
//! - IP literals and names that resolve to loopback, link-local or private
//!   addresses are refused (no LAN probing, no DNS rebinding);
//! - hosts on the native blocker lists are refused;
//! - the request head is capped at 8 KiB and must arrive within 10 s.
//!
//! What it cannot do yet: rewrite headers or strip cookies inside the TLS
//! tunnel (Servo's own policy and prefs govern those), and Tor routing for
//! Compat tabs (out of scope by design).

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use frihart_blocker::FilterEngine;
#[cfg(test)]
use frihart_config::ClearnetDns;

use crate::addr::{HostClass, classify_host, classify_ip};
use crate::dns::{DnsError, DnsExchange, LiveExchange, SharedClearnetDns, resolve_clearnet};

/// Largest request head accepted, in bytes.
pub const MAX_HEAD_BYTES: usize = 8 * 1024;
const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Idle limit on an open tunnel.
const TUNNEL_IDLE: Duration = Duration::from_secs(120);

/// A parsed request head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProxyRequest {
    Connect {
        host: String,
        port: u16,
    },
    /// Any other method (absolute-form plain HTTP, etc.).
    Other {
        method: String,
    },
}

/// What the proxy does with a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProxyVerdict {
    Allow { host: String, port: u16 },
    Refuse(&'static str),
}

/// Parse `CONNECT host:port HTTP/1.x` (the head up to the blank line).
pub fn parse_request_head(head: &[u8]) -> Result<ProxyRequest, &'static str> {
    let text = std::str::from_utf8(head).map_err(|_| "request head is not UTF-8")?;
    let line = text.lines().next().ok_or("empty request")?;
    let mut parts = line.split(' ');
    let method = parts.next().unwrap_or_default();
    let target = parts.next().ok_or("no request target")?;
    let version = parts.next().ok_or("no HTTP version")?;
    if parts.next().is_some() || !version.starts_with("HTTP/1.") {
        return Err("malformed request line");
    }
    if method != "CONNECT" {
        return Ok(ProxyRequest::Other {
            method: method.to_string(),
        });
    }
    let (host, port) = target
        .rsplit_once(':')
        .ok_or("CONNECT target has no port")?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let port: u16 = port.parse().map_err(|_| "bad CONNECT port")?;
    if host.is_empty() || host.len() > 253 {
        return Err("bad CONNECT host");
    }
    Ok(ProxyRequest::Connect {
        host: host.to_ascii_lowercase(),
        port,
    })
}

fn special_use(host: &str) -> bool {
    let h = host.trim_end_matches('.');
    [
        ".onion",
        ".i2p",
        ".local",
        ".localhost",
        ".internal",
        ".home.arpa",
        ".invalid",
    ]
    .iter()
    .any(|s| h.ends_with(s) || h == &s[1..])
}

/// Proxy policy for Compat tabs.
#[derive(Clone)]
pub struct CompatPolicy {
    blocker: FilterEngine,
    allow_private: bool,
    dns: SharedClearnetDns,
    exchange: Arc<dyn DnsExchange>,
}

impl CompatPolicy {
    /// Quad9 DoH, no system resolver, unless `dns` is later updated.
    pub fn new(blocker: FilterEngine) -> Self {
        Self::sharing_dns(blocker, SharedClearnetDns::quad9())
    }

    /// Compat proxy that follows `dns` (settings can change it in place).
    pub fn sharing_dns(blocker: FilterEngine, dns: SharedClearnetDns) -> Self {
        Self {
            blocker,
            allow_private: false,
            dns,
            exchange: Arc::new(LiveExchange::new()),
        }
    }

    /// Test-only: let tunnels reach loopback so a local echo server can
    /// stand in for a site.
    #[cfg(test)]
    fn allowing_loopback(blocker: FilterEngine) -> Self {
        Self {
            blocker,
            allow_private: true,
            dns: SharedClearnetDns::quad9(),
            exchange: Arc::new(LiveExchange::new()),
        }
    }

    #[cfg(test)]
    fn with_exchange(
        blocker: FilterEngine,
        dns: ClearnetDns,
        exchange: Arc<dyn DnsExchange>,
    ) -> Self {
        Self {
            blocker,
            allow_private: false,
            dns: SharedClearnetDns::new(dns),
            exchange,
        }
    }

    /// Decide on a parsed request before any DNS lookup.
    pub fn decide(&self, req: &ProxyRequest) -> ProxyVerdict {
        let (host, port) = match req {
            ProxyRequest::Other { .. } => {
                return ProxyVerdict::Refuse("only HTTPS (CONNECT) is allowed on Compat tabs");
            }
            ProxyRequest::Connect { host, port } => (host, *port),
        };
        if port != 443 && !self.allow_private {
            return ProxyVerdict::Refuse("only port 443 is allowed on Compat tabs");
        }
        if special_use(host) {
            return ProxyVerdict::Refuse("special-use name (.onion/.i2p/local) refused on Compat");
        }
        match classify_host(host) {
            HostClass::Public | HostClass::Name => {}
            _ if self.allow_private => {}
            _ => return ProxyVerdict::Refuse("private or loopback address refused"),
        }
        if let Ok(url) = url::Url::parse(&format!("https://{host}/")) {
            if self.blocker.decide(&url).blocked() {
                return ProxyVerdict::Refuse("host is on a blocker list");
            }
        } else {
            return ProxyVerdict::Refuse("host is not a valid URL host");
        }
        ProxyVerdict::Allow {
            host: host.clone(),
            port,
        }
    }

    /// Resolve through the clearnet plan. `.onion` and `.i2p` never reach it.
    /// A private answer is dropped (DNS rebinding). The system resolver runs
    /// only when the plan says the user opted in.
    pub fn resolve(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, &'static str> {
        let plan = self.dns.get();
        let addrs =
            resolve_clearnet(&plan, host, port, self.exchange.as_ref()).map_err(
                |err| match err {
                    DnsError::Hidden => "hidden name refused before DNS",
                    DnsError::Private => "name resolves to a private or loopback address",
                    DnsError::Empty => "DNS lookup returned nothing",
                    DnsError::SpecialUse => "special-use name refused before DNS",
                    DnsError::BadName
                    | DnsError::BadUrl
                    | DnsError::Refused
                    | DnsError::Transport => "DNS lookup failed",
                    #[cfg(test)]
                    DnsError::NoLocalDns => "DNS lookup failed",
                },
            )?;
        if !self.allow_private
            && addrs
                .iter()
                .any(|a| classify_ip(a.ip()) != HostClass::Public)
        {
            return Err("name resolves to a private or loopback address");
        }
        Ok(addrs)
    }
}

/// Counters a test or `about:` page can read.
#[derive(Default, Debug)]
pub struct ProxyStats {
    pub allowed: AtomicU64,
    pub refused: AtomicU64,
}

/// A running proxy on `127.0.0.1:<port>`. Threads stop when the process
/// exits; there is no global state.
pub struct CompatProxy {
    addr: SocketAddr,
    stats: Arc<ProxyStats>,
}

impl CompatProxy {
    /// Bind an ephemeral loopback port and serve in a background thread.
    pub fn spawn(policy: CompatPolicy) -> io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let stats = Arc::new(ProxyStats::default());
        let st = stats.clone();
        let policy = Arc::new(policy);
        thread::Builder::new()
            .name("frihart-compat-proxy".into())
            .spawn(move || {
                for conn in listener.incoming().flatten() {
                    let policy = policy.clone();
                    let st = st.clone();
                    let _ = thread::Builder::new()
                        .name("frihart-compat-conn".into())
                        .spawn(move || handle(conn, &policy, &st));
                }
            })?;
        Ok(Self { addr, stats })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// `http://127.0.0.1:<port>` for Servo's proxy preference.
    pub fn uri(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn stats(&self) -> &ProxyStats {
        &self.stats
    }
}

fn read_head(conn: &mut TcpStream) -> io::Result<Vec<u8>> {
    conn.set_read_timeout(Some(HEAD_TIMEOUT))?;
    let mut head = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    // Byte-wise so nothing after the head (the client's TLS hello) is eaten.
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= MAX_HEAD_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "head too large"));
        }
        if conn.read(&mut byte)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "closed"));
        }
        head.push(byte[0]);
    }
    Ok(head)
}

fn refuse(conn: &mut TcpStream, stats: &ProxyStats) {
    stats.refused.fetch_add(1, Ordering::Relaxed);
    let _ =
        conn.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let _ = conn.shutdown(Shutdown::Both);
}

fn handle(mut conn: TcpStream, policy: &CompatPolicy, stats: &ProxyStats) {
    let Ok(head) = read_head(&mut conn) else {
        return refuse(&mut conn, stats);
    };
    let verdict = match parse_request_head(&head) {
        Ok(req) => policy.decide(&req),
        Err(_) => ProxyVerdict::Refuse("malformed request"),
    };
    let ProxyVerdict::Allow { host, port } = verdict else {
        return refuse(&mut conn, stats);
    };
    let Ok(addrs) = policy.resolve(&host, port) else {
        return refuse(&mut conn, stats);
    };
    let Some(upstream) = addrs
        .iter()
        .find_map(|a| TcpStream::connect_timeout(a, CONNECT_TIMEOUT).ok())
    else {
        let _ = conn.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
        return;
    };
    stats.allowed.fetch_add(1, Ordering::Relaxed);
    if conn
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .is_err()
    {
        return;
    }
    let _ = conn.set_read_timeout(Some(TUNNEL_IDLE));
    let _ = upstream.set_read_timeout(Some(TUNNEL_IDLE));
    let (Ok(mut c_read), Ok(mut u_write)) = (conn.try_clone(), upstream.try_clone()) else {
        return;
    };
    let up = thread::spawn(move || {
        let _ = io::copy(&mut c_read, &mut u_write);
        let _ = u_write.shutdown(Shutdown::Write);
    });
    let mut u_read = upstream;
    let _ = io::copy(&mut u_read, &mut conn);
    let _ = conn.shutdown(Shutdown::Write);
    let _ = up.join();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> CompatPolicy {
        let mut b = FilterEngine::new(true);
        b.add_host("tracker.example");
        CompatPolicy::new(b)
    }

    fn connect(host: &str, port: u16) -> ProxyRequest {
        ProxyRequest::Connect {
            host: host.into(),
            port,
        }
    }

    #[test]
    fn parses_connect_and_rejects_junk() {
        assert_eq!(
            parse_request_head(b"CONNECT Example.org:443 HTTP/1.1\r\nHost: x\r\n\r\n"),
            Ok(connect("example.org", 443))
        );
        assert_eq!(
            parse_request_head(b"CONNECT [2001:db8::1]:443 HTTP/1.1\r\n\r\n"),
            Ok(connect("2001:db8::1", 443))
        );
        assert!(matches!(
            parse_request_head(b"GET http://example.org/ HTTP/1.1\r\n\r\n"),
            Ok(ProxyRequest::Other { .. })
        ));
        for bad in [
            &b""[..],
            b"CONNECT example.org HTTP/1.1\r\n\r\n",
            b"CONNECT example.org:99999 HTTP/1.1\r\n\r\n",
            b"CONNECT example.org:443 SPDY/3\r\n\r\n",
            b"CONNECT :443 HTTP/1.1\r\n\r\n",
            b"\xff\xfe",
        ] {
            assert!(parse_request_head(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn policy_is_fail_closed() {
        let p = policy();
        assert!(matches!(
            p.decide(&connect("example.org", 443)),
            ProxyVerdict::Allow { .. }
        ));
        for (host, port) in [
            ("example.org", 80),
            ("example.org", 8443),
            (
                "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion",
                443,
            ),
            ("stats.i2p", 443),
            ("printer.local", 443),
            ("localhost", 443),
            ("127.0.0.1", 443),
            ("10.0.0.8", 443),
            ("192.168.1.1", 443),
            ("::1", 443),
            ("fd00::1", 443),
            ("169.254.169.254", 443),
            ("tracker.example", 443),
            ("cdn.tracker.example", 443),
        ] {
            assert!(
                matches!(p.decide(&connect(host, port)), ProxyVerdict::Refuse(_)),
                "{host}:{port} should be refused"
            );
        }
        assert!(matches!(
            p.decide(&ProxyRequest::Other {
                method: "GET".into()
            }),
            ProxyVerdict::Refuse(_)
        ));
    }

    #[test]
    fn compat_resolve_uses_quad9_and_skips_hidden_names() {
        use std::net::{IpAddr, Ipv4Addr};
        use std::sync::Arc;

        use frihart_config::{ClearnetDns, QUAD9_DOH_URL};

        use crate::dns::ScriptedExchange;

        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))]);
        let policy = CompatPolicy::with_exchange(
            FilterEngine::new(true),
            ClearnetDns::quad9(),
            Arc::new(ex.clone()),
        );
        let addrs = policy.resolve("example.org", 443).unwrap();
        assert_eq!(addrs, vec!["93.184.216.34:443".parse().unwrap()]);
        assert!(ex.doh_calls().iter().all(|url| url == QUAD9_DOH_URL));
        assert!(ex.system_calls().is_empty());

        let hidden = ScriptedExchange::doh_fails();
        let policy = CompatPolicy::with_exchange(
            FilterEngine::new(true),
            ClearnetDns::quad9(),
            Arc::new(hidden.clone()),
        );
        assert_eq!(
            policy.resolve("www.example.onion", 443).unwrap_err(),
            "hidden name refused before DNS"
        );
        assert_eq!(
            policy.resolve("zzz.i2p", 443).unwrap_err(),
            "hidden name refused before DNS"
        );
        assert!(hidden.doh_calls().is_empty());
        assert!(hidden.system_calls().is_empty());

        let fallback = ScriptedExchange::doh_fails();
        let mut opted = ClearnetDns::quad9();
        let ClearnetDns::Doh {
            system_fallback, ..
        } = &mut opted
        else {
            panic!("quad9 is DoH");
        };
        *system_fallback = true;
        let policy =
            CompatPolicy::with_exchange(FilterEngine::new(true), opted, Arc::new(fallback.clone()));
        assert!(policy.resolve("example.org", 443).is_ok());
        assert_eq!(fallback.system_calls(), vec!["example.org".to_string()]);

        let no_fallback = ScriptedExchange::doh_fails();
        let policy = CompatPolicy::with_exchange(
            FilterEngine::new(true),
            ClearnetDns::quad9(),
            Arc::new(no_fallback.clone()),
        );
        assert!(policy.resolve("example.org", 443).is_err());
        assert!(no_fallback.system_calls().is_empty());
    }

    fn ask(proxy: &CompatProxy, head: &[u8]) -> String {
        let mut s = TcpStream::connect(proxy.addr()).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let _ = s.write_all(head);
        let mut buf = [0u8; 64];
        let n = s.read(&mut buf).unwrap_or(0);
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }

    #[test]
    fn live_proxy_refuses_with_403() {
        let proxy = CompatProxy::spawn(policy()).unwrap();
        assert!(proxy.uri().starts_with("http://127.0.0.1:"));
        for head in [
            &b"CONNECT 127.0.0.1:443 HTTP/1.1\r\n\r\n"[..],
            b"CONNECT abc.onion:443 HTTP/1.1\r\n\r\n",
            b"GET http://example.org/ HTTP/1.1\r\n\r\n",
            b"garbage\r\n\r\n",
        ] {
            assert!(ask(&proxy, head).starts_with("HTTP/1.1 403"), "{head:?}");
        }
        assert_eq!(proxy.stats().refused.load(Ordering::Relaxed), 4);
        assert_eq!(proxy.stats().allowed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn oversized_head_is_refused() {
        let proxy = CompatProxy::spawn(policy()).unwrap();
        let mut head = b"CONNECT example.org:443 HTTP/1.1\r\nX: ".to_vec();
        head.extend(std::iter::repeat_n(b'a', MAX_HEAD_BYTES));
        head.extend_from_slice(b"\r\n\r\n");
        // The proxy may reset the socket before the client reads the 403.
        let reply = ask(&proxy, &head);
        assert!(
            reply.is_empty() || reply.starts_with("HTTP/1.1 403"),
            "{reply}"
        );
        for _ in 0..200 {
            if proxy.stats().refused.load(Ordering::Relaxed) == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(proxy.stats().refused.load(Ordering::Relaxed), 1);
        assert_eq!(proxy.stats().allowed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn tunnel_relays_bytes_both_ways() {
        let echo = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = echo.local_addr().unwrap().port();
        thread::spawn(move || {
            let (mut s, _) = echo.accept().unwrap();
            let mut b = [0u8; 5];
            s.read_exact(&mut b).unwrap();
            s.write_all(&b.map(|c| c.to_ascii_uppercase())).unwrap();
        });
        let proxy =
            CompatProxy::spawn(CompatPolicy::allowing_loopback(FilterEngine::new(true))).unwrap();
        let mut s = TcpStream::connect(proxy.addr()).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        // Head and the first tunnel bytes in one write: none may be lost.
        s.write_all(format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\n\r\nhello").as_bytes())
            .unwrap();
        let mut got = Vec::new();
        let mut buf = [0u8; 128];
        while !got.ends_with(b"HELLO") {
            let n = s.read(&mut buf).unwrap();
            assert!(
                n > 0,
                "tunnel closed early: {:?}",
                String::from_utf8_lossy(&got)
            );
            got.extend_from_slice(&buf[..n]);
        }
        assert!(got.starts_with(b"HTTP/1.1 200"));
        assert_eq!(proxy.stats().allowed.load(Ordering::Relaxed), 1);
    }
}
