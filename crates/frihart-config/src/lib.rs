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
    pub i2p: I2pPrefs,
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
    pub strip_tracking: bool,
    pub search_suggestions: bool,
    pub prefetch: bool,
    pub speculative_connect: bool,
    pub send_pings: bool,
    pub geolocation: bool,
    pub wasm: bool,
    pub drm: bool,
    pub telemetry: bool,
    pub safe_browsing: bool,
    pub clear_on_shutdown: bool,
    pub punycode_hosts: bool,
    pub letterboxing: bool,
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
            strip_tracking: true,
            search_suggestions: false,
            prefetch: false,
            speculative_connect: false,
            send_pings: false,
            geolocation: false,
            wasm: false,
            drm: false,
            telemetry: false,
            safe_browsing: false,
            clear_on_shutdown: false,
            punycode_hosts: true,
            letterboxing: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkPrefs {
    pub user_agent: String,
    pub client_hints: bool,
    /// Clearnet DNS. `quad9` (default), `custom` (`doh_url`), or `system`
    /// (the operating-system resolver, opt-in). Legacy `off` loads as
    /// `quad9`. Legacy `always` / `optional` with a URL load as `custom`.
    pub doh_mode: String,
    /// DoH URL. Quad9 when `doh_mode` is `quad9`. Used as written in
    /// `custom` mode. Not used in `system` mode.
    pub doh_url: String,
    /// When true, a failed encrypted lookup may use the OS resolver.
    /// Default false. A reply from the encrypted resolver, including a
    /// block or NXDOMAIN, does not fall through. Tor and I2P tabs ignore
    /// this and never use local DNS.
    pub dns_system_fallback: bool,
    pub http2: bool,
    pub http3: bool,
    pub captive_portal: bool,
    pub connectivity_check: bool,
    pub tls_0rtt: bool,
}

impl Default for NetworkPrefs {
    fn default() -> Self {
        Self {
            user_agent: FROZEN_USER_AGENT.into(),
            client_hints: false,
            doh_mode: "quad9".into(),
            doh_url: QUAD9_DOH_URL.into(),
            dns_system_fallback: false,
            http2: true,
            http3: false,
            captive_portal: false,
            connectivity_check: false,
            tls_0rtt: false,
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
    pub autoplay: bool,
    /// Campaign S M5: open new clearnet tabs in Compat (Servo) when Frihart is
    /// built with the `servo` feature. Off until the live-page leak test
    /// passes. Tor and I2P tabs never use Compat, and sites Frihart's own
    /// engine claims stay in Reader.
    pub compat_default: bool,
}

impl Default for ContentPrefs {
    fn default() -> Self {
        Self {
            images: true,
            media: false,
            webgl: false,
            canvas: false,
            dark_mode: true,
            autoplay: false,
            compat_default: false,
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
    /// Your SearXNG search URL with `{q}`. Empty = not configured.
    pub searxng: String,
}

impl Default for SearchPrefs {
    fn default() -> Self {
        Self {
            primary: "swisscows".into(),
            secondary: "duckduckgo".into(),
            searxng: String::new(),
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
    /// When true, Tor tabs stay closed until Tor reports bridges active
    /// ([`frihart_net::bridges::allow_tor_fetch`]). Default off.
    pub require_bridges: bool,
    /// Tor ControlPort on `socks_host`, used only for the read-only bridge
    /// check (`frihart_net::tor_control`). Default 9051.
    pub control_port: u16,
}

impl Default for TorPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            socks_host: "127.0.0.1".into(),
            socks_port: 9050,
            require_bridges: false,
            control_port: 9051,
        }
    }
}

/// System I2P daemon (`i2pd` or Java I2P). Frihart does not bundle the net.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct I2pPrefs {
    pub enabled: bool,
    pub socks_host: String,
    pub socks_port: u16,
}

impl Default for I2pPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            socks_host: "127.0.0.1".into(),
            socks_port: 4447,
        }
    }
}

/// Quad9 DNS-over-HTTPS. Default clearnet resolver. The client connects
/// to Quad9's published addresses, so this name is not looked up first.
pub const QUAD9_DOH_URL: &str = "https://dns.quad9.net/dns-query";
/// Quad9 recursive IPv4 (malware and C2 blocking, DNSSEC). One of the
/// addresses used to reach [`QUAD9_DOH_URL`] without the system resolver.
pub const QUAD9_DNS_V4: &str = "9.9.9.9";

/// How a clearnet tab resolves names. Tor and I2P tabs do not use this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClearnetDns {
    /// Encrypted DNS. `system_fallback` is false unless the user opted in.
    Doh { url: String, system_fallback: bool },
    /// The user opted into the operating-system resolver.
    System,
}

impl Default for ClearnetDns {
    fn default() -> Self {
        Self::quad9()
    }
}

impl ClearnetDns {
    /// Quad9 DoH, and no system resolver.
    pub fn quad9() -> Self {
        Self::Doh {
            url: QUAD9_DOH_URL.into(),
            system_fallback: false,
        }
    }

