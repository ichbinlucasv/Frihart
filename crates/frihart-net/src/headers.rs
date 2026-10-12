use frihart_core::{CircuitKind, FROZEN_USER_AGENT};
use frihart_privacy::Policy;

use crate::Request;

/// Caller headers allowed through, and only on a request with a body.
/// Everything else is dropped so the request shape is set by policy,
/// not by whichever code built the request.
const BODY_HEADERS: &[&str] = &["content-type"];

/// Accept-Language on Tor and I2P tabs, whatever the profile says. It is
/// the shipped default, so every Frihart user on a hidden circuit sends
/// the same value.
pub const HIDDEN_NET_LANGUAGE: &str = "en";

/// Rebuild the outgoing header list for one hop.
///
/// Every request carries the same names in the same order: User-Agent,
/// Accept, Accept-Language, then Sec-GPC and DNT when enabled. Cookie is
/// added later by the client for the current hop only. Calling this again
/// after a redirect gives the same list, never a duplicate.
///
/// On Tor and I2P circuits the values are pinned too: the frozen
/// User-Agent, [`HIDDEN_NET_LANGUAGE`], Sec-GPC and never DNT. A changed
/// user agent, language or DNT setting would make that user stand out among
/// everyone else on the same exits, so those prefs only apply to direct tabs.
pub fn apply_identity_headers(request: &mut Request, policy: &Policy, circuit: CircuitKind) {
    let has_body = request.body.is_some();
    let kept: Vec<(String, String)> = request
        .headers
        .drain(..)
        .filter(|(name, _)| has_body && BODY_HEADERS.iter().any(|h| name.eq_ignore_ascii_case(h)))
        .collect();

    let hidden = circuit != CircuitKind::Direct;
    let (user_agent, language) = if hidden {
        (
            FROZEN_USER_AGENT.to_string(),
            HIDDEN_NET_LANGUAGE.to_string(),
        )
    } else {
        (
            policy.user_agent().to_string(),
            policy.prefs().privacy.language.clone(),
        )
    };
    let mut headers = vec![
        ("User-Agent".to_string(), user_agent),
        // Pinned here so a ureq upgrade cannot change what goes out.
        ("Accept".to_string(), "*/*".to_string()),
        ("Accept-Language".to_string(), language),
    ];
    if hidden || policy.send_gpc() {
        headers.push(("Sec-GPC".into(), "1".into()));
    }
    if !hidden && policy.send_dnt() {
        headers.push(("DNT".into(), "1".into()));
    }
    headers.extend(kept);
    request.headers = headers;
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_config::Prefs;
    use frihart_privacy::Policy;
    use url::Url;

    fn names(req: &Request) -> Vec<String> {
        req.headers
            .iter()
            .map(|(n, _)| n.to_ascii_lowercase())
            .collect()
    }

    #[test]
    fn identity_headers_are_frozen() {
        let policy = Policy::new(Prefs::default());
        let mut req = Request::get(Url::parse("https://example.com").unwrap());
        req.headers
            .push(("sec-ch-ua".into(), "should-not-survive".into()));
        req.headers
            .push(("Referer".into(), "https://leak.test/x".into()));
        apply_identity_headers(&mut req, &policy, CircuitKind::Direct);
        let names = names(&req);
        assert!(names.contains(&"user-agent".to_string()));
        assert!(names.contains(&"sec-gpc".to_string()));
        assert!(!names.contains(&"dnt".to_string()));
        assert!(!names.contains(&"sec-ch-ua".to_string()));
        assert!(!names.contains(&"referer".to_string()));
    }

    #[test]
    fn header_set_and_order_are_fixed() {
        let policy = Policy::new(Prefs::default());
        let mut req = Request::get(Url::parse("https://example.com").unwrap());
        req.headers.push(("X-Requested-With".into(), "app".into()));
        req.headers.push(("Accept".into(), "text/x-unique".into()));
        apply_identity_headers(&mut req, &policy, CircuitKind::Direct);
        assert_eq!(
            names(&req),
            ["user-agent", "accept", "accept-language", "sec-gpc"]
        );
        assert_eq!(req.headers[1].1, "*/*");
    }

    #[test]
    fn second_hop_does_not_duplicate() {
        let policy = Policy::new(Prefs::default());
        let mut req = Request::get(Url::parse("https://example.com").unwrap());
        apply_identity_headers(&mut req, &policy, CircuitKind::Direct);
        let first = req.headers.clone();
        req.headers.push(("Cookie".into(), "sid=1".into()));
        apply_identity_headers(&mut req, &policy, CircuitKind::Direct);
        assert_eq!(req.headers, first);
    }

    fn custom_prefs() -> Policy {
        let mut prefs = Prefs::default();
        prefs.network.user_agent = "Custom/9.9".into();
        prefs.privacy.language = "pt-BR,pt;q=0.9,fr;q=0.8".into();
        prefs.privacy.send_dnt = true;
        prefs.privacy.send_gpc = false;
        Policy::new(prefs)
    }

    #[test]
    fn hidden_circuits_send_one_uniform_set() {
        let policy = custom_prefs();
        for circuit in [CircuitKind::Tor, CircuitKind::I2p] {
            let mut req = Request::get(Url::parse("http://example.onion/").unwrap());
            apply_identity_headers(&mut req, &policy, circuit);
            assert_eq!(
                req.headers,
                vec![
                    ("User-Agent".to_string(), FROZEN_USER_AGENT.to_string()),
                    ("Accept".to_string(), "*/*".to_string()),
                    (
                        "Accept-Language".to_string(),
                        HIDDEN_NET_LANGUAGE.to_string()
                    ),
                    ("Sec-GPC".to_string(), "1".to_string()),
                ]
            );
        }
    }

    #[test]
    fn hidden_set_matches_shipped_defaults() {
        let policy = Policy::new(Prefs::default());
        let mut direct = Request::get(Url::parse("https://example.com/").unwrap());
        let mut tor = direct.clone();
        apply_identity_headers(&mut direct, &policy, CircuitKind::Direct);
        apply_identity_headers(&mut tor, &policy, CircuitKind::Tor);
        assert_eq!(direct.headers, tor.headers);
    }

    #[test]
    fn direct_tabs_keep_user_prefs() {
        let policy = custom_prefs();
        let mut req = Request::get(Url::parse("https://example.com/").unwrap());
        apply_identity_headers(&mut req, &policy, CircuitKind::Direct);
        assert_eq!(req.headers[0].1, "Custom/9.9");
        assert_eq!(req.headers[2].1, "pt-BR,pt;q=0.9,fr;q=0.8");
        assert_eq!(
            names(&req),
            ["user-agent", "accept", "accept-language", "dnt"]
        );
    }

    #[test]
    fn content_type_only_with_a_body() {
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://example.com/form").unwrap();

        let mut post = Request::post(url.clone(), b"q=1".to_vec());
        apply_identity_headers(&mut post, &policy, CircuitKind::Direct);
        assert_eq!(names(&post).last().unwrap(), "content-type");

        let mut get = Request::get(url);
        get.headers
            .push(("Content-Type".into(), "text/plain".into()));
        apply_identity_headers(&mut get, &policy, CircuitKind::Direct);
        assert!(!names(&get).contains(&"content-type".to_string()));
    }
}
