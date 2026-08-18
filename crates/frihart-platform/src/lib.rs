//! Operating-system seams. Linux is the reference implementation.
//!
//! Other platforms return `Unsupported` until their roadmap phase.

#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;

use frihart_core::{APP_NAME, Result};

/// XDG-style data directory: `$XDG_DATA_HOME/frihart`.
pub fn data_dir() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", ".local/share").join("frihart")
}

/// XDG-style config directory: `$XDG_CONFIG_HOME/frihart`.
pub fn config_dir() -> PathBuf {
    xdg_dir("XDG_CONFIG_HOME", ".config").join("frihart")
}

/// XDG-style cache directory: `$XDG_CACHE_HOME/frihart`.
pub fn cache_dir() -> PathBuf {
    xdg_dir("XDG_CACHE_HOME", ".cache").join("frihart")
}

/// Directory that holds named profiles.
pub fn profiles_dir() -> PathBuf {
    data_dir().join("profiles")
}

fn xdg_dir(var: &str, fallback: &str) -> PathBuf {
    if let Ok(value) = env::var(var) {
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    home_dir().join(fallback)
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

/// Window title for a page.
pub fn window_title(page_title: &str) -> String {
    if page_title.is_empty() {
        APP_NAME.to_string()
    } else {
        format!("{page_title} — {APP_NAME}")
    }
}

/// Sandbox hooks. Phase 1 is a no-op that records intent.
#[derive(Clone, Debug, Default)]
pub struct SandboxSpec {
    pub enabled: bool,
}

impl SandboxSpec {
    pub fn apply(&self) -> Result<()> {
        let _ = self.enabled;
        // Phase 6: seccomp-bpf, landlock, no_new_privs.
        Ok(())
    }
}

/// True when this build is the Linux reference.
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirs_contain_frihart() {
        assert!(data_dir().ends_with("frihart"));
        assert!(config_dir().ends_with("frihart"));
        assert!(cache_dir().ends_with("frihart"));
    }

    #[test]
    fn title_format() {
        assert_eq!(window_title(""), "Frihart");
        assert_eq!(window_title("Home"), "Home — Frihart");
    }
}
