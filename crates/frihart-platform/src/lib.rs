//! Operating-system seams. Linux is the reference implementation.
//!
//! Other platforms return `Unsupported` until their roadmap phase.

#![deny(unsafe_code)]

mod sandbox;

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

pub use sandbox::{SandboxReport, SandboxSpec, landlock_abi, rlimit_names, seccomp_denies};

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

/// Which Linux home we think we are on. Used for OPSEC hints, not branding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinuxHome {
    Arch,
    Cachyos,
    Fedora,
    Mint,
    Debian,
    Tails,
    Qubes,
    Other(String),
}

impl LinuxHome {
    pub fn label(&self) -> String {
        match self {
            Self::Arch => "Arch Linux".into(),
            Self::Cachyos => "CachyOS".into(),
            Self::Fedora => "Fedora".into(),
            Self::Mint => "Linux Mint".into(),
            Self::Debian => "Debian".into(),
            Self::Tails => "Tails".into(),
            Self::Qubes => "Qubes OS".into(),
            Self::Other(id) => {
                if id.is_empty() {
                    "Linux".into()
                } else {
                    id.clone()
                }
            }
        }
    }

    pub fn prefer_ephemeral(&self) -> bool {
        matches!(self, Self::Tails) || (matches!(self, Self::Qubes) && is_qubes_disposable())
    }

    pub fn tor_is_the_network(&self) -> bool {
        matches!(self, Self::Tails)
    }
}

pub fn detect_linux_home() -> LinuxHome {
    if Path::new("/usr/share/qubes").is_dir() || Path::new("/etc/qubes-rpc").exists() {
        return LinuxHome::Qubes;
    }
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    parse_os_release(&text)
}

pub fn parse_os_release(text: &str) -> LinuxHome {
    let id = os_kv(text, "ID");
    let like = os_kv(text, "ID_LIKE");
    let name = os_kv(text, "NAME").to_ascii_lowercase();
    if id == "tails" || name.contains("tails") {
        return LinuxHome::Tails;
    }
    if id == "qubes" || name.contains("qubes") {
        return LinuxHome::Qubes;
    }
    if id == "cachyos" || name.contains("cachy") {
        return LinuxHome::Cachyos;
    }
    if id == "arch" || id == "manjaro" || id == "endeavouros" || like.contains("arch") {
        return LinuxHome::Arch;
    }
    if id == "fedora" {
        return LinuxHome::Fedora;
    }
    if id == "linuxmint" || id == "mint" {
        return LinuxHome::Mint;
    }
    if id == "debian" || like.contains("debian") {
        return LinuxHome::Debian;
    }
    LinuxHome::Other(id)
}

fn os_kv(text: &str, key: &str) -> String {
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        return v.trim().trim_matches('"').to_ascii_lowercase();
    }
    String::new()
}

pub fn is_qubes_disposable() -> bool {
    Path::new("/run/qubes/this-is-dvm").exists() || env::var_os("QUBES_DVM").is_some()
}

/// Tails / Qubes-DVM: open memory-only unless the user passed `--profile`.
pub fn should_open_ephemeral(private_flag: bool, explicit_profile: bool) -> bool {
    ephemeral_decision(
        private_flag,
        explicit_profile,
        detect_linux_home().prefer_ephemeral(),
    )
}

pub fn ephemeral_decision(private_flag: bool, explicit_profile: bool, prefer: bool) -> bool {
    if explicit_profile {
        return false;
    }
    private_flag || prefer
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

    #[test]
    fn os_release_homes() {
        assert_eq!(
            parse_os_release("ID=tails\nNAME=\"Tails\"\n"),
            LinuxHome::Tails
        );
        assert_eq!(parse_os_release("ID=cachyos\n"), LinuxHome::Cachyos);
        assert_eq!(parse_os_release("ID=linuxmint\n"), LinuxHome::Mint);
        assert_eq!(parse_os_release("ID=fedora\n"), LinuxHome::Fedora);
        assert_eq!(parse_os_release("ID=arch\n"), LinuxHome::Arch);
        assert_eq!(
            parse_os_release("ID=manjaro\nID_LIKE=arch\n"),
            LinuxHome::Arch
        );
        assert_eq!(
            parse_os_release("ID=endeavouros\nID_LIKE=arch\n"),
            LinuxHome::Arch
        );
        assert!(LinuxHome::Tails.tor_is_the_network());
        assert!(LinuxHome::Tails.prefer_ephemeral());
        assert!(ephemeral_decision(false, false, true));
        assert!(!ephemeral_decision(false, true, true));
        assert!(ephemeral_decision(true, false, false));
    }
}
