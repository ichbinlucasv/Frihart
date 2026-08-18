//! Strip known tracking query keys. Native ClearURLs stance.

use url::Url;

const KEYS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "utm_name",
    "utm_cid",
    "utm_reader",
    "utm_referrer",
    "utm_social",
    "utm_social-type",
    "fbclid",
    "fb_action_ids",
    "fb_action_types",
    "fb_source",
    "fb_ref",
    "gclid",
    "gclsrc",
    "dclid",
    "gbraid",
    "wbraid",
    "gad_source",
    "gad_campaignid",
    "msclkid",
    "twclid",
    "yclid",
    "mc_cid",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "hsa_cam",
    "hsa_grp",
    "hsa_mt",
    "hsa_src",
    "hsa_ad",
    "hsa_acc",
    "hsa_net",
    "hsa_kw",
    "hsa_tgt",
    "hsa_ver",
    "igshid",
    "mkt_tok",
    "_openstat",
    "ref_src",
    "ref_url",
    "spm",
    "scm",
    "ns_campaign",
    "ns_mchannel",
    "ns_source",
    "ns_linkname",
    "icid",
    "ncid",
    "nr_email_referer",
    "vero_conv",
    "vero_id",
    "wickedid",
    "yclid",
    "zanpid",
    "mbid",
    "cmpid",
    "s_cid",
];

pub fn is_tracking_key(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    KEYS.contains(&n.as_str()) || n.starts_with("utm_")
}

pub fn strip_tracking(url: &Url) -> Url {
    let mut out = url.clone();
    let pairs: Vec<(String, String)> = out
        .query_pairs()
        .filter(|(k, _)| !is_tracking_key(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if pairs.is_empty() {
        out.set_query(None);
    } else {
        let q = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding_lite(k), urlencoding_lite(v)))
            .collect::<Vec<_>>()
            .join("&");
        out.set_query(Some(&q));
    }
    out
}

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_utm_and_click_ids() {
        let u = Url::parse("https://ex.test/a?utm_source=x&q=keep&fbclid=1&gclid=2").unwrap();
        let clean = strip_tracking(&u);
        assert_eq!(clean.as_str(), "https://ex.test/a?q=keep");
    }

    #[test]
    fn empty_query_dropped() {
        let u = Url::parse("https://ex.test/?utm_campaign=x").unwrap();
        assert_eq!(strip_tracking(&u).as_str(), "https://ex.test/");
    }
}
