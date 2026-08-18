use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use frihart_core::{FrihartError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkStore {
    pub items: Vec<Bookmark>,
}

impl BookmarkStore {
    pub fn defaults() -> Self {
        Self {
            items: vec![
                Bookmark {
                    title: "Home".into(),
                    url: "about:home".into(),
                },
                Bookmark {
                    title: "Settings".into(),
                    url: "about:settings".into(),
                },
                Bookmark {
                    title: "Privacy".into(),
                    url: "about:privacy".into(),
                },
                Bookmark {
                    title: "Containers".into(),
                    url: "about:containers".into(),
                },
                Bookmark {
                    title: "Blocker".into(),
                    url: "about:blocker".into(),
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

    pub fn add(&mut self, title: impl Into<String>, url: impl Into<String>) {
        self.items.push(Bookmark {
            title: title.into(),
            url: url.into(),
        });
    }
}
