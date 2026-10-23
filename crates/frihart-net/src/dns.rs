//! Clearnet DNS. Quad9 over HTTPS by default. No system resolver unless
//! the user opts in.
//!
//! Tor and I2P tabs do not use this module for names: those agents refuse
//! every lookup and send the name to SOCKS. `.onion` and `.i2p` are refused
//! here as well, before any query, so a clearnet path cannot leak them.
//!
//! The Quad9 connection is pinned to Quad9's published addresses. The name
//! `dns.quad9.net` is not resolved through the operating system.

use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub use frihart_config::ClearnetDns;
use frihart_config::QUAD9_DOH_URL;

use crate::{HostClass, classify_ip};

const QTYPE_A: u16 = 1;
const QTYPE_AAAA: u16 = 28;
const DNS_TIMEOUT: Duration = Duration::from_secs(5);
/// DNS over TCP/HTTPS is at most 64 KiB.
const MAX_DNS_MESSAGE: u64 = 65_535;

/// Published Quad9 addresses for `dns.quad9.net` (filtered, DNSSEC).
/// IPv4 first so a host without working IPv6 does not wait on it.
const QUAD9_V4: [Ipv4Addr; 2] = [Ipv4Addr::new(9, 9, 9, 9), Ipv4Addr::new(149, 112, 112, 112)];
const QUAD9_V6: [Ipv6Addr; 2] = [
    Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x00fe),
    Ipv6Addr::new(0x2620, 0x00fe, 0, 0, 0, 0, 0, 0x0009),
];

/// Where a tab is allowed to resolve. Production Tor and I2P agents use
/// their own resolver that refuses names; this scope is the same rule for
/// tests that share the clearnet exchange.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DnsScope {
    /// Servo Compat and clearnet own-engine tabs.
    Clearnet,
    /// Tor or I2P. No local lookup of any name.
    HiddenCircuit,
}

/// Why a lookup stopped. Text is static so a name never lands in a log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DnsError {
    Hidden,
    /// Tor or I2P scope: the clearnet exchange must not be called.
    #[cfg(test)]
    NoLocalDns,
    SpecialUse,
    BadName,
    BadUrl,
    Empty,
    Private,
    Refused,
    Transport,
}

impl DnsError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hidden => "hidden name is not sent to a resolver",
            #[cfg(test)]
            Self::NoLocalDns => "no local DNS on a hidden circuit",
            Self::SpecialUse => "special-use name is not sent to clearnet DNS",
            Self::BadName => "name is not a DNS name",
            Self::BadUrl => "DoH URL must be https",
            Self::Empty => "resolver returned no address",
            Self::Private => "name resolves to a private or loopback address",
            Self::Refused => "resolver refused the query",
            Self::Transport => "encrypted DNS failed",
        }
    }
}

/// Shared plan. The clearnet client and the Compat proxy read the same one
/// when the user changes settings.
#[derive(Clone, Debug)]
pub struct SharedClearnetDns {
    inner: Arc<RwLock<ClearnetDns>>,
}

impl SharedClearnetDns {
    pub fn new(plan: ClearnetDns) -> Self {
        Self {
            inner: Arc::new(RwLock::new(plan)),
        }
    }

    pub fn quad9() -> Self {
        Self::new(ClearnetDns::quad9())
    }

