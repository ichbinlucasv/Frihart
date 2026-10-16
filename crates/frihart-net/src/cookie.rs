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
            partition_host: first_party.to_ascii_lowercase(),
            container: container.0,
            circuit: circuit.slug().into(),
        };
        self.cookies.retain(|c| {
            !(c.name == stored.name
                && c.domain == stored.domain
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
            if !domain_matches(&c.domain, &host) {
                continue;
            }
            if !path.starts_with(&c.path) {
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
}

fn parse_set_cookie(header: &str, request_url: &Url) -> Option<ParsedCookie> {
    let mut parts = header.split(';');
    let nv = parts.next()?.trim();
    let (name, value) = nv.split_once('=')?;
    if name.trim().is_empty() {
        return None;
    }
    let mut cookie = ParsedCookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain: request_url.host_str()?.to_ascii_lowercase(),
        path: "/".into(),
        secure: false,
        http_only: false,
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
                if !d.is_empty() {
                    cookie.domain = d;
                }
            }
            "path" => {
                if v.starts_with('/') {
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

fn domain_matches(cookie_domain: &str, host: &str) -> bool {
    host == cookie_domain || host.ends_with(&format!(".{cookie_domain}"))
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
}
