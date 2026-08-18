//! Typed preferences. Defaults are the product.
//!
//! Changing a default requires a matching edit of `docs/defaults.md`.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use frihart_core::{FROZEN_USER_AGENT, FrihartError, Result, write_private_str};

/// How much of a referrer Frihart is willing to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReferrerPref {
    /// Never send `Referer`.
    Never,
    /// Same-origin: origin only. Cross-site: omit. This is the default.
    #[default]
    OriginOnly,
    /// Full URL on same origin. Cross-site: origin only.
    OriginWhenCrossSite,
}

/// Timezone strategy for fingerprint resistance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TimezonePref {
    #[default]
    Utc,
    System,
}

/// Top-level preference file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Prefs {
    pub general: GeneralPrefs,
    pub privacy: PrivacyPrefs,
    pub network: NetworkPrefs,
    pub content: ContentPrefs,
    pub translate: TranslatePrefs,
    pub search: SearchPrefs,
    pub tor: TorPrefs,
    pub vpn: VpnPrefs,
    pub extensions: ExtensionPrefs,
    pub support: SupportPrefs,
    pub pass: PassPrefs,
    pub autofill: AutofillPrefs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralPrefs {
    pub homepage: String,
    pub new_tab_url: String,
    /// Legacy override. Empty means "use `search.primary`."
    pub search_url: String,
    pub restore_session: bool,
    pub show_status_bar: bool,
    pub welcome_seen: bool,
}

impl Default for GeneralPrefs {
    fn default() -> Self {
        Self {
            homepage: "about:home".into(),
            new_tab_url: "about:newtab".into(),
            search_url: String::new(),
            restore_session: false,
            show_status_bar: true,
            welcome_seen: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyPrefs {
    pub https_only: bool,
    pub resist_fingerprinting: bool,
    pub third_party_cookies: bool,
    pub partition_first_party_state: bool,
    pub send_referrer: ReferrerPref,
    pub send_dnt: bool,
    pub send_gpc: bool,
    pub persist_history: bool,
    pub persist_cookies: bool,
    pub webrtc: bool,
    pub javascript: bool,
    pub timezone: TimezonePref,
    pub language: String,
    /// Native identity containers (LibreWolf/Firefox Multi-Account inspired).
    pub containers: bool,
    /// Native uBlock-inspired blocker. On by default. Not an add-on.
    pub blocker: bool,
    /// Always false. Frihart does not collect or store logins.
    pub store_logins: bool,
}

impl Default for PrivacyPrefs {
    fn default() -> Self {
        Self {
            https_only: true,
            resist_fingerprinting: true,
            third_party_cookies: false,
            partition_first_party_state: true,
            send_referrer: ReferrerPref::OriginOnly,
            send_dnt: false,
            send_gpc: true,
            persist_history: true,
            persist_cookies: true,
            webrtc: false,
            javascript: false,
            timezone: TimezonePref::Utc,
            language: "en".into(),
            containers: true,
            blocker: true,
            store_logins: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkPrefs {
    pub user_agent: String,
    pub client_hints: bool,
    /// `off`, `optional`, or `always`. No vendor is configured.
    pub doh_mode: String,
    pub doh_url: String,
    pub http2: bool,
    pub http3: bool,
}

impl Default for NetworkPrefs {
    fn default() -> Self {
        Self {
            user_agent: FROZEN_USER_AGENT.into(),
            client_hints: false,
            doh_mode: "off".into(),
            doh_url: String::new(),
            http2: true,
            http3: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentPrefs {
    pub images: bool,
    pub media: bool,
    pub webgl: bool,
    pub canvas: bool,
    /// Dark pages and chrome. The product look is black and yellow.
    pub dark_mode: bool,
}

impl Default for ContentPrefs {
    fn default() -> Self {
        Self {
            images: true,
            media: false,
            webgl: false,
            canvas: false,
            dark_mode: true,
        }
    }
}

/// Built-in translator. DeepL is the product default. No Google.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranslatePrefs {
    pub enabled: bool,
    /// `deepl` or `libretranslate`.
    pub provider: String,
    /// DeepL API root. Free-tier default. A key is still required.
    pub endpoint: String,
    /// DeepL auth key, stored only in the local profile. Never uploaded.
    pub api_key: String,
    pub source: String,
    pub target: String,
}

impl Default for TranslatePrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "deepl".into(),
            endpoint: "https://api-free.deepl.com/v2/translate".into(),
            api_key: String::new(),
            source: "auto".into(),
            target: "en".into(),
        }
    }
}

/// Search engines. Swisscows first, DuckDuckGo second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchPrefs {
    pub primary: String,
    pub secondary: String,
}

impl Default for SearchPrefs {
    fn default() -> Self {
        Self {
            primary: "swisscows".into(),
            secondary: "duckduckgo".into(),
        }
    }
}

/// System Tor daemon. Frihart does not bundle the Tor network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TorPrefs {
    pub enabled: bool,
    pub socks_host: String,
    pub socks_port: u16,
}

impl Default for TorPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            socks_host: "127.0.0.1".into(),
            socks_port: 9050,
        }
    }
}

