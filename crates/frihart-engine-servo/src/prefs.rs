//! Privacy settings applied to every Servo instance Frihart creates.
//!
//! Kept free of Servo types so the policy is tested in the default build;
//! `headless` maps it onto `servo::Preferences` when the feature is on.

/// Proxy nothing listens on (TCP discard port on loopback). Until M3 routes
/// Servo through `frihart-net`, every HTTP(S) load Servo attempts is sent
/// here and fails closed instead of leaving the machine.
pub const DEAD_PROXY: &str = "http://127.0.0.1:9";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardenedPrefs {
    pub user_agent: String,
    pub http_proxy: String,
    pub https_proxy: String,
    pub devtools_server: bool,
    pub webrtc: bool,
    pub geolocation: bool,
    pub bluetooth: bool,
    pub gamepad: bool,
    pub webgpu: bool,
    pub notifications: bool,
    pub service_workers: bool,
    /// JavaScript for this Compat frame. Default false (fail closed).
    /// Servo 0.7 has no master "disable scripts" preference; when false the
    /// adapter strips `<script>` from HTML and turns off Wasm. When true,
    /// scripts may run (clearnet Compat only).
    pub javascript: bool,
}

impl Default for HardenedPrefs {
    fn default() -> Self {
        Self {
            user_agent: frihart_core::FROZEN_USER_AGENT.to_string(),
            http_proxy: DEAD_PROXY.to_string(),
            https_proxy: DEAD_PROXY.to_string(),
            devtools_server: false,
            webrtc: false,
            geolocation: false,
            bluetooth: false,
            gamepad: false,
            webgpu: false,
            notifications: false,
            service_workers: false,
            javascript: false,
        }
    }
}

impl HardenedPrefs {
    /// Leak surfaces other than the intentional per-site JS opt-in are off
    /// and every load goes to a loopback proxy.
    pub fn leak_surfaces_locked(&self) -> bool {
        let loopback =
            |p: &str| p.starts_with("http://127.0.0.1:") || p.starts_with("http://[::1]:");
        loopback(&self.http_proxy)
            && loopback(&self.https_proxy)
            && !self.devtools_server
            && !self.webrtc
            && !self.geolocation
            && !self.bluetooth
            && !self.gamepad
            && !self.webgpu
            && !self.notifications
            && !self.service_workers
            && self.user_agent == frihart_core::FROZEN_USER_AGENT
    }

    /// Fail-closed baseline: leak surfaces locked **and** JavaScript off.
    pub fn is_locked_down(&self) -> bool {
        self.leak_surfaces_locked() && !self.javascript
    }
}

/// Remove `<script>` elements (case-insensitive) so Servo cannot run them
/// when `HardenedPrefs.javascript` is false. Best-effort on the HTML string
/// Frihart already fetched; not a full HTML sanitizer. Tag scans are ASCII
/// so byte indices stay valid; body text is copied as UTF-8 slices.
pub fn strip_scripts(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while let Some(rel) = lower[i..].find("<script") {
        let start = i + rel;
        out.push_str(&html[i..start]);
        let Some(rel_gt) = lower[start..].find('>') else {
            return out;
        };
        let open_end = start + rel_gt + 1;
        let open = &lower[start..open_end];
        if open.trim_end().ends_with("/>") {
            i = open_end;
            continue;
        }
        if let Some(rel_close) = lower[open_end..].find("</script") {
            let close_at = open_end + rel_close;
            if let Some(rel_gt2) = lower[close_at..].find('>') {
                i = close_at + rel_gt2 + 1;
                continue;
            }
        }
        // Malformed: drop the rest.
        return out;
    }
    out.push_str(&html[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_locked_down() {
        assert!(HardenedPrefs::default().is_locked_down());
        assert!(!HardenedPrefs::default().javascript);
        let open = HardenedPrefs {
            webrtc: true,
            ..HardenedPrefs::default()
        };
        assert!(!open.is_locked_down());
        let direct = HardenedPrefs {
            https_proxy: String::new(),
            ..HardenedPrefs::default()
        };
        assert!(
            !direct.is_locked_down(),
            "empty proxy means direct in Servo"
        );
    }

    #[test]
    fn javascript_on_breaks_full_lockdown_keeps_leak_lock() {
        let js = HardenedPrefs {
            javascript: true,
            ..HardenedPrefs::default()
        };
        assert!(!js.is_locked_down());
        assert!(js.leak_surfaces_locked());
    }

    #[test]
    fn strip_scripts_removes_inline_and_src() {
        let html = "<title>t</title><script>document.title='x'</script><p>hi</p>\
            <SCRIPT src=x.js></SCRIPT><script src='y.js'/>";
        let out = strip_scripts(html);
        assert!(!out.to_ascii_lowercase().contains("<script"));
        assert!(out.contains("<p>hi</p>"));
        assert!(out.contains("<title>t</title>"));
    }

    #[test]
    fn strip_scripts_preserves_noscript_text() {
        let html = "<noscript>enable JS</noscript><p>ok</p>";
        assert_eq!(strip_scripts(html), html);
    }
}
