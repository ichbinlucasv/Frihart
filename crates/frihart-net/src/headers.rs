use frihart_privacy::Policy;

use crate::Request;

/// Caller headers allowed through, and only on a request with a body.
/// Everything else is dropped so the request shape is set by policy,
/// not by whichever code built the request.
const BODY_HEADERS: &[&str] = &["content-type"];

/// Rebuild the outgoing header list for one hop.
///
/// Every request carries the same names in the same order: User-Agent,
/// Accept, Accept-Language, then Sec-GPC and DNT when enabled. Cookie is
/// added later by the client for the current hop only. Calling this again
/// after a redirect gives the same list, never a duplicate.
pub fn apply_identity_headers(request: &mut Request, policy: &Policy) {
    let has_body = request.body.is_some();
    let kept: Vec<(String, String)> = request
        .headers
        .drain(..)
        .filter(|(name, _)| has_body && BODY_HEADERS.iter().any(|h| name.eq_ignore_ascii_case(h)))
        .collect();

    let mut headers = vec![
        ("User-Agent".to_string(), policy.user_agent().to_string()),
        // Pinned here so a ureq upgrade cannot change what goes out.
        ("Accept".to_string(), "*/*".to_string()),
        (
            "Accept-Language".to_string(),
            policy.prefs().privacy.language.clone(),
        ),
    ];
    if policy.send_gpc() {
        headers.push(("Sec-GPC".into(), "1".into()));
    }
    if policy.send_dnt() {
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
        apply_identity_headers(&mut req, &policy);
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
        apply_identity_headers(&mut req, &policy);
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
        apply_identity_headers(&mut req, &policy);
        let first = req.headers.clone();
        req.headers.push(("Cookie".into(), "sid=1".into()));
        apply_identity_headers(&mut req, &policy);
        assert_eq!(req.headers, first);
    }

    #[test]
    fn content_type_only_with_a_body() {
        let policy = Policy::new(Prefs::default());
        let url = Url::parse("https://example.com/form").unwrap();

        let mut post = Request::post(url.clone(), b"q=1".to_vec());
        apply_identity_headers(&mut post, &policy);
        assert_eq!(names(&post).last().unwrap(), "content-type");

        let mut get = Request::get(url);
        get.headers
            .push(("Content-Type".into(), "text/plain".into()));
        apply_identity_headers(&mut get, &policy);
        assert!(!names(&get).contains(&"content-type".to_string()));
    }
}