/// Official-client hooks. Frihart is not a VPN vendor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VpnPrefs {
    /// `none`, `proton`, or `mullvad`.
    pub provider: String,
}

impl Default for VpnPrefs {
    fn default() -> Self {
        Self {
            provider: "none".into(),
        }
    }
}

/// WebExtensions compatibility host. Sideload only. No AMO fetch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtensionPrefs {
    pub enabled: bool,
}

impl Default for ExtensionPrefs {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutofillPrefs {
    pub enabled: bool,
}

impl Default for AutofillPrefs {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// External password manager. Frihart never stores logins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PassPrefs {
    /// `none`, `proton-pass`, `keepassxc`, `bitwarden`, or `pass`.
    pub manager: String,
}

/// Local-only funding. Addresses are never fetched. Fill them yourself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SupportPrefs {
    pub xmr: String,
    pub btc: String,
    /// Optional HTTPS page you host for card/SEPA. Empty = none.
    pub fiat_url: String,
}

impl Prefs {
    /// Load from TOML. Missing fields take defaults. A missing file is
    /// not an error — the caller gets `Prefs::default()`.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        Self::from_toml(&text)
    }

    pub fn from_toml(text: &str) -> Result<Self> {
        let mut prefs: Self =
            toml::from_str(text).map_err(|e| FrihartError::config(e.to_string()))?;
        prefs.privacy.store_logins = false;
        Ok(prefs)
    }

    pub fn to_toml(&self) -> Result<String> {
        let header = "# Frihart preferences. See docs/defaults.md.\n\
                      # There is no telemetry section. Do not add one.\n\n";
        let body = toml::to_string_pretty(self).map_err(|e| FrihartError::config(e.to_string()))?;
        Ok(format!("{header}{body}"))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = self.to_toml()?;
        write_private_str(path, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sovereign() {
        let p = Prefs::default();
        assert!(p.privacy.https_only);
        assert!(p.privacy.resist_fingerprinting);
        assert!(!p.privacy.third_party_cookies);
        assert!(!p.privacy.send_dnt);
        assert!(p.privacy.send_gpc);
        assert!(!p.privacy.javascript);
        assert!(!p.privacy.webrtc);
        assert!(!p.network.client_hints);
        assert_eq!(p.network.doh_mode, "off");
        assert!(p.network.doh_url.is_empty());
        assert!(p.general.search_url.is_empty());
        assert_eq!(p.search.primary, "swisscows");
        assert_eq!(p.search.secondary, "duckduckgo");
        assert!(!p.general.restore_session);
        assert!(!p.content.webgl);
        assert!(!p.content.canvas);
        assert!(p.content.dark_mode);
        assert!(p.privacy.blocker);
        assert!(p.privacy.containers);
        assert!(p.translate.enabled);
        assert_eq!(p.translate.provider, "deepl");
        assert!(p.translate.api_key.is_empty());
        assert!(p.tor.enabled);
        assert_eq!(p.tor.socks_port, 9050);
        assert_eq!(p.vpn.provider, "none");
        assert!(p.extensions.enabled);
        assert!(!p.privacy.store_logins);
        assert!(!p.general.welcome_seen);
    }

    #[test]
    fn roundtrip() {
        let original = Prefs::default();
        let text = original.to_toml().unwrap();
        let parsed = Prefs::from_toml(&text).unwrap();
        assert_eq!(original, parsed);
        assert!(text.contains("There is no telemetry section"));
    }

    #[test]
    fn missing_fields_default() {
        let parsed = Prefs::from_toml("[privacy]\nhttps_only = false\n").unwrap();
        assert!(!parsed.privacy.https_only);
        assert!(parsed.privacy.resist_fingerprinting);
        assert_eq!(parsed.general.homepage, "about:home");
    }
}
