use std::path::Path;

use serde::{Deserialize, Serialize};
use url::Url;

use frihart_core::{CircuitKind, ContainerId, Result, write_private_str};
use frihart_privacy::{Policy, ResourceKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub partition_host: String,
    pub container: u32,
    /// `direct` / `tor` / `i2p`. Missing in old jars means direct.
    #[serde(default)]
    pub circuit: String,
    /// Set when the cookie had no `Domain` attribute. It is then sent only
    /// to the host that set it. Missing in old jars means the previous,
    /// wider match (the stored domain and its subdomains).
    #[serde(default)]
    pub host_only: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    cookies: Vec<StoredCookie>,
}

impl CookieJar {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let cookies = serde_json::from_str(&text).unwrap_or_default();
        Ok(Self { cookies })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string(&self.cookies).unwrap_or_else(|_| "[]".into());
        write_private_str(path, &text)
    }

    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    pub fn clear(&mut self) {
        self.cookies.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn store(
        &mut self,
        set_cookie: &str,
        request_url: &Url,
        first_party: &str,
        container: ContainerId,
        circuit: CircuitKind,
        policy: &Policy,
        third_party: bool,
    ) {
        if !policy
            .decide(ResourceKind::CookieWrite { third_party })
            .allowed()
        {
            return;
        }
        let Some(parsed) = parse_set_cookie(set_cookie, request_url) else {
            return;
        };
        if parsed.secure && request_url.scheme() != "https" {
            return;
        }
        let stored = StoredCookie {
            name: parsed.name,
            value: parsed.value,
            domain: parsed.domain,
            path: parsed.path,
            secure: parsed.secure,
            http_only: parsed.http_only,
            host_only: parsed.host_only,
            partition_host: first_party.to_ascii_lowercase(),
            container: container.0,
            circuit: circuit.slug().into(),
        };
        self.cookies.retain(|c| {
            !(c.name == stored.name
                && c.domain == stored.domain
                && c.path == stored.path
                && c.host_only == stored.host_only
                && c.container == stored.container
                && c.partition_host == stored.partition_host
                && CircuitKind::from_slug(&c.circuit) == circuit)
        });
        self.cookies.push(stored);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn header_for(
        &self,
        url: &Url,
        first_party: &str,
        container: ContainerId,
        circuit: CircuitKind,
        policy: &Policy,
        third_party: bool,
    ) -> Option<String> {
        if !policy
            .decide(ResourceKind::CookieRead { third_party })
            .allowed()
        {
            return None;
        }
        let host = url.host_str()?.to_ascii_lowercase();
        let path = url.path();
        let https = url.scheme() == "https";
        let part = first_party.to_ascii_lowercase();
        let mut pairs = Vec::new();
        for c in &self.cookies {
            if c.container != container.0 {
                continue;
            }
            if CircuitKind::from_slug(&c.circuit) != circuit {
                continue;
            }
            if c.partition_host != part {
                continue;
            }
            if c.secure && !https {
                continue;
            }
            let domain = c.domain.to_ascii_lowercase();
            if c.host_only {
                if host != domain {
                    continue;
                }
            } else if !domain_attribute_ok(&host, &domain) {
                continue;
            }
            if !path_matches(&c.path, path) {
                continue;
            }
            pairs.push(format!("{}={}", c.name, c.value));
        }
        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }
}

struct ParsedCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    http_only: bool,
    host_only: bool,
}

fn parse_set_cookie(header: &str, request_url: &Url) -> Option<ParsedCookie> {
    let mut parts = header.split(';');
    let nv = parts.next()?.trim();
    let (name, value) = nv.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if !cookie_pair_ok(name, value) {
        return None;
    }
    let host = request_url.host_str()?.to_ascii_lowercase();
    let mut cookie = ParsedCookie {
        name: name.to_string(),
        value: value.to_string(),
        domain: host.clone(),
        path: "/".into(),
        secure: false,
        http_only: false,
        host_only: true,
    };
    for attr in parts {
        let attr = attr.trim();
        let (k, v) = match attr.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => (attr, ""),
        };
        match k.to_ascii_lowercase().as_str() {
            "domain" => {
                let d = v.trim_start_matches('.').to_ascii_lowercase();
                if !domain_attribute_ok(&host, &d) {
                    return None;
                }
                cookie.domain = d;
                cookie.host_only = false;
            }
            "path" => {
                if v.starts_with('/') && !v.chars().any(|c| c.is_ascii_control()) {
                    cookie.path = v.to_string();
                }
            }
            "secure" => cookie.secure = true,
            "httponly" => cookie.http_only = true,
            _ => {}
        }
    }
    Some(cookie)
}

