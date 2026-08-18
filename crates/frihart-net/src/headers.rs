use frihart_privacy::Policy;

use crate::Request;

pub fn apply_identity_headers(request: &mut Request, policy: &Policy) {
    request.headers.retain(|(name, _)| {
        let lower = name.to_ascii_lowercase();
        lower != "user-agent"
            && lower != "dnt"
            && lower != "sec-gpc"
            && lower != "referer"
            && !lower.starts_with("sec-ch-")
    });
    request
        .headers
        .push(("User-Agent".into(), policy.user_agent().to_string()));
    request.headers.push((
        "Accept-Language".into(),
        policy.prefs().privacy.language.clone(),
    ));
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
    use url::Url;

    #[test]
    fn identity_headers_are_frozen() {
        let policy = Policy::new(Prefs::default());
        let mut req = Request::get(Url::parse("https://example.com").unwrap());
        req.headers
            .push(("sec-ch-ua".into(), "should-not-survive".into()));
        req.headers
            .push(("Referer".into(), "https://leak.test/x".into()));
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
        assert!(!names.contains(&"referer".to_string()));
    }
}
