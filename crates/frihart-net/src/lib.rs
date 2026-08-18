//! Network stack interfaces.
//!
//! The implementation is Phase 2. Phase 0/1 only ships the types so
//! chrome and content can talk about requests without growing a hidden
//! HTTP client.

#![forbid(unsafe_code)]

use frihart_core::{FrihartError, Result};
use frihart_privacy::{Policy, ResourceKind};
use url::Url;

/// An outbound request that has not yet been sent.
#[derive(Clone, Debug)]
pub struct Request {
    pub url: Url,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn get(url: Url) -> Self {
        Self {
            url,
            method: "GET".into(),
            headers: Vec::new(),
        }
    }
}

/// A response that would come back from Phase 2.
#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// HTTP client contract. Chrome must not call this until Phase 2 wires
/// a real implementation through the policy engine.
pub trait HttpClient {
    fn send(&self, request: Request, policy: &Policy) -> Result<Response>;
}

/// Placeholder client. Always fails, on purpose.
#[derive(Clone, Debug, Default)]
pub struct UnimplementedClient;

impl HttpClient for UnimplementedClient {
    fn send(&self, request: Request, policy: &Policy) -> Result<Response> {
        let https = request.url.scheme() == "https";
        let decision = policy.decide(ResourceKind::OutboundHttp { https });
        if !decision.allowed() {
            return Err(FrihartError::network(
                decision.reason().unwrap_or("blocked by policy").to_string(),
            ));
        }
        Err(FrihartError::network(
            "the network stack is Phase 2; this client does not send bytes",
        ))
    }
}

/// Apply the frozen identity headers Frihart is willing to send.
pub fn apply_identity_headers(request: &mut Request, policy: &Policy) {
    request.headers.retain(|(name, _)| {
        let lower = name.to_ascii_lowercase();
        lower != "user-agent"
            && lower != "dnt"
            && lower != "sec-gpc"
            && !lower.starts_with("sec-ch-")
    });
    request
        .headers
        .push(("User-Agent".into(), policy.user_agent().to_string()));
    if policy.send_gpc() {
        request.headers.push(("Sec-GPC".into(), "1".into()));
    }
    if policy.send_dnt() {
        request.headers.push(("DNT".into(), "1".into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_config::Prefs;
    use frihart_privacy::Policy;

    #[test]
    fn identity_headers_are_frozen() {
        let policy = Policy::new(Prefs::default());
        let mut req = Request::get(Url::parse("https://example.com").unwrap());
        req.headers
            .push(("sec-ch-ua".into(), "should-not-survive".into()));
        apply_identity_headers(&mut req, &policy);
        let names: Vec<_> = req
            .headers
            .iter()
            .map(|(n, _)| n.to_ascii_lowercase())
            .collect();
        assert!(names.contains(&"user-agent".to_string()));
        assert!(names.contains(&"sec-gpc".to_string()));
        assert!(!names.contains(&"dnt".to_string()));
        assert!(!names.contains(&"sec-ch-ua".to_string()));
    }

    #[test]
    fn unimplemented_client_does_not_send() {
        let policy = Policy::new(Prefs::default());
        let client = UnimplementedClient;
        let req = Request::get(Url::parse("https://example.com").unwrap());
        let err = client.send(req, &policy).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Phase 2"), "{msg}");
    }

    #[test]
    fn unimplemented_client_still_honors_https_only() {
        let policy = Policy::new(Prefs::default());
        let client = UnimplementedClient;
        let req = Request::get(Url::parse("http://example.com").unwrap());
        let err = client.send(req, &policy).unwrap_err();
        assert!(err.to_string().contains("HTTPS-only"));
    }
}
