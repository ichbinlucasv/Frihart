//! Read-only Tor ControlPort client for the bridge check.
//!
//! Frihart only *asks* the system Tor about bridges (`GETCONF`, `GETINFO`);
//! it never changes Tor's configuration. The authentication is the same
//! fail-closed SAFECOOKIE scheme HashChat uses:
//!
//! - The ControlPort must be on a loopback address.
//! - `PROTOCOLINFO` must offer `SAFECOOKIE`; bare or password `AUTHENTICATE`
//!   is never sent.
//! - The advertised `COOKIEFILE` is accepted only if it resolves to a known
//!   system Tor cookie path, or to `FRIHART_TOR_COOKIE_FILE` when that is set
//!   (then it is the only accepted path). Otherwise a listener that is not Tor
//!   could name a cookie file it wrote itself.
//! - The cookie must be a regular 32-byte file, opened without following a
//!   final symlink. The server's HMAC is checked in constant time **before**
//!   our own proof is sent, and the cookie never leaves the process.
//! - Replies are bounded in line length and line count; errors never include
//!   paths, bridge lines or fingerprints.

use crate::bridges::{BridgeStatus, evaluate_bridge_status};
use ring::hmac;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

/// Env var naming the only cookie file Frihart will accept.
pub const TOR_COOKIE_FILE_ENV: &str = "FRIHART_TOR_COOKIE_FILE";

const DEFAULT_COOKIE_PATHS: &[&str] = &[
    "/run/tor/control.authcookie",
    "/var/run/tor/control.authcookie",
    "/var/lib/tor/control_auth_cookie",
    "/var/lib/tor/control.authcookie",
];

const COOKIE_LEN: usize = 32;
const SERVER_KEY: &[u8] = b"Tor safe cookie authentication server-to-controller hash";
const CLIENT_KEY: &[u8] = b"Tor safe cookie authentication controller-to-server hash";
const MAX_LINE: usize = 4096;
const MAX_LINES: usize = 256;

/// Why the bridge query did not produce a status. Messages are fixed strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlError {
    NotLoopback,
    Unreachable,
    Io,
    ReplyTooLong,
    NoSafeCookie,
    CookieRefused,
    CookieUnreadable,
    BadChallenge,
    ServerProofFailed,
    AuthFailed,
    QueryRefused,
}

impl ControlError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotLoopback => "ControlPort host is not loopback",
            Self::Unreachable => "Tor ControlPort unreachable",
            Self::Io => "Tor ControlPort I/O failed",
            Self::ReplyTooLong => "Tor ControlPort reply too long",
            Self::NoSafeCookie => "Tor ControlPort does not offer SAFECOOKIE",
            Self::CookieRefused => "Tor cookie file is not at an expected location",
            Self::CookieUnreadable => "Tor cookie file unreadable or malformed",
            Self::BadChallenge => "bad AUTHCHALLENGE reply",
            Self::ServerProofFailed => "ControlPort failed the SAFECOOKIE proof (not our Tor)",
            Self::AuthFailed => "Tor cookie authentication failed",
            Self::QueryRefused => "Tor refused the bridge status query",
        }
    }
}

impl std::fmt::Display for ControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for ControlError {}

fn loopback_addr(host: &str, port: u16) -> Option<SocketAddr> {
    let h = host.trim().trim_matches(|c| c == '[' || c == ']');
    let ip = if h.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        h.parse::<IpAddr>().ok()?
    };
    let is_loop = match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6 == Ipv6Addr::LOCALHOST,
    };
    is_loop.then_some(SocketAddr::new(ip, port))
}

/// Ask the system Tor whether bridges are configured and in use.
/// Uses `FRIHART_TOR_COOKIE_FILE` as the only accepted cookie path when set.
pub fn query_bridge_status(host: &str, port: u16) -> Result<BridgeStatus, ControlError> {
    let cookie_override = std::env::var_os(TOR_COOKIE_FILE_ENV).map(PathBuf::from);
    query_bridge_status_with(host, port, cookie_override.as_deref())
}

