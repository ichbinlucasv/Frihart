//! On-disk and ephemeral user profiles.

#![forbid(unsafe_code)]

mod bookmarks;
mod containers;
mod history;
mod lock;

use std::fs;
use std::path::{Path, PathBuf};

use frihart_config::Prefs;
use frihart_core::{ContainerId, DEFAULT_PROFILE_NAME, Result};
use frihart_platform::profiles_dir;

pub use bookmarks::{Bookmark, BookmarkStore};
pub use containers::{Container, ContainerStore};
pub use frihart_extensions::{AddonStore, InstalledAddon};
pub use history::{HistoryEntry, HistoryStore};

use lock::ProfileLock;

/// A user profile: prefs, bookmarks, history, and a lock.
pub struct Profile {
    root: PathBuf,
    name: String,
    ephemeral: bool,
    prefs: Prefs,
    bookmarks: BookmarkStore,
    history: HistoryStore,
    containers: ContainerStore,
    addons: AddonStore,
    _lock: Option<ProfileLock>,
}

impl Profile {
    /// Open the named default profile under the XDG data directory.
    pub fn open_default() -> Result<Self> {
        Self::open_named(DEFAULT_PROFILE_NAME)
    }

    pub fn open_named(name: &str) -> Result<Self> {
        let root = profiles_dir().join(sanitize_name(name));
        Self::open_path(root, name, false)
    }

    /// Open a specific directory as a profile (CLI `--profile`).
    pub fn open_dir(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("custom")
            .to_string();
        Self::open_path(root, &name, false)
    }

    /// Private window: nothing is written to disk.
    pub fn ephemeral() -> Result<Self> {
        let mut profile = Self {
            root: PathBuf::from("/dev/null"),
            name: "private".into(),
            ephemeral: true,
            prefs: Prefs::default(),
            bookmarks: BookmarkStore::defaults(),
            history: HistoryStore::default(),
            containers: ContainerStore::defaults(),
            addons: AddonStore::default(),
            _lock: None,
        };
        profile.prefs.privacy.persist_history = false;
        profile.prefs.privacy.persist_cookies = false;
        Ok(profile)
    }

    fn open_path(root: PathBuf, name: &str, ephemeral: bool) -> Result<Self> {
        if !ephemeral {
            fs::create_dir_all(&root)?;
        }
        let lock = if ephemeral {
            None
        } else {
            Some(ProfileLock::acquire(root.join("lock"))?)
        };
        let prefs = if ephemeral {
            Prefs::default()
        } else {
            Prefs::load(&root.join("prefs.toml"))?
        };
        let bookmarks = if ephemeral {
            BookmarkStore::defaults()
        } else {
            BookmarkStore::load(&root.join("bookmarks.toml"))?
        };
        let history = if ephemeral {
            HistoryStore::default()
        } else {
            HistoryStore::load(&root.join("history.jsonl"))?
        };
        let containers = if ephemeral {
            ContainerStore::defaults()
        } else {
            ContainerStore::load(&root.join("containers.toml"))?
        };
        let addons = if ephemeral {
            AddonStore::default()
        } else {
            AddonStore::load(&AddonStore::registry_path(&root))?
        };
        Ok(Self {
            root,
            name: name.to_string(),
            ephemeral,
            prefs,
            bookmarks,
            history,
            containers,
            addons,
            _lock: lock,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_ephemeral(&self) -> bool {
        self.ephemeral
    }

    pub fn prefs(&self) -> &Prefs {
        &self.prefs
    }

    pub fn prefs_mut(&mut self) -> &mut Prefs {
        &mut self.prefs
    }

    pub fn save_prefs(&self) -> Result<()> {
        if self.ephemeral {
            return Ok(());
        }
        self.prefs.save(&self.root.join("prefs.toml"))
    }

    pub fn bookmarks(&self) -> &BookmarkStore {
        &self.bookmarks
    }

    pub fn bookmarks_mut(&mut self) -> &mut BookmarkStore {
        &mut self.bookmarks
    }

    pub fn save_bookmarks(&self) -> Result<()> {
        if self.ephemeral {
            return Ok(());
        }
        self.bookmarks.save(&self.root.join("bookmarks.toml"))
    }

    pub fn history(&self) -> &HistoryStore {
        &self.history
    }

    pub fn containers(&self) -> &ContainerStore {
        &self.containers
    }

    pub fn container(&self, id: ContainerId) -> Option<&Container> {
        self.containers.get(id)
    }

    pub fn addons(&self) -> &AddonStore {
        &self.addons
    }

    pub fn install_addon(&mut self, source: &Path) -> Result<frihart_extensions::InstalledAddon> {
        if self.ephemeral {
            return Err(frihart_core::FrihartError::profile(
                "private windows cannot install add-ons",
            ));
        }
        let ext_dir = AddonStore::dir(&self.root);
        let addon = self.addons.install(source, &ext_dir)?;
        self.addons.save(&AddonStore::registry_path(&self.root))?;
        Ok(addon)
    }

    pub fn record_visit(&mut self, url: &str, title: &str) -> Result<()> {
        if self.ephemeral || !self.prefs.privacy.persist_history {
            return Ok(());
        }
        self.history.record(url, title);
        self.history.save(&self.root.join("history.jsonl"))
    }

    pub fn clear_history(&mut self) -> Result<()> {
        self.history.clear();
        if self.ephemeral {
            return Ok(());
        }
        self.history.save(&self.root.join("history.jsonl"))
    }
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        DEFAULT_PROFILE_NAME.into()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ephemeral_does_not_persist() {
        let mut p = Profile::ephemeral().unwrap();
        assert!(p.is_ephemeral());
        p.prefs_mut().privacy.https_only = false;
        p.save_prefs().unwrap();
        p.record_visit("about:home", "Home").unwrap();
        assert!(p.history().is_empty());
    }

    #[test]
    fn disk_profile_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("frihart-test-profile-{stamp}"));
        {
            let mut p = Profile::open_dir(&root).unwrap();
            p.prefs_mut().privacy.javascript = true;
            p.save_prefs().unwrap();
            p.record_visit("about:home", "Home").unwrap();
        }
        {
            let p = Profile::open_dir(&root).unwrap();
            assert!(p.prefs().privacy.javascript);
            assert_eq!(p.history().len(), 1);
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sanitize_rejects_path_escape() {
        assert_eq!(sanitize_name("../etc"), "___etc");
    }
}