    pub fn get(&self) -> ClearnetDns {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set(&self, plan: ClearnetDns) {
        *self.inner.write().unwrap_or_else(|e| e.into_inner()) = plan;
    }
}

/// What a lookup may call. The system method runs only after an explicit opt-in.
pub(crate) trait DnsExchange: Send + Sync {
    fn doh(&self, url: &str, query: &[u8]) -> Result<Vec<u8>, ()>;
    fn system(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, ()>;
}

/// Resolve `host` for a tab. Hidden circuits never call `exchange`.
#[cfg(test)]
pub(crate) fn resolve_in_scope(
    scope: DnsScope,
    plan: &ClearnetDns,
    host: &str,
    port: u16,
    exchange: &dyn DnsExchange,
) -> Result<Vec<SocketAddr>, DnsError> {
    match scope {
        DnsScope::HiddenCircuit => Err(DnsError::NoLocalDns),
        DnsScope::Clearnet => resolve_clearnet(plan, host, port, exchange),
    }
}

pub(crate) fn resolve_clearnet(
    plan: &ClearnetDns,
    host: &str,
    port: u16,
    exchange: &dyn DnsExchange,
) -> Result<Vec<SocketAddr>, DnsError> {
    if port == 0 {
        return Err(DnsError::BadName);
    }
    if is_hidden_name(host) {
        return Err(DnsError::Hidden);
    }
    if let Some(ip) = parse_ip(host) {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }
    if is_localhost(host) {
        return Ok(vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
        ]);
    }
    let ascii = ascii_dns_name(host)?;
    if is_hidden_name(&ascii) {
        return Err(DnsError::Hidden);
    }
    match plan {
        ClearnetDns::System => exchange
            .system(&ascii, port)
            .map_err(|_| DnsError::Transport),
        ClearnetDns::Doh {
            url,
            system_fallback,
        } => resolve_doh(url, *system_fallback, &ascii, port, exchange),
    }
}

fn resolve_doh(
    url: &str,
    system_fallback: bool,
    name: &str,
    port: u16,
    exchange: &dyn DnsExchange,
) -> Result<Vec<SocketAddr>, DnsError> {
    if is_special_use(name) {
        return Err(DnsError::SpecialUse);
    }
    if !usable_doh_url(url) {
        return if system_fallback {
            exchange.system(name, port).map_err(|_| DnsError::Transport)
        } else {
            Err(DnsError::BadUrl)
        };
    }
    match doh_addresses(exchange, url, name) {
        Ok(ips) => public_sockets(&ips, port),
        Err(DnsError::Transport) if system_fallback => {
            exchange.system(name, port).map_err(|_| DnsError::Transport)
        }
        Err(other) => Err(other),
    }
}

fn public_sockets(ips: &[IpAddr], port: u16) -> Result<Vec<SocketAddr>, DnsError> {
    let mut public = Vec::new();
    for ip in ips {
        if classify_ip(*ip) != HostClass::Public {
            continue;
        }
        let addr = SocketAddr::new(*ip, port);
        if !public.contains(&addr) {
            public.push(addr);
        }
    }
    if !public.is_empty() {
        return Ok(public);
    }
    if ips.is_empty() {
        Err(DnsError::Empty)
    } else {
        Err(DnsError::Private)
    }
}

fn doh_addresses(
    exchange: &dyn DnsExchange,
    url: &str,
    name: &str,
) -> Result<Vec<IpAddr>, DnsError> {
    let mut ips = Vec::new();
    let mut saw_reply = false;
    let mut saw_refused = false;
    for qtype in [QTYPE_A, QTYPE_AAAA] {
        match one_query(exchange, url, name, qtype) {
            Ok(found) => {
                saw_reply = true;
                ips.extend(found);
            }
            Err(DnsError::Refused) => saw_refused = true,
            Err(DnsError::Transport) => {}
            Err(other) => return Err(other),
        }
    }
    if saw_reply {
        Ok(ips)
    } else if saw_refused {
        Err(DnsError::Refused)
    } else {
        Err(DnsError::Transport)
    }
}

fn one_query(
    exchange: &dyn DnsExchange,
    url: &str,
    name: &str,
    qtype: u16,
) -> Result<Vec<IpAddr>, DnsError> {
    let id = random_id()?;
    let query = encode_query(name, qtype, id)?;
    let body = exchange.doh(url, &query).map_err(|_| DnsError::Transport)?;
    parse_answers(&body, id)
}

fn random_id() -> Result<u16, DnsError> {
    let mut buf = [0u8; 2];
    getrandom::getrandom(&mut buf).map_err(|_| DnsError::Transport)?;
    Ok(u16::from_be_bytes(buf))
}

fn encode_query(name: &str, qtype: u16, id: u16) -> Result<Vec<u8>, DnsError> {
    let mut msg = Vec::with_capacity(name.len() + 18);
    msg.extend_from_slice(&id.to_be_bytes());
    msg.extend_from_slice(&0x0100u16.to_be_bytes());
    msg.extend_from_slice(&1u16.to_be_bytes());
    msg.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(DnsError::BadName);
        }
        msg.push(u8::try_from(label.len()).map_err(|_| DnsError::BadName)?);
        msg.extend_from_slice(label.as_bytes());
    }
    msg.push(0);
    msg.extend_from_slice(&qtype.to_be_bytes());
    msg.extend_from_slice(&1u16.to_be_bytes());
    Ok(msg)
}

