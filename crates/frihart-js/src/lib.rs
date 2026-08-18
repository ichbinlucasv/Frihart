//! JS types. Execution stays off until Phase 7 is real.

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

/// Host APIs. Fingerprint surfaces stay denied even when JS exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostApi {
    DomRead,
    DomWrite,
    Fetch,
    Timer,
    Console,
    CanvasToDataUrl,
    WebGl,
    AudioContext,
    Eval,
    Wasm,
}

impl HostApi {
    pub fn allowed(self) -> bool {
        let _ = self;
        false
    }

    pub fn reason(self) -> &'static str {
        match self {
            Self::CanvasToDataUrl | Self::WebGl | Self::AudioContext => "fingerprint surface",
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
    pub fn eval_untrusted(&self, _src: &str) -> Result<Value> {
        Err(FrihartError::Message("js off".into()))
    }

    pub fn call_host(&self, api: HostApi) -> Result<Value> {
        let _ = self.enabled;
        Err(FrihartError::Message(api.reason().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_by_default() {
        let rt = Runtime::default();
        assert!(rt.eval_untrusted("1+1").is_err());
        assert!(!HostApi::CanvasToDataUrl.allowed());
        assert!(rt.call_host(HostApi::Eval).is_err());
    }
}
