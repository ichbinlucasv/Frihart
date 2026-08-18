use std::io::Read;
use std::time::Duration;

use frihart_blocker::FilterEngine;
use frihart_core::{ContainerId, FrihartError, Result, sanitize_error};
use frihart_privacy::{Policy, ResourceKind};

use crate::cookie::CookieJar;
use crate::headers::apply_identity_headers;
use crate::{HttpClient, Request, Response};

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
        let agent;
        let agent_ref: &ureq::Agent = match &mode {
            FetchMode::Direct => &self.agent,
            FetchMode::Tor { socks } => {
                let socks = socks.trim();
                if socks.is_empty() {
                    return Err(FrihartError::network("tor refused: no socks"));
                }
                if !socks.contains(':') {
                    return Err(FrihartError::network("tor refused: socks host:port"));
                }
                let proxy = ureq::Proxy::new(format!("socks5://{socks}"))
                    .map_err(|_| FrihartError::network("tor refused: socks invalid"))?;
                agent = ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(TIMEOUT_SECS))
                    .redirects(0)
                    .user_agent("Frihart")
                    .proxy(proxy)
                    .build();
                &agent
            }
        };

        let mut current = request.url.clone();
        let first_party = current.host_str().unwrap_or("").to_ascii_lowercase();

        for _ in 0..=MAX_REDIRECTS {
            let https = current.scheme() == "https";
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
            if let Some(cookie) = jar.header_for(&current, &first_party, container, policy, false) {
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
                    jar.store(value, &current, &first_party, container, policy, third);
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
}
