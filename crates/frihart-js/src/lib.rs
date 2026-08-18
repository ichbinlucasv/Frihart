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

/// Host APIs. Fingerprint surfaces stay denied even when a runtime exists.
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
}

impl HostApi {
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
        )
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
}
