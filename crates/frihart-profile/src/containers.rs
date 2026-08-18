use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use frihart_core::{ContainerId, FrihartError, Result};

/// A user-facing identity container. Tabs belong to one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: u32,
    pub name: String,
    pub slug: String,
    /// Packed 0x00RRGGBB used by chrome.
    pub color: u32,
}

impl Container {
    pub fn id(&self) -> ContainerId {
        ContainerId(self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerStore {
    pub items: Vec<Container>,
}

impl Default for ContainerStore {
    fn default() -> Self {
        Self::defaults()
    }
}

impl ContainerStore {
    pub fn defaults() -> Self {
        Self {
            items: vec![
                Container {
                    id: ContainerId::PERSONAL.0,
                    name: "Personal".into(),
                    slug: "personal".into(),
                    color: 0x00F5C400,
                },
                Container {
                    id: ContainerId::WORK.0,
                    name: "Work".into(),
                    slug: "work".into(),
                    color: 0x00E6B800,
                },
                Container {
                    id: ContainerId::BANKING.0,
                    name: "Banking".into(),
                    slug: "banking".into(),
                    color: 0x00C9A000,
                },
                Container {
                    id: ContainerId::SHOPPING.0,
                    name: "Shopping".into(),
                    slug: "shopping".into(),
                    color: 0x00A88800,
                },
            ],
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let store = Self::defaults();
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
        fs::write(path, text)?;
        Ok(())
    }

    pub fn get(&self, id: ContainerId) -> Option<&Container> {
        self.items.iter().find(|c| c.id == id.0)
    }

    pub fn by_slug(&self, slug: &str) -> Option<&Container> {
        self.items.iter().find(|c| c.slug == slug)
    }

    pub fn cycle(&self, current: ContainerId) -> ContainerId {
        if self.items.is_empty() {
            return current;
        }
        let pos = self
            .items
            .iter()
            .position(|c| c.id == current.0)
            .unwrap_or(0);
        let next = self.items[(pos + 1) % self.items.len()].id;
        ContainerId(next)
    }
}