fn parse_answers(msg: &[u8], expected_id: u16) -> Result<Vec<IpAddr>, DnsError> {
    if msg.len() < 12 {
        return Err(DnsError::Transport);
    }
    let id = u16::from_be_bytes([msg[0], msg[1]]);
    if id != expected_id {
        return Err(DnsError::Transport);
    }
    let flags = u16::from_be_bytes([msg[2], msg[3]]);
    if flags & 0x8000 == 0 || flags & 0x0200 != 0 {
        return Err(DnsError::Transport);
    }
    let rcode = flags & 0x000F;
    if rcode == 3 {
        return Ok(Vec::new());
    }
    if rcode != 0 {
        return Err(DnsError::Refused);
    }
    let questions = u16::from_be_bytes([msg[4], msg[5]]) as usize;
    let answers = u16::from_be_bytes([msg[6], msg[7]]) as usize;
    let mut pos = 12;
    for _ in 0..questions {
        pos = skip_name(msg, pos)?;
        pos = pos.checked_add(4).ok_or(DnsError::Transport)?;
        if pos > msg.len() {
            return Err(DnsError::Transport);
        }
    }
    let mut ips = Vec::new();
    for _ in 0..answers {
        pos = skip_name(msg, pos)?;
        if pos + 10 > msg.len() {
            return Err(DnsError::Transport);
        }
        let typ = u16::from_be_bytes([msg[pos], msg[pos + 1]]);
        let class = u16::from_be_bytes([msg[pos + 2], msg[pos + 3]]);
        let rdlen = u16::from_be_bytes([msg[pos + 8], msg[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlen > msg.len() {
            return Err(DnsError::Transport);
        }
        if class == 1 && typ == QTYPE_A && rdlen == 4 {
            ips.push(IpAddr::V4(Ipv4Addr::new(
                msg[pos],
                msg[pos + 1],
                msg[pos + 2],
                msg[pos + 3],
            )));
        } else if class == 1 && typ == QTYPE_AAAA && rdlen == 16 {
            let mut oct = [0u8; 16];
            oct.copy_from_slice(&msg[pos..pos + 16]);
            ips.push(IpAddr::V6(Ipv6Addr::from(oct)));
        }
        pos += rdlen;
    }
    Ok(ips)
}

fn skip_name(msg: &[u8], mut pos: usize) -> Result<usize, DnsError> {
    let mut hops = 0;
    loop {
        if pos >= msg.len() || hops > 128 {
            return Err(DnsError::Transport);
        }
        let len = msg[pos];
        if len == 0 {
            return Ok(pos + 1);
        }
        if len & 0xC0 == 0xC0 {
            if pos + 1 >= msg.len() {
                return Err(DnsError::Transport);
            }
            return Ok(pos + 2);
        }
        if len & 0xC0 != 0 {
            return Err(DnsError::Transport);
        }
        let next = pos
            .checked_add(1)
            .and_then(|p| p.checked_add(len as usize))
            .ok_or(DnsError::Transport)?;
        if next > msg.len() {
            return Err(DnsError::Transport);
        }
        pos = next;
        hops += 1;
    }
}

fn is_hidden_name(host: &str) -> bool {
    frihart_core::hidden_net_host(host).is_some()
}

fn is_localhost(host: &str) -> bool {
    host.trim_end_matches('.').eq_ignore_ascii_case("localhost")
}

fn is_special_use(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    [
        ".local",
        ".localhost",
        ".internal",
        ".home.arpa",
        ".invalid",
    ]
    .iter()
    .any(|suffix| h.ends_with(suffix) || h == suffix[1..])
}

fn parse_ip(host: &str) -> Option<IpAddr> {
    host.trim()
        .trim_matches(|c| c == '[' || c == ']')
        .parse()
        .ok()
}

fn ascii_dns_name(host: &str) -> Result<String, DnsError> {
    let trimmed = host.trim().trim_end_matches('.').trim();
    if trimmed.is_empty() || trimmed.len() > 253 {
        return Err(DnsError::BadName);
    }
    let ascii = if trimmed.is_ascii() {
        trimmed.to_ascii_lowercase()
    } else {
        let converted = idna::domain_to_ascii(trimmed).map_err(|_| DnsError::BadName)?;
        if !converted.is_ascii() || converted.is_empty() || converted.len() > 253 {
            return Err(DnsError::BadName);
        }
        converted.to_ascii_lowercase()
    };
    for label in ascii.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(DnsError::BadName);
        }
        let ok = label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if !ok || label.starts_with('-') || label.ends_with('-') {
            return Err(DnsError::BadName);
        }
    }
    Ok(ascii)
}

fn usable_doh_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }
    matches!(
        parsed.host_str(),
        Some(host) if !host.is_empty() && !is_hidden_name(host) && parse_ip(host).is_none()
    )
}

