//! Per-site (first-party host) JavaScript overrides for Compat / Servo.
//!
//! Absence of a host key inherits the global `privacy.javascript` pref
//! (default false). Persist under the profile as `site_js.toml` (0600).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use frihart_core::{FrihartError, Result, write_private_str};

/// Map of first-party host → whether JavaScript is allowed for that host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteJsStore {
    /// Lowercase host keys only (no port, no trailing dot).
    #[serde(default)]
    pub hosts: BTreeMap<String, bool>,
}

impl SiteJsStore {
    fn key(host: &str) -> String {
        host.trim().trim_end_matches('.').to_ascii_lowercase()
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let store = Self::default();
            store.save(path)?;
            return Ok(store);
        }
        let text = fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| FrihartError::profile(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|e| FrihartError::profile(e.to_string()))?;
        write_private_str(path, &text)
    }

    /// Explicit override for `host`, if any.
    pub fn override_for(&self, host: &str) -> Option<bool> {
        let k = Self::key(host);
        if k.is_empty() || k == "-" {
            return None;
        }
        self.hosts.get(&k).copied()
    }

    /// Effective allow: override if set, else `global_default`.
    pub fn allowed(&self, host: &str, global_default: bool) -> bool {
        self.override_for(host).unwrap_or(global_default)
    }

    /// Set an explicit override (always stored, even when equal to global).
    pub fn set(&mut self, host: &str, allowed: bool) {
        let k = Self::key(host);
        if k.is_empty() || k == "-" {
            return;
        }
        self.hosts.insert(k, allowed);
    }

    pub fn clear(&mut self, host: &str) -> bool {
        self.hosts.remove(&Self::key(host)).is_some()
    }

    pub fn clear_all(&mut self) {
        self.hosts.clear();
    }

    /// Flip the effective value for `host` and persist the new override.
    /// Returns the new effective allow.
    pub fn toggle(&mut self, host: &str, global_default: bool) -> bool {
        let next = !self.allowed(host, global_default);
        self.set(host, next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn inherits_global_when_unset() {
        let s = SiteJsStore::default();
        assert!(!s.allowed("example.com", false));
        assert!(s.allowed("example.com", true));
        assert_eq!(s.override_for("example.com"), None);
    }

    #[test]
    fn override_beats_global() {
        let mut s = SiteJsStore::default();
        s.set("Example.COM.", true);
        assert!(s.allowed("example.com", false));
        assert_eq!(s.override_for("example.com"), Some(true));
        s.set("example.com", false);
        assert!(!s.allowed("example.com", true));
    }

    #[test]
    fn toggle_writes_override() {
        let mut s = SiteJsStore::default();
        assert!(s.toggle("a.test", false));
        assert_eq!(s.override_for("a.test"), Some(true));
        assert!(!s.toggle("a.test", false));
        assert_eq!(s.override_for("a.test"), Some(false));
    }

    #[test]
    fn roundtrip_disk() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        // Own subdirectory so write_private can chmod 0700 (not the shared temp root).
        let dir = std::env::temp_dir().join(format!("frihart-site-js-{stamp}"));
        frihart_core::ensure_private_dir(&dir).unwrap();
        let path = dir.join("site_js.toml");
        {
            let mut s = SiteJsStore::default();
            s.set("docs.example", true);
            s.save(&path).unwrap();
        }
        {
            let s = SiteJsStore::load(&path).unwrap();
            assert_eq!(s.override_for("docs.example"), Some(true));
            assert!(!s.allowed("other.example", false));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_host_ignored() {
        let mut s = SiteJsStore::default();
        s.set("", true);
        s.set("-", true);
        assert!(s.hosts.is_empty());
    }
}
