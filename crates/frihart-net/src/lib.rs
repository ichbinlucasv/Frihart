//! rustls HTTP client, first-party cookie jar, no unsolicited I/O.

#![forbid(unsafe_code)]

mod client;
mod cookie;
mod headers;

pub use client::{FetchMode, RustlsClient, content_type, decode_body};
pub use cookie::{CookieJar, StoredCookie};
pub use headers::apply_identity_headers;

use std::path::PathBuf;

use url::Url;

#[derive(Clone, Debug)]
pub struct Request {
    pub url: Url,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn get(url: Url) -> Self {
        Self {
            url,
            method: "GET".into(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn post(url: Url, body: impl Into<Vec<u8>>) -> Self {
        Self {
            url,
            method: "POST".into(),
            headers: vec![(
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            )],
            body: Some(body.into()),
        }
    }
}

/// A download never runs itself.
#[derive(Clone, Debug)]
pub struct Download {
    pub url: Url,
    pub dest: PathBuf,
    pub bytes: u64,
}

impl Download {
    pub fn may_execute(&self) -> bool {
        let _ = (self.url.as_str(), &self.dest, self.bytes);
        false
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub final_url: Url,
}

pub trait HttpClient {
    fn send(
        &self,
        request: Request,
        policy: &frihart_privacy::Policy,
        jar: &mut CookieJar,
        blocker: &frihart_blocker::FilterEngine,
        mode: FetchMode,
        container: frihart_core::ContainerId,
    ) -> frihart_core::Result<Response>;
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn download_never_executes() {
        let d = Download {
            url: Url::parse("https://ex.test/a.bin").unwrap(),
            dest: std::path::PathBuf::from("/tmp/a.bin"),
            bytes: 1,
        };
        assert!(!d.may_execute());
        assert_eq!(
            Request::post(d.url.clone(), b"q=1".as_slice()).method,
            "POST"
        );
    }
}
