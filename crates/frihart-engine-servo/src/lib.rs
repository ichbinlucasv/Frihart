//! Servo Compat engine adapter (docs/ENGINE_HYBRID.md, campaign S).
//!
//! Status: M1 skeleton. No Servo code is linked yet. With or without the
//! `servo` feature this engine refuses Tor/I2P tabs, always.

#![forbid(unsafe_code)]

use frihart_engine::{Engine, EngineError, EngineKind, Rendered, TabNetwork};

/// True when this build was compiled with the `servo` feature.
pub const AVAILABLE: bool = cfg!(feature = "servo");

#[derive(Default)]
pub struct ServoEngine;

impl Engine for ServoEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Servo
    }
    fn allowed_on(&self, net: TabNetwork) -> bool {
        // Servo must never handle Tor/I2P until proven not to leak.
        net == TabNetwork::Clearnet
    }
    fn render_html(&mut self, _html: &str, _viewport_w: f32) -> Result<Rendered, EngineError> {
        Err(EngineError::Unavailable(
            "Servo Compat engine is not linked yet (M1)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_tor_and_i2p() {
        let e = ServoEngine;
        assert!(!e.allowed_on(TabNetwork::Tor));
        assert!(!e.allowed_on(TabNetwork::I2p));
        assert!(e.allowed_on(TabNetwork::Clearnet));
    }
}
