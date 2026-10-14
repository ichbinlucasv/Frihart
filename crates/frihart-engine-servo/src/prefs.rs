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
        }
    }
}

impl HardenedPrefs {
    /// Every switch that opens a fingerprinting or leak surface is off and
    /// every load goes to a loopback proxy.
    pub fn is_locked_down(&self) -> bool {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_locked_down() {
        assert!(HardenedPrefs::default().is_locked_down());
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
}
