//! WebExtensions host. Install and audit only. No JS execution.

#![forbid(unsafe_code)]

mod compat;
mod manifest;
mod store;
mod xpi;

pub use compat::{ApiSupport, classify, dormant_reason};
pub use manifest::Manifest;
pub use store::{AddonStore, InstalledAddon};
pub use xpi::{materialize, read_manifest_from_path};

/// Execute sideloaded add-ons. Off until JS + isolation exist.
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