/// `Domain` must be this host, or a dotted parent of it (`www.example.com`
/// may set `example.com`). A single label such as `com` is refused: it is
/// not a registrable parent, and accepting it would send the cookie to
/// every host under that label after a redirect.
fn domain_attribute_ok(host: &str, domain: &str) -> bool {
    if domain.is_empty() || domain.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return false;
    }
    let parent = host == domain || host.ends_with(&format!(".{domain}"));
    parent && (domain.contains('.') || domain == host)
}

fn cookie_pair_ok(name: &str, value: &str) -> bool {
    !name.is_empty()
        && !name
            .chars()
            .any(|c| c.is_ascii_control() || matches!(c, ' ' | ';' | ',' | '='))
        && !value.chars().any(|c| c.is_ascii_control() || c == ';')
}

/// RFC 6265 path-match: `/foo` matches `/foo` and `/foo/bar`, not `/foobar`.
fn path_matches(cookie_path: &str, request_path: &str) -> bool {
    if cookie_path.is_empty() {
        return true;
    }
    if !request_path.starts_with(cookie_path) {
        return false;
    }
    request_path.len() == cookie_path.len()
        || cookie_path.ends_with('/')
        || request_path.as_bytes().get(cookie_path.len()) == Some(&b'/')
}

/// Per first-party host cookie jars for Compat clearnet tabs (Campaign S M3).
///
/// Kept separate from the profile [`CookieJar`] so a Compat site cannot read
/// cookies set while browsing in Reader / Tor / I2P mode. Keys are lowercased
/// host names without a trailing dot. Not persisted to disk in this cut (Servo
/// still owns in-tunnel cookies; chrome uses these for Compat fetches Frihart
/// performs itself, e.g. before handing HTML to Servo).
#[derive(Debug, Clone, Default)]
pub struct SiteCookieJars {
    jars: std::collections::HashMap<String, CookieJar>,
}

impl SiteCookieJars {
    fn key(host: &str) -> String {
        host.trim().trim_end_matches('.').to_ascii_lowercase()
    }

    pub fn for_site_mut(&mut self, host: &str) -> &mut CookieJar {
        let k = Self::key(host);
        self.jars.entry(k).or_default()
    }

    pub fn for_site(&self, host: &str) -> Option<&CookieJar> {
        self.jars.get(&Self::key(host))
    }

    pub fn clear(&mut self) {
        self.jars.clear();
    }

    pub fn site_count(&self) -> usize {
        self.jars.len()
    }

