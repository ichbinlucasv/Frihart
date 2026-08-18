//! JS types. Execution stays off. Campaign G is refuse-first.

#![forbid(unsafe_code)]

use frihart_core::{FrihartError, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    Nop,
    LoadConst,
    Add,
    Return,
}

/// Host APIs. Fingerprint, storage, and network surfaces stay denied
/// even when a runtime exists. Flipping the JS pref does not open them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostApi {
    DomRead,
    DomWrite,
    Fetch,
    Timer,
    Console,
    CanvasToDataUrl,
    WebGl,
    WebGpu,
    AudioContext,
    Eval,
    Wasm,
    NavigatorPlugins,
    Battery,
    ClientRects,
    Cookie,
    LocalStorage,
    SessionStorage,
    IndexedDb,
    WebRtc,
    WebSocket,
    Geolocation,
    Notification,
    Clipboard,
    ServiceWorker,
}

impl HostApi {
    pub const ALL: &'static [Self] = &[
        Self::DomRead,
        Self::DomWrite,
        Self::Fetch,
        Self::Timer,
        Self::Console,
        Self::CanvasToDataUrl,
        Self::WebGl,
        Self::WebGpu,
        Self::AudioContext,
        Self::Eval,
        Self::Wasm,
        Self::NavigatorPlugins,
        Self::Battery,
        Self::ClientRects,
        Self::Cookie,
        Self::LocalStorage,
        Self::SessionStorage,
        Self::IndexedDb,
        Self::WebRtc,
        Self::WebSocket,
        Self::Geolocation,
        Self::Notification,
        Self::Clipboard,
        Self::ServiceWorker,
    ];

    pub fn allowed(self) -> bool {
        let _ = self;
        false
    }

    pub fn fingerprint(self) -> bool {
        matches!(
            self,
            Self::CanvasToDataUrl
                | Self::WebGl
                | Self::WebGpu
                | Self::AudioContext
                | Self::NavigatorPlugins
                | Self::Battery
                | Self::ClientRects
                | Self::WebRtc
                | Self::Geolocation
        )
    }

    pub fn storage(self) -> bool {
        matches!(
            self,
            Self::Cookie | Self::LocalStorage | Self::SessionStorage | Self::IndexedDb
        )
    }

    pub fn network(self) -> bool {
        matches!(self, Self::Fetch | Self::WebRtc | Self::WebSocket)
    }

    pub fn reason(self) -> &'static str {
        match self {
            Self::CanvasToDataUrl
            | Self::WebGl
            | Self::WebGpu
            | Self::AudioContext
            | Self::NavigatorPlugins
            | Self::Battery
            | Self::ClientRects => "fingerprint surface",
            Self::Eval => "eval denied",
            Self::Wasm => "wasm later",
            Self::Cookie | Self::LocalStorage | Self::SessionStorage | Self::IndexedDb => {
                "storage denied"
            }
            Self::WebRtc => "webrtc denied",
            Self::WebSocket | Self::Fetch => "network denied",
            Self::Geolocation => "geolocation denied",
            Self::Notification => "notification denied",
            Self::Clipboard => "clipboard denied",
            Self::ServiceWorker => "service worker denied",
            _ => "js off",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Runtime {
    pub enabled: bool,
}

impl Runtime {
    /// Untrusted script never runs. The pref may exist; the runtime does not.
    pub fn eval_untrusted(&self, _src: &str) -> Result<Value> {
        let _ = self.enabled;
        Err(FrihartError::Message("js runtime not implemented".into()))
    }

    pub fn call_host(&self, api: HostApi) -> Result<Value> {
        let _ = self.enabled;
        Err(FrihartError::Message(api.reason().into()))
    }
}

/// Even if the user flipped the JS pref, we do not execute until campaign G
/// is a real engine. Fingerprint APIs stay denied after that too.
pub fn untrusted_eval_allowed() -> bool {
    false
}

/// Pref flip is not a capability grant.
pub fn host_allowed(pref_javascript: bool, api: HostApi) -> bool {
    let _ = pref_javascript;
    api.allowed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_even_when_enabled() {
        let rt = Runtime { enabled: true };
        assert!(rt.eval_untrusted("1+1").is_err());
        assert!(!untrusted_eval_allowed());
        assert!(HostApi::WebGpu.fingerprint());
        assert!(!HostApi::CanvasToDataUrl.allowed());
        assert!(rt.call_host(HostApi::Eval).is_err());
        assert!(rt.call_host(HostApi::Battery).is_err());
    }

    #[test]
    fn pref_does_not_open_host() {
        assert!(!host_allowed(true, HostApi::Cookie));
        assert!(!host_allowed(true, HostApi::LocalStorage));
        assert!(!host_allowed(true, HostApi::WebRtc));
        assert!(!host_allowed(true, HostApi::WebSocket));
        assert!(HostApi::Cookie.storage());
        assert!(HostApi::WebRtc.network());
        assert!(HostApi::ALL.iter().all(|a| !a.allowed()));
        assert!(
            HostApi::ALL
                .iter()
                .any(|a| matches!(a, HostApi::ServiceWorker))
        );
    }
}