/// [`query_bridge_status`] with an explicit cookie path allow-list override.
pub fn query_bridge_status_with(
    host: &str,
    port: u16,
    cookie_override: Option<&Path>,
) -> Result<BridgeStatus, ControlError> {
    let addr = loopback_addr(host, port).ok_or(ControlError::NotLoopback)?;
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
        .map_err(|_| ControlError::Unreachable)?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut conn = BufReader::new(stream);
    authenticate(&mut conn, cookie_override)?;
    let ok = |lines: &[String]| lines.last().is_some_and(|l| l.starts_with("250 "));
    let mut conf = command(&mut conn, "GETCONF UseBridges Bridge ClientTransportPlugin")?;
    let mut boot = command(&mut conn, "GETINFO status/bootstrap-phase")?;
    let mut circ = command(&mut conn, "GETINFO circuit-status")?;
    let result = if ok(&conf) && ok(&boot) && ok(&circ) {
        Ok(evaluate_bridge_status(&conf, &boot, &circ))
    } else {
        Err(ControlError::QueryRefused)
    };
    // Bridge lines and relay fingerprints are sensitive; drop them now.
    for v in [&mut conf, &mut boot, &mut circ] {
        v.iter_mut().for_each(Zeroize::zeroize);
    }
    let _ = command(&mut conn, "QUIT");
    result
}

fn command(conn: &mut BufReader<TcpStream>, cmd: &str) -> Result<Vec<String>, ControlError> {
    {
        let s = conn.get_mut();
        s.write_all(cmd.as_bytes()).map_err(|_| ControlError::Io)?;
        s.write_all(b"\r\n").map_err(|_| ControlError::Io)?;
        s.flush().map_err(|_| ControlError::Io)?;
    }
    read_reply(conn)
}

/// Read one control reply with bounded line length and count.
fn read_reply(reader: &mut impl BufRead) -> Result<Vec<String>, ControlError> {
    let mut lines = Vec::new();
    loop {
        if lines.len() >= MAX_LINES {
            return Err(ControlError::ReplyTooLong);
        }
        let mut raw = Vec::new();
        let n = reader
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut raw)
            .map_err(|_| ControlError::Io)?;
        if n == 0 {
            break;
        }
        if raw.len() > MAX_LINE {
            raw.zeroize();
            return Err(ControlError::ReplyTooLong);
        }
        let mut t = String::from_utf8(raw).map_err(|e| {
            let mut b = e.into_bytes();
            b.zeroize();
            ControlError::Io
        })?;
        let end = t.trim_end_matches(['\r', '\n']).len();
        t.truncate(end);
        let done = t.starts_with("250 ") || t.starts_with('5');
        lines.push(t);
        if done {
            break;
        }
    }
    Ok(lines)
}

fn authenticate(
    conn: &mut BufReader<TcpStream>,
    cookie_override: Option<&Path>,
) -> Result<(), ControlError> {
    let info = command(conn, "PROTOCOLINFO 1")?;
    if !offers_safecookie(&info) {
        return Err(ControlError::NoSafeCookie);
    }
    let advertised = cookie_path_from_protocolinfo(&info).ok_or(ControlError::NoSafeCookie)?;
    let path = resolve_cookie_path(&advertised, cookie_override)?;
    let mut cookie = read_cookie(&path)?;
    let mut client_nonce = [0u8; 32];
    if getrandom::getrandom(&mut client_nonce).is_err() {
        cookie.zeroize();
        return Err(ControlError::Io);
    }
    let result = safecookie(conn, &cookie, &client_nonce);
    cookie.zeroize();
    result
}

