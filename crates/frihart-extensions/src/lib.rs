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
