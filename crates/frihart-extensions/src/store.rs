//! Installed add-ons in the profile.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use frihart_core::{FrihartError, Result, write_private_str};

use crate::compat::{self, ApiSupport};
use crate::manifest::Manifest;
use crate::xpi;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledAddon {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub enabled: bool,
    /// Always `dormant` until the JS engine exists.
    pub run_state: String,
    pub manifest_version: u32,
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl InstalledAddon {
    pub fn from_manifest(manifest: &Manifest) -> Self {
        Self {
            id: manifest.addon_id(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description: manifest.description.clone().unwrap_or_default(),
            enabled: true,
            run_state: "dormant".into(),
            manifest_version: manifest.manifest_version,
            permissions: manifest.all_permissions(),
        }
    }

    pub fn permission_report(&self) -> Vec<(String, ApiSupport)> {
        self.permissions
            .iter()
            .map(|p| (p.clone(), compat::classify(p)))
            .collect()
    }

    pub fn dormant_reason(&self) -> &'static str {
        compat::dormant_reason(true)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonStore {
    #[serde(default)]
    pub items: Vec<InstalledAddon>,
}

impl AddonStore {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| FrihartError::profile(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text =
            toml::to_string_pretty(self).map_err(|e| FrihartError::profile(e.to_string()))?;
        write_private_str(path, &text)
    }

    pub fn get(&self, id: &str) -> Option<&InstalledAddon> {
        self.items.iter().find(|a| a.id == id)
    }

    /// Sideload an `.xpi` or unpacked directory into `extensions_dir`.
    pub fn install(&mut self, source: &Path, extensions_dir: &Path) -> Result<InstalledAddon> {
        let manifest = xpi::read_manifest_from_path(source)?;
        let addon = InstalledAddon::from_manifest(&manifest);
        let dest = extensions_dir.join(safe_dir(&addon.id));
        xpi::materialize(source, &dest)?;
        self.items.retain(|a| a.id != addon.id);
        self.items.push(addon.clone());
        Ok(addon)
    }

    pub fn uninstall(&mut self, id: &str, extensions_dir: &Path) -> Result<()> {
        self.items.retain(|a| a.id != id);
        let dest = extensions_dir.join(safe_dir(id));
        if dest.exists() {
            fs::remove_dir_all(dest)?;
        }
        Ok(())
    }

    pub fn dir(profile_root: &Path) -> PathBuf {
        profile_root.join("extensions")
    }

    pub fn registry_path(profile_root: &Path) -> PathBuf {
        profile_root.join("addons.toml")
    }
}

fn safe_dir(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_unpacked() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("frihart-addon-{stamp}"));
        let src = root.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("manifest.json"),
            r#"{
                "manifest_version": 2,
                "name": "Demo",
                "version": "0.1.0",
                "browser_specific_settings": { "gecko": { "id": "demo@frihart" } },
                "permissions": ["storage"]
            }"#,
        )
        .unwrap();
        let ext = root.join("extensions");
        let mut store = AddonStore::default();
        let installed = store.install(&src, &ext).unwrap();
        assert_eq!(installed.id, "demo@frihart");
        assert_eq!(installed.run_state, "dormant");
        assert!(ext.join("demo_frihart").join("manifest.json").exists());
        store.uninstall("demo@frihart", &ext).unwrap();
        assert!(store.items.is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
