//! Firefox-style WebExtensions manifest (MV2 / MV3).

use serde::Deserialize;

use frihart_core::{FrihartError, Result};

/// Enough of `manifest.json` to install and audit an add-on.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_mv")]
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_permissions: Vec<String>,
    #[serde(default)]
    pub host_permissions: Vec<String>,
    #[serde(default)]
    pub background: Option<Background>,
    #[serde(default)]
    pub content_scripts: Vec<ContentScript>,
    #[serde(default)]
    pub browser_specific_settings: Option<BrowserSpecific>,
    #[serde(default)]
    pub applications: Option<BrowserSpecific>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Background {
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default)]
    pub persistent: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContentScript {
    #[serde(default)]
    pub matches: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub css: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BrowserSpecific {
    #[serde(default)]
    pub gecko: Option<Gecko>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Gecko {
    #[serde(default)]
    pub id: Option<String>,
}

fn default_mv() -> u32 {
    2
}

impl Manifest {
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|e| FrihartError::config(format!("manifest.json: {e}")))
    }

    /// Gecko add-on id, or a slug derived from the name.
    pub fn addon_id(&self) -> String {
        if let Some(id) = self
            .browser_specific_settings
            .as_ref()
            .and_then(|b| b.gecko.as_ref())
            .and_then(|g| g.id.clone())
        {
            return id;
        }
        if let Some(id) = self
            .applications
            .as_ref()
            .and_then(|b| b.gecko.as_ref())
            .and_then(|g| g.id.clone())
        {
            return id;
        }
        slug(&self.name)
    }

    pub fn all_permissions(&self) -> Vec<String> {
        let mut out = self.permissions.clone();
        out.extend(self.optional_permissions.iter().cloned());
        out.extend(self.host_permissions.iter().cloned());
        out.sort();
        out.dedup();
        out
    }

    pub fn needs_javascript(&self) -> bool {
        self.background
            .as_ref()
            .is_some_and(|b| !b.scripts.is_empty() || b.service_worker.is_some())
            || self.content_scripts.iter().any(|c| !c.js.is_empty())
    }
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "addon".into() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ublock_shaped_manifest() {
        let json = r#"{
            "manifest_version": 2,
            "name": "uBlock Origin",
            "version": "1.63.0",
            "browser_specific_settings": { "gecko": { "id": "uBlock0@raymondhill.net" } },
            "permissions": ["storage", "tabs", "webRequest", "webRequestBlocking", "<all_urls>"],
            "background": { "scripts": ["js/background.js"] }
        }"#;
        let m = Manifest::from_json(json).unwrap();
        assert_eq!(m.addon_id(), "uBlock0@raymondhill.net");
        assert!(m.needs_javascript());
        assert!(m.all_permissions().iter().any(|p| p == "webRequest"));
    }
}