fn safecookie(
    conn: &mut BufReader<TcpStream>,
    cookie: &[u8; COOKIE_LEN],
    client_nonce: &[u8; 32],
) -> Result<(), ControlError> {
    let chal = command(
        conn,
        &format!("AUTHCHALLENGE SAFECOOKIE {}", hex_lower(client_nonce)),
    )?;
    let (server_hash, server_nonce) =
        parse_authchallenge(&chal).ok_or(ControlError::BadChallenge)?;
    let mut expected = safecookie_hmac(SERVER_KEY, cookie, client_nonce, &server_nonce);
    let ok: bool = expected.ct_eq(&server_hash).into();
    expected.zeroize();
    if !ok {
        // The peer does not know the cookie: it is not our Tor. Send nothing more.
        return Err(ControlError::ServerProofFailed);
    }
    let mut client_hash = safecookie_hmac(CLIENT_KEY, cookie, client_nonce, &server_nonce);
    let mut hex = hex_lower(&client_hash);
    client_hash.zeroize();
    let resp = command(conn, &format!("AUTHENTICATE {hex}"));
    hex.zeroize();
    match resp?.as_slice() {
        [only] if only == "250 OK" => Ok(()),
        _ => Err(ControlError::AuthFailed),
    }
}

/// HMAC-SHA256(key, cookie ‖ client_nonce ‖ server_nonce) per control-spec.
pub(crate) fn safecookie_hmac(
    key: &[u8],
    cookie: &[u8; COOKIE_LEN],
    client_nonce: &[u8; 32],
    server_nonce: &[u8; 32],
) -> [u8; 32] {
    let k = hmac::Key::new(hmac::HMAC_SHA256, key);
    let mut ctx = hmac::Context::with_key(&k);
    ctx.update(cookie);
    ctx.update(client_nonce);
    ctx.update(server_nonce);
    let mut out = [0u8; 32];
    out.copy_from_slice(ctx.sign().as_ref());
    out
}

fn hex_lower(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 0x0f) as usize] as char);
    }
    s
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let nib = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (nib(b[2 * i])? << 4) | nib(b[2 * i + 1])?;
    }
    Some(out)
}

/// `250 AUTHCHALLENGE SERVERHASH=<64 hex> SERVERNONCE=<64 hex>`.
fn parse_authchallenge(lines: &[String]) -> Option<([u8; 32], [u8; 32])> {
    let [line] = lines else { return None };
    let rest = line.strip_prefix("250 AUTHCHALLENGE ")?;
    let (mut hash, mut nonce) = (None, None);
    for tok in rest.split_ascii_whitespace() {
        if let Some(v) = tok.strip_prefix("SERVERHASH=") {
            if hash.replace(hex32(v)?).is_some() {
                return None;
            }
        } else if let Some(v) = tok.strip_prefix("SERVERNONCE=") {
            if nonce.replace(hex32(v)?).is_some() {
                return None;
            }
        }
    }
    Some((hash?, nonce?))
}

fn offers_safecookie(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.strip_prefix("250-AUTH ")
            .and_then(|rest| {
                rest.split_ascii_whitespace()
                    .find_map(|t| t.strip_prefix("METHODS="))
            })
            .is_some_and(|m| m.split(',').any(|x| x == "SAFECOOKIE"))
    })
}

/// `COOKIEFILE="…"` from the PROTOCOLINFO reply.
fn cookie_path_from_protocolinfo(lines: &[String]) -> Option<String> {
    for line in lines {
        if !line.starts_with("250-AUTH ") {
            continue;
        }
        if let Some(idx) = line.find("COOKIEFILE=") {
            let rest = line[idx + "COOKIEFILE=".len()..].trim_start_matches('"');
            let end = rest.find('"').unwrap_or(rest.len());
            let p = rest[..end].trim();
            if !p.is_empty() {
                return Some(p.to_string());
            }
        }
    }
    None
}

fn resolve_cookie_path(
    advertised: &str,
    cookie_override: Option<&Path>,
) -> Result<PathBuf, ControlError> {
    let adv = Path::new(advertised);
    if !adv.is_absolute() {
        return Err(ControlError::CookieRefused);
    }
    let canon = std::fs::canonicalize(adv).map_err(|_| ControlError::CookieUnreadable)?;
    let allowed: Vec<PathBuf> = match cookie_override {
        Some(p) if p.is_absolute() => vec![p.to_path_buf()],
        Some(_) => return Err(ControlError::CookieRefused),
        None => DEFAULT_COOKIE_PATHS.iter().map(PathBuf::from).collect(),
    };
    allowed
        .iter()
        .filter_map(|a| std::fs::canonicalize(a).ok())
        .any(|ca| ca == canon)
        .then_some(canon)
        .ok_or(ControlError::CookieRefused)
}

