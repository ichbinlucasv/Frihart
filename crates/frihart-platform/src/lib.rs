//! Operating-system seams. Linux is the reference implementation.
//!
//! Other platforms return `Unsupported` until their roadmap phase.

#![forbid(unsafe_code)]

use std::env;
use std::path::{Path, PathBuf};

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

/// User downloads directory. Never execute what lands here.
pub fn downloads_dir() -> PathBuf {
    if let Ok(value) = env::var("XDG_DOWNLOAD_DIR") {
        if !value.is_empty() {
            return PathBuf::from(value);
        }
    }
    home_dir().join("Downloads")
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

/// Sandbox hooks. Phase 1 records intent; Phase 6 applies.
#[derive(Clone, Debug, Default)]
pub struct SandboxSpec {
    pub enabled: bool,
    pub seccomp: bool,
    pub landlock: bool,
    pub no_new_privs: bool,
}

impl SandboxSpec {
    pub fn content_default() -> Self {
        Self {
            enabled: true,
            seccomp: true,
            landlock: true,
            no_new_privs: true,
        }
    }

    pub fn apply(&self) -> Result<()> {
        let _ = (self.enabled, self.seccomp, self.landlock, self.no_new_privs);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
    Macos,
    Android,
    Other,
}

pub fn current_os() -> Os {
    match std::env::consts::OS {
        "linux" => Os::Linux,
        "windows" => Os::Windows,
        "macos" => Os::Macos,
        "android" => Os::Android,
        _ => Os::Other,
    }
}

pub fn profile_root_for(os: Os) -> PathBuf {
    match os {
        Os::Linux => data_dir(),
        Os::Windows => home_dir().join("AppData/Local/Frihart"),
        Os::Macos => home_dir().join("Library/Application Support/Frihart"),
        Os::Android => data_dir(),
        Os::Other => data_dir(),
    }
}

pub fn is_linux() -> bool {
    matches!(current_os(), Os::Linux)
}

/// Look up an executable on PATH. No subprocess.
pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// How we expect to talk to a local Tor daemon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TorPresence {
    pub binary: Option<PathBuf>,
    pub socks_hint: String,
}

pub fn detect_tor() -> TorPresence {
    TorPresence {
        binary: find_in_path("tor"),
        socks_hint: "127.0.0.1:9050".into(),
    }
}

/// Official VPN CLIs we are willing to hook, never to reimplement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnPresence {
    pub proton: Option<PathBuf>,
    pub mullvad: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassManager {
    pub id: &'static str,
    pub name: &'static str,
    pub path: PathBuf,
}

pub fn detect_pass_managers() -> Vec<PassManager> {
    const CANDIDATES: &[(&str, &str)] = &[
        ("proton-pass", "Proton Pass"),
        ("protonpass", "Proton Pass"),
        ("keepassxc", "KeePassXC"),
        ("bitwarden", "Bitwarden"),
        ("bw", "Bitwarden CLI"),
        ("pass", "pass"),
    ];
    let mut out = Vec::new();
    for (bin, name) in CANDIDATES {
        if let Some(path) = find_in_path(bin) {
            out.push(PassManager {
                id: bin,
                name,
                path,
            });
        }
    }
    out
}

pub fn launch_local(path: &Path) -> Result<()> {
    std::process::Command::new(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(frihart_core::FrihartError::from)
}

pub fn detect_vpn() -> VpnPresence {
    VpnPresence {
        proton: find_in_path("protonvpn-cli")
            .or_else(|| find_in_path("protonvpn"))
            .or_else(|| find_in_path("proton-vpn")),
        mullvad: find_in_path("mullvad"),
    }
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

    #[test]
    fn profile_roots_are_os_specific() {
        assert!(profile_root_for(Os::Linux).ends_with("frihart"));
        assert!(
            profile_root_for(Os::Windows)
                .to_string_lossy()
                .contains("Frihart")
        );
        assert!(
            profile_root_for(Os::Macos)
                .to_string_lossy()
                .contains("Frihart")
        );
        assert!(downloads_dir().ends_with("Downloads") || env::var("XDG_DOWNLOAD_DIR").is_ok());
        assert!(SandboxSpec::content_default().enabled);
    }
}
