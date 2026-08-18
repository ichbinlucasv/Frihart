//! WebExtensions compatibility host.
//!
//! Frihart is not a Firefox fork. It can still *read* Firefox add-ons
//! (`.xpi` / unpacked `manifest.json`) and, over time, implement the
//! `browser.*` APIs on its own engine. Installed add-ons are dormant
//! until Phase 7 (JS). That is honest, not a fake runtime.

#![forbid(unsafe_code)]

mod compat;
mod manifest;
mod store;
mod xpi;

pub use compat::{ApiSupport, classify, dormant_reason};
pub use manifest::Manifest;
pub use store::{AddonStore, InstalledAddon};
pub use xpi::{materialize, read_manifest_from_path};

/// Phase 15: execute sideloaded add-ons. Off until JS + isolation exist.
#[derive(Clone, Debug, Default)]
pub struct Runtime {
    pub enabled: bool,
}

impl Runtime {
    pub fn execute(&self, addon: &InstalledAddon) -> frihart_core::Result<()> {
        let _ = addon;
        let _ = self.enabled;
        Err(frihart_core::FrihartError::Message(
            dormant_reason(true).into(),
        ))
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[test]
    fn runtime_stays_dormant() {
        let addon = InstalledAddon {
            id: "x".into(),
            name: "x".into(),
            version: "0".into(),
            description: String::new(),
            enabled: true,
            run_state: "dormant".into(),
            manifest_version: 2,
            permissions: vec![],
        };
        assert!(Runtime::default().execute(&addon).is_err());
    }
}
