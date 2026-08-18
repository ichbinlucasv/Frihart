//! rustls HTTP client, first-party cookie jar, no unsolicited I/O.

#![forbid(unsafe_code)]

mod client;
mod cookie;
mod headers;

pub use client::{FetchMode, RustlsClient, content_type, decode_body};
pub use cookie::{CookieJar, StoredCookie};
pub use headers::apply_identity_headers;

use url::Url;

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