fn read_cookie(path: &Path) -> Result<[u8; COOKIE_LEN], ControlError> {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    let f = opts
        .open(path)
        .map_err(|_| ControlError::CookieUnreadable)?;
    let md = f.metadata().map_err(|_| ControlError::CookieUnreadable)?;
    if !md.is_file() || md.len() != COOKIE_LEN as u64 {
        return Err(ControlError::CookieUnreadable);
    }
    let mut cookie = [0u8; COOKIE_LEN];
    let mut f = f.take(COOKIE_LEN as u64 + 1);
    let mut extra = [0u8; 1];
    if f.read_exact(&mut cookie).is_err() || matches!(f.read(&mut extra), Ok(n) if n > 0) {
        cookie.zeroize();
        return Err(ControlError::CookieUnreadable);
    }
    Ok(cookie)
}

/// How long a passing bridge check is reused before Tor is asked again.
pub const BRIDGE_CHECK_OK_SECS: u64 = 30;
/// How long a failing bridge check is reused (shorter, so recovery is quick).
pub const BRIDGE_CHECK_FAIL_SECS: u64 = 10;

/// Cached fail-closed gate for Tor tabs when `tor.require_bridges` is on.
///
/// Each check reuses the last ControlPort answer for a short time so a page
/// with many subresources does not open a control connection per request.
/// A query error or any verdict other than Active refuses.
#[derive(Debug, Default)]
pub struct BridgeGate {
    last: Option<(std::time::Instant, Result<BridgeStatus, ControlError>)>,
}