fn is_quad9_host(host: &str) -> bool {
    host.trim_end_matches('.')
        .eq_ignore_ascii_case("dns.quad9.net")
}

pub(crate) fn quad9_bootstrap(port: u16) -> Vec<SocketAddr> {
    QUAD9_V4
        .into_iter()
        .map(IpAddr::V4)
        .chain(QUAD9_V6.into_iter().map(IpAddr::V6))
        .map(|ip| SocketAddr::new(ip, port))
        .collect()
}

/// ureq resolver for the clearnet agent. Reads the plan on every lookup.
pub(crate) struct PlanResolver {
    pub plan: SharedClearnetDns,
    pub exchange: Arc<dyn DnsExchange>,
}

impl ureq::Resolver for PlanResolver {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        let (host, port) = split_netloc(netloc)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bad address"))?;
        resolve_clearnet(&self.plan.get(), host, port, self.exchange.as_ref()).map_err(|e| {
            let kind = match e {
                DnsError::Hidden | DnsError::SpecialUse | DnsError::Private => {
                    io::ErrorKind::PermissionDenied
                }
                #[cfg(test)]
                DnsError::NoLocalDns => io::ErrorKind::PermissionDenied,
                DnsError::BadName | DnsError::BadUrl => io::ErrorKind::InvalidInput,
                DnsError::Empty | DnsError::Refused | DnsError::Transport => io::ErrorKind::Other,
            };
            io::Error::new(kind, e.as_str())
        })
    }
}

pub(crate) fn split_netloc(netloc: &str) -> Option<(&str, u16)> {
    if let Some(rest) = netloc.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        let port = port.parse().ok()?;
        return Some((host, port));
    }
    let (host, port) = netloc.rsplit_once(':')?;
    let port = port.parse().ok()?;
    Some((host, port))
}

struct PinnedQuad9;

impl ureq::Resolver for PinnedQuad9 {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        let (host, port) = split_netloc(netloc)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bad DoH address"))?;
        if !is_quad9_host(host) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "DoH bootstrap is pinned to Quad9",
            ));
        }
        Ok(quad9_bootstrap(port))
    }
}

struct OneHost {
    host: String,
    addrs: Vec<IpAddr>,
}

impl ureq::Resolver for OneHost {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        let (host, port) = split_netloc(netloc)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "bad DoH address"))?;
        if !host.eq_ignore_ascii_case(&self.host) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "resolver is pinned to the DoH host",
            ));
        }
        if self.addrs.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "DoH host has no address",
            ));
        }
        Ok(self
            .addrs
            .iter()
            .copied()
            .map(|ip| SocketAddr::new(ip, port))
            .collect())
    }
}