    pub fn clear_site(&mut self, host: &str) -> bool {
        self.jars.remove(&Self::key(host)).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_config::Prefs;
    use frihart_privacy::Policy;

    #[test]
    fn first_party_roundtrip() {
        let mut jar = CookieJar::default();
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://a.example/x").unwrap();
        jar.store(
            "sid=1; Secure; HttpOnly; Path=/",
            &url,
            "a.example",
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            false,
        );
        let hdr = jar
            .header_for(
                &url,
                "a.example",
                ContainerId(1),
                CircuitKind::Direct,
                &policy,
                false,
            )
            .unwrap();
        assert_eq!(hdr, "sid=1");
    }

    #[test]
    fn third_party_write_blocked() {
        let mut jar = CookieJar::default();
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://tracker.test/p").unwrap();
        jar.store(
            "t=1",
            &url,
            "news.test",
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            true,
        );
        assert!(jar.is_empty());
    }

    #[test]
    fn containers_do_not_share_cookies() {
        let mut jar = CookieJar::default();
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://bank.example/").unwrap();
        jar.store(
            "sess=a",
            &url,
            "bank.example",
            ContainerId(3),
            CircuitKind::Direct,
            &policy,
            false,
        );
        assert!(
            jar.header_for(
                &url,
                "bank.example",
                ContainerId(4),
                CircuitKind::Direct,
                &policy,
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn circuits_do_not_share_cookies() {
        let mut jar = CookieJar::default();
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://a.example/").unwrap();
        jar.store(
            "sid=tor",
            &url,
            "a.example",
            ContainerId(1),
            CircuitKind::Tor,
            &policy,
            false,
        );
        assert!(
            jar.header_for(
                &url,
                "a.example",
                ContainerId(1),
                CircuitKind::Direct,
                &policy,
                false,
            )
            .is_none()
        );
        let hdr = jar
            .header_for(
                &url,
                "a.example",
                ContainerId(1),
                CircuitKind::Tor,
                &policy,
                false,
            )
            .unwrap();
        assert_eq!(hdr, "sid=tor");
    }
    #[test]
    fn site_jars_isolate_hosts() {
        let mut sites = SiteCookieJars::default();
        let policy = Policy::new(Prefs::default());
        let a = Url::parse("https://a.example/").unwrap();
        let b = Url::parse("https://b.example/").unwrap();
        sites.for_site_mut("a.example").store(
            "id=1; Path=/",
            &a,
            "a.example",
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            false,
        );
        assert_eq!(sites.site_count(), 1);
        assert!(
            sites
                .for_site("b.example")
                .map(|j| j.is_empty())
                .unwrap_or(true)
        );
        assert!(!sites.for_site_mut("a.example").is_empty());
        sites.for_site_mut("b.example").store(
            "id=2; Path=/",
            &b,
            "b.example",
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            false,
        );
        assert_eq!(sites.site_count(), 2);
        sites.clear_site("a.example");
        assert_eq!(sites.site_count(), 1);
        sites.clear();
        assert_eq!(sites.site_count(), 0);
    }

    fn store_on<'a>(
        jar: &'a mut CookieJar,
        set: &str,
        url: &str,
        party: &str,
    ) -> &'a mut CookieJar {
        let policy = Policy::new(Prefs::default());
        jar.store(
            set,
            &Url::parse(url).unwrap(),
            party,
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            false,
        );
        jar
    }

    fn sent(jar: &CookieJar, url: &str, party: &str) -> Option<String> {
        let policy = Policy::new(Prefs::default());
        jar.header_for(
            &Url::parse(url).unwrap(),
            party,
            ContainerId(1),
            CircuitKind::Direct,
            &policy,
            false,
        )
    }

    #[test]
    fn domain_attribute_must_be_this_host() {
        let mut jar = CookieJar::default();
        store_on(
            &mut jar,
            "a=1; Domain=com",
            "https://evil.example/",
            "evil.example",
        );
        assert!(jar.is_empty(), "Domain=com is not a parent of this host");
        store_on(
            &mut jar,
            "a=1; Domain=bank.com",
            "https://evil.example/",
            "evil.example",
        );
        assert!(jar.is_empty(), "a foreign Domain is refused");
        store_on(
            &mut jar,
            "a=1; Domain=example.com",
            "https://www.example.com/",
            "www.example.com",
        );
        assert_eq!(
            sent(&jar, "https://example.com/", "www.example.com").as_deref(),
            Some("a=1")
        );
        assert_eq!(
            sent(&jar, "https://api.example.com/", "www.example.com").as_deref(),
            Some("a=1")
        );
        assert!(sent(&jar, "https://bank.com/", "www.example.com").is_none());
    }

    #[test]
    fn a_stored_public_suffix_domain_is_not_sent() {
        // A jar written before the Domain check, or edited on disk, must
        // not attach `Domain=com` to every host under `.com`.
        let path =
            std::env::temp_dir().join(format!("frihart-cookies-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"[{"name":"a","value":"1","domain":"com","path":"/","secure":true,"http_only":false,"partition_host":"com","container":1,"circuit":"direct","host_only":false}]"#,
        )
        .unwrap();
        let jar = CookieJar::load(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(sent(&jar, "https://evil.example/", "com").is_none());
        assert!(sent(&jar, "https://bank.com/", "com").is_none());
        assert_eq!(
            sent(&jar, "https://com/", "com").as_deref(),
            Some("a=1"),
            "the single label only matches that exact host"
        );
    }

    #[test]
    fn host_only_cookies_do_not_leak_to_siblings() {
        let mut jar = CookieJar::default();
        store_on(
            &mut jar,
            "sid=1",
            "https://www.example.com/",
            "www.example.com",
        );
        assert_eq!(
            sent(&jar, "https://www.example.com/app", "www.example.com").as_deref(),
            Some("sid=1")
        );
        assert!(sent(&jar, "https://example.com/", "www.example.com").is_none());
        assert!(sent(&jar, "https://other.example.com/", "www.example.com").is_none());
        store_on(
            &mut jar,
            "sid=bad\r\nX: 1",
            "https://www.example.com/",
            "www.example.com",
        );
        assert_eq!(
            sent(&jar, "https://www.example.com/", "www.example.com").as_deref(),
            Some("sid=1"),
            "a control character in the value is not stored"
        );
    }

    #[test]
    fn cookie_path_does_not_match_a_prefix_of_the_name() {
        let mut jar = CookieJar::default();
        store_on(
            &mut jar,
            "sid=1; Path=/foo",
            "https://example.com/foo",
            "example.com",
        );
        assert!(sent(&jar, "https://example.com/foobar", "example.com").is_none());
        assert_eq!(
            sent(&jar, "https://example.com/foo/bar", "example.com").as_deref(),
            Some("sid=1")
        );
    }
}