impl BridgeGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check against the system Tor at `host:control_port`.
    pub fn check(&mut self, require: bool, host: &str, control_port: u16) -> Result<(), String> {
        self.check_with(require, std::time::Instant::now(), || {
            query_bridge_status(host, control_port)
        })
    }

    /// [`Self::check`] with an injected clock and query (tests).
    pub fn check_with(
        &mut self,
        require: bool,
        now: std::time::Instant,
        query: impl FnOnce() -> Result<BridgeStatus, ControlError>,
    ) -> Result<(), String> {
        if !require {
            return Ok(());
        }
        let fresh = self.last.as_ref().is_some_and(|(at, res)| {
            let ttl = if res
                .as_ref()
                .is_ok_and(|st| crate::bridges::allow_tor_fetch(true, Some(st)).is_ok())
            {
                BRIDGE_CHECK_OK_SECS
            } else {
                BRIDGE_CHECK_FAIL_SECS
            };
            now.saturating_duration_since(*at) < Duration::from_secs(ttl)
        });
        if !fresh {
            self.last = Some((now, query()));
        }
        match self.last.as_ref().map(|(_, r)| r) {
            Some(Ok(st)) => crate::bridges::allow_tor_fetch(true, Some(st)).map_err(str::to_string),
            Some(Err(e)) => Err(format!("bridges required: {}", e.message())),
            None => Err("bridges required: Tor bridge status not checked yet".into()),
        }
    }

    /// Summary of the last answer for `about:bridges`, if any.
    pub fn last_summary(&self) -> Option<String> {
        self.last.as_ref().map(|(_, r)| match r {
            Ok(st) => st.summary_line(),
            Err(e) => format!("bridges=unknown ({})", e.message()),
        })
    }

    /// Forget the cached answer (e.g. after the require switch changes).
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridges::BridgeVerdict;
    use std::net::TcpListener;
    use std::thread;

    const FP: &str = "0123456789ABCDEF0123456789ABCDEF01234567";

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parsers_are_strict() {
        let info = s(&[
            "250-PROTOCOLINFO 1",
            "250-AUTH METHODS=COOKIE,SAFECOOKIE COOKIEFILE=\"/run/tor/control.authcookie\"",
            "250 OK",
        ]);
        assert!(offers_safecookie(&info));
        assert_eq!(
            cookie_path_from_protocolinfo(&info).as_deref(),
            Some("/run/tor/control.authcookie")
        );
        assert!(!offers_safecookie(&s(&[
            "250-AUTH METHODS=NULL,HASHEDPASSWORD"
        ])));
        let h = "ab".repeat(32);
        let ok = s(&[&format!("250 AUTHCHALLENGE SERVERHASH={h} SERVERNONCE={h}")]);
        assert!(parse_authchallenge(&ok).is_some());
        let dup = s(&[&format!(
            "250 AUTHCHALLENGE SERVERHASH={h} SERVERHASH={h} SERVERNONCE={h}"
        )]);
        assert!(parse_authchallenge(&dup).is_none());
        assert!(parse_authchallenge(&s(&["250 AUTHCHALLENGE SERVERHASH=zz"])).is_none());
        assert!(hex32(&"g".repeat(64)).is_none());
    }

    #[test]
    fn non_loopback_and_relative_cookie_are_refused() {
        assert_eq!(
            query_bridge_status_with("192.0.2.1", 9051, None).unwrap_err(),
            ControlError::NotLoopback
        );
        assert_eq!(
            query_bridge_status_with("example.org", 9051, None).unwrap_err(),
            ControlError::NotLoopback
        );
        assert_eq!(
            resolve_cookie_path("relative/cookie", None).unwrap_err(),
            ControlError::CookieRefused
        );
    }

    fn temp_cookie(tag: &str, bytes: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "frihart-ctl-{tag}-{}-{:?}",
            std::process::id(),
            thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("control.authcookie");
        std::fs::write(&p, bytes).unwrap();
        p
    }

    /// Minimal fake Tor: speaks PROTOCOLINFO / AUTHCHALLENGE / AUTHENTICATE and
    /// answers the three bridge queries. `knows_cookie` false simulates an
    /// impostor that cannot compute the server proof.
    fn fake_tor(
        cookie_path: PathBuf,
        cookie: [u8; 32],
        knows_cookie: bool,
    ) -> (u16, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut r = BufReader::new(stream.try_clone().unwrap());
            let mut w = stream;
            let mut seen = Vec::new();
            let server_nonce = [7u8; 32];
            let mut line = String::new();
            while r.read_line(&mut line).unwrap_or(0) > 0 {
                let cmd = line.trim_end().to_string();
                line.clear();
                seen.push(cmd.split(' ').next().unwrap_or("").to_string());
                let reply = if cmd == "PROTOCOLINFO 1" {
                    format!(
                        "250-PROTOCOLINFO 1\r\n250-AUTH METHODS=COOKIE,SAFECOOKIE COOKIEFILE=\"{}\"\r\n250 OK\r\n",
                        cookie_path.display()
                    )
                } else if let Some(n) = cmd.strip_prefix("AUTHCHALLENGE SAFECOOKIE ") {
                    let cn = hex32(n).unwrap();
                    let key = if knows_cookie { cookie } else { [0u8; 32] };
                    let sh = safecookie_hmac(SERVER_KEY, &key, &cn, &server_nonce);
                    format!(
                        "250 AUTHCHALLENGE SERVERHASH={} SERVERNONCE={}\r\n",
                        hex_lower(&sh),
                        hex_lower(&server_nonce)
                    )
                } else if cmd.starts_with("AUTHENTICATE ") {
                    "250 OK\r\n".to_string()
                } else if cmd.starts_with("GETCONF") {
                    format!(
                        "250-UseBridges=1\r\n250-Bridge=obfs4 192.0.2.10:443 {FP} cert=AAAA iat-mode=0\r\n250 ClientTransportPlugin=obfs4 exec /usr/bin/lyrebird\r\n"
                    )
                } else if cmd == "GETINFO status/bootstrap-phase" {
                    "250-status/bootstrap-phase=NOTICE BOOTSTRAP PROGRESS=100 TAG=done\r\n250 OK\r\n".to_string()
                } else if cmd == "GETINFO circuit-status" {
                    format!("250+circuit-status=\r\n1 BUILT ${FP}~b1,$AAAA~x\r\n.\r\n250 OK\r\n")
                } else if cmd == "QUIT" {
                    let _ = w.write_all(b"250 closing connection\r\n");
                    break;
                } else {
                    "510 Unrecognized command\r\n".to_string()
                };
                if w.write_all(reply.as_bytes()).is_err() {
                    break;
                }
            }
            seen
        });
        (port, h)
    }

    #[test]
    fn safecookie_query_against_fake_tor_reports_active() {
        let cookie = [0x5Au8; 32];
        let path = temp_cookie("ok", &cookie);
        let (port, h) = fake_tor(path.clone(), cookie, true);
        let st = query_bridge_status_with("127.0.0.1", port, Some(&path)).unwrap();
        assert_eq!(st.verdict(), BridgeVerdict::Active, "{st:?}");
        assert_eq!(st.bridge_count, 1);
        let seen = h.join().unwrap();
        assert_eq!(seen[0], "PROTOCOLINFO");
        assert!(seen.contains(&"AUTHENTICATE".to_string()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn impostor_never_receives_our_proof() {
        let cookie = [0x33u8; 32];
        let path = temp_cookie("imp", &cookie);
        let (port, h) = fake_tor(path.clone(), cookie, false);
        assert_eq!(
            query_bridge_status_with("127.0.0.1", port, Some(&path)).unwrap_err(),
            ControlError::ServerProofFailed
        );
        let seen = h.join().unwrap();
        assert!(!seen.contains(&"AUTHENTICATE".to_string()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cookie_outside_allow_list_or_wrong_size_is_refused() {
        let cookie = [0x11u8; 32];
        let path = temp_cookie("list", &cookie);
        let other = temp_cookie("other", &cookie);
        let (port, h) = fake_tor(path.clone(), cookie, true);
        assert_eq!(
            query_bridge_status_with("127.0.0.1", port, Some(&other)).unwrap_err(),
            ControlError::CookieRefused
        );
        drop(h);
        let short = temp_cookie("short", &[1u8; 31]);
        assert_eq!(
            read_cookie(&short).unwrap_err(),
            ControlError::CookieUnreadable
        );
        for p in [path, other, short] {
            let _ = std::fs::remove_file(p);
        }
    }

    fn active_status() -> BridgeStatus {
        BridgeStatus {
            use_bridges: true,
            bridge_count: 1,
            bootstrap_percent: Some(100),
            ..BridgeStatus::default()
        }
    }

    #[test]
    fn gate_caches_and_fails_closed() {
        use std::cell::Cell;
        use std::time::Instant;
        let mut g = BridgeGate::new();
        let calls = Cell::new(0);
        let t0 = Instant::now();
        // Off: never queries.
        assert!(
            g.check_with(false, t0, || panic!("no query when off"))
                .is_ok()
        );
        // Unreachable ControlPort refuses with a fixed message.
        let err = g
            .check_with(true, t0, || {
                calls.set(calls.get() + 1);
                Err(ControlError::Unreachable)
            })
            .unwrap_err();
        assert_eq!(err, "bridges required: Tor ControlPort unreachable");
        // Within the fail TTL the cached refusal is reused.
        assert!(
            g.check_with(true, t0 + Duration::from_secs(5), || panic!("cached"))
                .is_err()
        );
        // After the fail TTL Tor is asked again; Active opens the gate.
        let later = t0 + Duration::from_secs(BRIDGE_CHECK_FAIL_SECS);
        assert!(
            g.check_with(true, later, || {
                calls.set(calls.get() + 1);
                Ok(active_status())
            })
            .is_ok()
        );
        assert_eq!(calls.get(), 2);
        assert!(g.last_summary().unwrap().starts_with("bridges=active"));
        // Passing answers are reused for the OK TTL, then re-checked.
        assert!(
            g.check_with(true, later + Duration::from_secs(20), || panic!("cached"))
                .is_ok()
        );
        let off = BridgeStatus::default();
        let again = later + Duration::from_secs(BRIDGE_CHECK_OK_SECS);
        assert_eq!(
            g.check_with(true, again, || Ok(off)).unwrap_err(),
            "bridges required: UseBridges is off"
        );
        g.invalidate();
        assert!(g.last_summary().is_none());
    }
}