fn dns_agent(resolver: impl ureq::Resolver + 'static) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(DNS_TIMEOUT)
        .timeout_connect(DNS_TIMEOUT)
        .redirects(0)
        .user_agent(frihart_core::FROZEN_USER_AGENT)
        .resolver(resolver)
        .build()
}

fn post_dns(agent: &ureq::Agent, url: &str, query: &[u8]) -> Result<Vec<u8>, ()> {
    let response = agent
        .post(url)
        .set("Content-Type", "application/dns-message")
        .set("Accept", "application/dns-message")
        .send_bytes(query)
        .map_err(|_| ())?;
    if response.status() != 200 {
        return Err(());
    }
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_DNS_MESSAGE)
        .read_to_end(&mut body)
        .map_err(|_| ())?;
    if body.len() < 12 { Err(()) } else { Ok(body) }
}

fn lookup_public_via_quad9(agent: &ureq::Agent, host: &str) -> Result<Vec<IpAddr>, ()> {
    let name = ascii_dns_name(host).map_err(|_| ())?;
    if is_hidden_name(&name) || is_special_use(&name) {
        return Err(());
    }
    let mut ips = Vec::new();
    for qtype in [QTYPE_A, QTYPE_AAAA] {
        let Ok(id) = random_id() else {
            continue;
        };
        let Ok(query) = encode_query(&name, qtype, id) else {
            continue;
        };
        let Ok(body) = post_dns(agent, QUAD9_DOH_URL, &query) else {
            continue;
        };
        if let Ok(found) = parse_answers(&body, id) {
            ips.extend(
                found
                    .into_iter()
                    .filter(|ip| classify_ip(*ip) == HostClass::Public),
            );
        }
    }
    if ips.is_empty() { Err(()) } else { Ok(ips) }
}

/// Production transport. Quad9 is reached on pinned addresses. Any other
/// DoH host is resolved through that pinned Quad9 path, never the OS,
/// unless [`DnsExchange::system`] is called by the policy.
pub(crate) struct LiveExchange {
    quad9: ureq::Agent,
}

impl LiveExchange {
    pub(crate) fn new() -> Self {
        Self {
            quad9: dns_agent(PinnedQuad9),
        }
    }
}

impl DnsExchange for LiveExchange {
    fn doh(&self, url: &str, query: &[u8]) -> Result<Vec<u8>, ()> {
        let host = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .ok_or(())?;
        if is_quad9_host(&host) {
            return post_dns(&self.quad9, url, query);
        }
        let ips = lookup_public_via_quad9(&self.quad9, &host)?;
        let agent = dns_agent(OneHost {
            host: host.trim_end_matches('.').to_ascii_lowercase(),
            addrs: ips,
        });
        post_dns(&agent, url, query)
    }

    fn system(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, ()> {
        if is_hidden_name(host) {
            return Err(());
        }
        let mut addrs = Vec::new();
        for addr in (host, port).to_socket_addrs().map_err(|_| ())? {
            addrs.push(addr);
        }
        if addrs.is_empty() { Err(()) } else { Ok(addrs) }
    }
}

/// Test double. Records every DoH URL and every system host.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct ScriptedExchange {
    inner: Arc<ScriptInner>,
}

#[cfg(test)]
struct ScriptInner {
    doh: Mutex<Vec<String>>,
    system: Mutex<Vec<String>>,
    fail: Mutex<bool>,
    rcode: Mutex<Option<u16>>,
    ips: Mutex<Vec<IpAddr>>,
    system_fail: Mutex<bool>,
    system_addrs: Mutex<Vec<SocketAddr>>,
}