    /// Plan for the current preferences. Unknown modes use Quad9.
    pub fn from_prefs(prefs: &Prefs) -> Self {
        let mode = prefs.network.doh_mode.trim();
        let fallback = prefs.network.dns_system_fallback;
        if mode.eq_ignore_ascii_case("system") {
            return Self::System;
        }
        if mode.eq_ignore_ascii_case("custom") {
            let url = prefs.network.doh_url.trim();
            if !url.is_empty() {
                return Self::Doh {
                    url: url.to_string(),
                    system_fallback: fallback,
                };
            }
        }
        Self::Doh {
            url: QUAD9_DOH_URL.into(),
            system_fallback: fallback,
        }
    }

    /// True when a clearnet lookup may call the operating system.
    /// False for the Quad9 default.
    pub fn allows_system_resolver(&self) -> bool {
        match self {
            Self::System => true,
            Self::Doh {
                system_fallback, ..
            } => *system_fallback,
        }
    }

    pub fn doh_url(&self) -> Option<&str> {
        match self {
            Self::System => None,
            Self::Doh { url, .. } => Some(url.as_str()),
        }
    }
}

/// Map saved preferences onto the current DNS modes.
///
/// `off` was the old product default (system DNS, DoH not wired). It now
/// loads as Quad9. Choosing the system resolver is `doh_mode = "system"`.
fn normalize_dns(prefs: &mut Prefs) {
    let mode = prefs.network.doh_mode.trim().to_ascii_lowercase();
    let url = prefs.network.doh_url.trim().to_string();
    match mode.as_str() {
        "system" => {
            prefs.network.doh_mode = "system".into();
        }
        "custom" | "always" | "optional" => {
            if url.is_empty() {
                prefs.network.doh_mode = "quad9".into();
                prefs.network.doh_url = QUAD9_DOH_URL.into();
            } else {
                prefs.network.doh_mode = "custom".into();
                prefs.network.doh_url = url;
            }
        }
        "quad9" => {
            prefs.network.doh_mode = "quad9".into();
            prefs.network.doh_url = QUAD9_DOH_URL.into();
        }
        _ => {
            prefs.network.doh_mode = "quad9".into();
            prefs.network.doh_url = QUAD9_DOH_URL.into();
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
        prefs.privacy.telemetry = false;
        prefs.privacy.safe_browsing = false;
        prefs.privacy.drm = false;
        normalize_dns(&mut prefs);
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
        assert_eq!(p.network.doh_mode, "quad9");
        assert_eq!(p.network.doh_url, QUAD9_DOH_URL);
        assert!(!p.network.dns_system_fallback);
        let dns = ClearnetDns::from_prefs(&p);
        assert_eq!(dns, ClearnetDns::quad9());
        assert!(!dns.allows_system_resolver());
        assert_eq!(dns.doh_url(), Some(QUAD9_DOH_URL));
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
        assert!(!p.tor.require_bridges);
        assert_eq!(p.tor.control_port, 9051);
        assert!(p.i2p.enabled);
        assert_eq!(p.i2p.socks_port, 4447);
        assert_eq!(p.vpn.provider, "none");
        assert!(p.extensions.enabled);
        assert!(!p.privacy.store_logins);
        assert!(!p.general.welcome_seen);
        assert!(p.privacy.letterboxing);
        assert!(p.privacy.strip_tracking);
        assert!(!p.privacy.search_suggestions);
        assert!(!p.privacy.prefetch);
        assert!(!p.privacy.speculative_connect);
        assert!(!p.privacy.telemetry);
        assert!(!p.privacy.safe_browsing);
        assert!(!p.privacy.drm);
        assert!(!p.network.captive_portal);
        assert!(!p.content.autoplay);
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
        assert_eq!(parsed.network.doh_mode, "quad9");
        assert!(!parsed.network.dns_system_fallback);
        assert!(!ClearnetDns::from_prefs(&parsed).allows_system_resolver());
    }

    #[test]
    fn old_doh_off_loads_as_quad9_without_system_fallback() {
        let parsed = Prefs::from_toml(
            "[network]\ndoh_mode = \"off\"\ndoh_url = \"\"\ndns_system_fallback = false\n",
        )
        .unwrap();
        assert_eq!(parsed.network.doh_mode, "quad9");
        assert_eq!(parsed.network.doh_url, QUAD9_DOH_URL);
        assert!(!parsed.network.dns_system_fallback);
        assert_eq!(ClearnetDns::from_prefs(&parsed), ClearnetDns::quad9());
    }

    #[test]
    fn system_resolver_and_fallback_are_opt_in() {
        let system = Prefs::from_toml("[network]\ndoh_mode = \"system\"\n").unwrap();
        assert_eq!(system.network.doh_mode, "system");
        assert!(ClearnetDns::from_prefs(&system).allows_system_resolver());
        assert!(ClearnetDns::from_prefs(&system).doh_url().is_none());

        let mut fallback = Prefs::default();
        fallback.network.dns_system_fallback = true;
        let plan = ClearnetDns::from_prefs(&fallback);
        assert!(plan.allows_system_resolver());
        assert_eq!(plan.doh_url(), Some(QUAD9_DOH_URL));

        let custom = Prefs::from_toml(
            "[network]\ndoh_mode = \"always\"\ndoh_url = \"https://dns.example/dns-query\"\n",
        )
        .unwrap();
        assert_eq!(custom.network.doh_mode, "custom");
        assert_eq!(
            ClearnetDns::from_prefs(&custom).doh_url(),
            Some("https://dns.example/dns-query")
        );
        assert!(!ClearnetDns::from_prefs(&custom).allows_system_resolver());
    }
}