#[cfg(test)]
impl ScriptedExchange {
    fn build(fail: bool, rcode: Option<u16>, ips: Vec<IpAddr>) -> Self {
        Self {
            inner: Arc::new(ScriptInner {
                doh: Mutex::new(Vec::new()),
                system: Mutex::new(Vec::new()),
                fail: Mutex::new(fail),
                rcode: Mutex::new(rcode),
                ips: Mutex::new(ips),
                system_fail: Mutex::new(false),
                system_addrs: Mutex::new(vec![SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
                    0,
                )]),
            }),
        }
    }

    pub(crate) fn answering(ips: Vec<IpAddr>) -> Self {
        Self::build(false, None, ips)
    }

    pub(crate) fn doh_fails() -> Self {
        Self::build(true, None, Vec::new())
    }

    pub(crate) fn nxdomain() -> Self {
        Self::build(false, Some(3), Vec::new())
    }

    pub(crate) fn refused() -> Self {
        Self::build(false, Some(5), Vec::new())
    }

    pub(crate) fn doh_calls(&self) -> Vec<String> {
        self.inner
            .doh
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(crate) fn system_calls(&self) -> Vec<String> {
        self.inner
            .system
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

#[cfg(test)]
impl DnsExchange for ScriptedExchange {
    fn doh(&self, url: &str, query: &[u8]) -> Result<Vec<u8>, ()> {
        self.inner
            .doh
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(url.to_string());
        if *self.inner.fail.lock().unwrap_or_else(|e| e.into_inner()) {
            return Err(());
        }
        let rcode = self
            .inner
            .rcode
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(0);
        let ips = self
            .inner
            .ips
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        Ok(response_for(query, rcode, &ips))
    }

    fn system(&self, host: &str, port: u16) -> Result<Vec<SocketAddr>, ()> {
        self.inner
            .system
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(host.to_string());
        if *self
            .inner
            .system_fail
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        {
            return Err(());
        }
        Ok(self
            .inner
            .system_addrs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|addr| SocketAddr::new(addr.ip(), port))
            .collect())
    }
}

#[cfg(test)]
fn response_for(query: &[u8], rcode: u16, ips: &[IpAddr]) -> Vec<u8> {
    let mut msg = query.to_vec();
    let flags = 0x8000 | (rcode & 0x000F);
    msg[2] = (flags >> 8) as u8;
    msg[3] = (flags & 0xFF) as u8;
    let an = if rcode == 0 {
        u16::try_from(ips.len()).unwrap_or(0)
    } else {
        0
    };
    msg[6] = (an >> 8) as u8;
    msg[7] = (an & 0xFF) as u8;
    if rcode != 0 {
        return msg;
    }
    for ip in ips {
        msg.extend_from_slice(&[0xC0, 0x0C]);
        let (typ, rdata) = match ip {
            IpAddr::V4(v4) => (QTYPE_A, v4.octets().to_vec()),
            IpAddr::V6(v6) => (QTYPE_AAAA, v6.octets().to_vec()),
        };
        msg.extend_from_slice(&typ.to_be_bytes());
        msg.extend_from_slice(&1u16.to_be_bytes());
        msg.extend_from_slice(&60u32.to_be_bytes());
        let rdlen = u16::try_from(rdata.len()).unwrap_or(0);
        msg.extend_from_slice(&rdlen.to_be_bytes());
        msg.extend_from_slice(&rdata);
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_config::{QUAD9_DNS_V4, QUAD9_DOH_URL};

    fn quad9() -> ClearnetDns {
        ClearnetDns::quad9()
    }

    fn lookup(
        plan: &ClearnetDns,
        host: &str,
        ex: &ScriptedExchange,
    ) -> Result<Vec<SocketAddr>, DnsError> {
        resolve_in_scope(DnsScope::Clearnet, plan, host, 443, ex)
    }

    #[test]
    fn quad9_is_the_default_and_does_not_call_the_system() {
        let plan = quad9();
        assert_eq!(plan.doh_url(), Some(QUAD9_DOH_URL));
        assert!(!plan.allows_system_resolver());
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))]);
        let addrs = lookup(&plan, "example.com", &ex).unwrap();
        assert_eq!(addrs, vec!["93.184.216.34:443".parse().unwrap()]);
        assert!(ex.doh_calls().iter().all(|u| u == QUAD9_DOH_URL));
        assert_eq!(ex.doh_calls().len(), 2);
        assert!(ex.system_calls().is_empty());
    }

    #[test]
    fn transport_failure_does_not_fall_back_unless_opted_in() {
        let ex = ScriptedExchange::doh_fails();
        let err = lookup(&quad9(), "example.com", &ex).unwrap_err();
        assert_eq!(err, DnsError::Transport);
        assert!(ex.system_calls().is_empty());

        let opted = ClearnetDns::Doh {
            url: QUAD9_DOH_URL.into(),
            system_fallback: true,
        };
        let addrs = lookup(&opted, "example.com", &ex).unwrap();
        assert_eq!(addrs, vec!["93.184.216.34:443".parse().unwrap()]);
        assert_eq!(ex.system_calls(), vec!["example.com".to_string()]);
    }

    #[test]
    fn nxdomain_and_refusal_do_not_fall_back_even_when_opted_in() {
        let opted = ClearnetDns::Doh {
            url: QUAD9_DOH_URL.into(),
            system_fallback: true,
        };
        let nx = ScriptedExchange::nxdomain();
        assert_eq!(
            lookup(&opted, "blocked.example", &nx).unwrap_err(),
            DnsError::Empty
        );
        assert!(nx.system_calls().is_empty());
        assert!(!nx.doh_calls().is_empty());

        let refused = ScriptedExchange::refused();
        assert_eq!(
            lookup(&opted, "blocked.example", &refused).unwrap_err(),
            DnsError::Refused
        );
        assert!(refused.system_calls().is_empty());
    }

    #[test]
    fn private_answer_does_not_fall_back() {
        let opted = ClearnetDns::Doh {
            url: QUAD9_DOH_URL.into(),
            system_fallback: true,
        };
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))]);
        assert_eq!(
            lookup(&opted, "rebind.example", &ex).unwrap_err(),
            DnsError::Private
        );
        assert!(ex.system_calls().is_empty());
    }

    #[test]
    fn system_mode_skips_doh() {
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))]);
        let addrs = lookup(&ClearnetDns::System, "example.com", &ex).unwrap();
        assert_eq!(addrs, vec!["93.184.216.34:443".parse().unwrap()]);
        assert!(ex.doh_calls().is_empty());
        assert_eq!(ex.system_calls(), vec!["example.com".to_string()]);
    }

    #[test]
    fn onion_and_i2p_never_hit_either_resolver() {
        let plans = [
            quad9(),
            ClearnetDns::Doh {
                url: QUAD9_DOH_URL.into(),
                system_fallback: true,
            },
            ClearnetDns::System,
        ];
        let names = [
            "www.example.onion",
            "www.example.ONION",
            "www.example.onion.",
            "zzz.i2p",
            "zzz.I2P.",
        ];
        for plan in &plans {
            for name in names {
                let ex = ScriptedExchange::doh_fails();
                let result = lookup(plan, name, &ex);
                assert_eq!(result.unwrap_err(), DnsError::Hidden, "{name}");
                assert!(ex.doh_calls().is_empty(), "{name}");
                assert!(ex.system_calls().is_empty(), "{name}");
            }
        }
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))]);
        assert!(lookup(&quad9(), "notonion.example", &ex).is_ok());
        assert!(!ex.doh_calls().is_empty());
    }

    #[test]
    fn hidden_circuit_never_calls_a_resolver() {
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))]);
        for host in ["example.com", "www.example.onion", "zzz.i2p", "9.9.9.9"] {
            let err =
                resolve_in_scope(DnsScope::HiddenCircuit, &quad9(), host, 443, &ex).unwrap_err();
            assert_eq!(err, DnsError::NoLocalDns, "{host}");
        }
        assert!(ex.doh_calls().is_empty());
        assert!(ex.system_calls().is_empty());
    }

    #[test]
    fn special_use_names_are_not_sent_to_quad9() {
        let ex = ScriptedExchange::doh_fails();
        assert_eq!(
            lookup(&quad9(), "printer.local", &ex).unwrap_err(),
            DnsError::SpecialUse
        );
        assert_eq!(
            lookup(&quad9(), "frihart-test.invalid", &ex).unwrap_err(),
            DnsError::SpecialUse
        );
        assert!(ex.doh_calls().is_empty());
        assert!(ex.system_calls().is_empty());
        let system = ScriptedExchange::answering(Vec::new());
        assert!(lookup(&ClearnetDns::System, "printer.local", &system).is_ok());
        assert_eq!(system.system_calls(), vec!["printer.local".to_string()]);
    }

    #[test]
    fn literals_and_localhost_skip_dns() {
        let ex = ScriptedExchange::doh_fails();
        let v4 = lookup(&quad9(), "93.184.216.34", &ex).unwrap();
        assert_eq!(v4, vec!["93.184.216.34:443".parse().unwrap()]);
        let v6 = lookup(&quad9(), "2606:4700:4700::1111", &ex).unwrap();
        assert_eq!(v6.len(), 1);
        let local = lookup(&quad9(), "localhost", &ex).unwrap();
        assert_eq!(local[0], "127.0.0.1:443".parse().unwrap());
        assert!(ex.doh_calls().is_empty());
        assert!(ex.system_calls().is_empty());
    }

    #[test]
    fn http_doh_url_is_not_queried() {
        let plan = ClearnetDns::Doh {
            url: "http://dns.example/dns-query".into(),
            system_fallback: false,
        };
        let ex = ScriptedExchange::doh_fails();
        assert_eq!(
            lookup(&plan, "example.com", &ex).unwrap_err(),
            DnsError::BadUrl
        );
        assert!(ex.doh_calls().is_empty());
        assert!(ex.system_calls().is_empty());
    }

    #[test]
    fn pinned_bootstrap_is_quad9_and_refuses_other_names() {
        let addrs = quad9_bootstrap(443);
        assert_eq!(addrs[0].ip().to_string(), QUAD9_DNS_V4);
        assert!(addrs.iter().any(|a| a.ip().is_ipv6()));
        let pinned = PinnedQuad9;
        assert_eq!(
            ureq::Resolver::resolve(&pinned, "dns.quad9.net:443").unwrap(),
            addrs
        );
        assert!(ureq::Resolver::resolve(&pinned, "example.com:443").is_err());
        assert!(ureq::Resolver::resolve(&pinned, "www.example.onion:443").is_err());
        assert_eq!(
            ureq::Resolver::resolve(&pinned, "dns.quad9.net.:443").unwrap()[0].port(),
            443
        );
    }

    #[test]
    fn public_answers_keep_one_of_each_family() {
        let ips = vec![
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            IpAddr::V6("2606:4700:4700::1111".parse().unwrap()),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
        ];
        let sockets = public_sockets(&ips, 443).unwrap();
        assert_eq!(
            sockets,
            vec![
                "93.184.216.34:443".parse().unwrap(),
                "[2606:4700:4700::1111]:443".parse().unwrap(),
            ]
        );
    }

    #[test]
    fn parser_reads_a_compressed_answer_and_rejects_a_bad_id() {
        let query = encode_query("example.com", QTYPE_A, 0x1234).unwrap();
        let ips = vec![
            IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)),
            IpAddr::V6("2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()),
        ];
        let msg = response_for(&query, 0, &ips);
        assert_eq!(parse_answers(&msg, 0x1234).unwrap(), ips);
        assert_eq!(
            parse_answers(&msg, 0x1235).unwrap_err(),
            DnsError::Transport
        );
        let nx = response_for(&query, 3, &ips);
        assert!(parse_answers(&nx, 0x1234).unwrap().is_empty());
    }

    #[test]
    fn idna_names_are_ascii_before_a_query() {
        let ascii = ascii_dns_name("räksmörgås.example").unwrap();
        assert!(ascii.starts_with("xn--"));
        assert!(ascii.ends_with(".example"));
        assert!(ascii.is_ascii());
        let ex = ScriptedExchange::answering(vec![IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))]);
        lookup(&quad9(), "räksmörgås.example", &ex).unwrap();
        let query_has_punycode = ex.doh_calls().len() == 2;
        assert!(query_has_punycode);
    }

    #[test]
    fn shared_plan_updates() {
        let shared = SharedClearnetDns::quad9();
        assert!(!shared.get().allows_system_resolver());
        shared.set(ClearnetDns::System);
        assert!(shared.get().allows_system_resolver());
    }
}
