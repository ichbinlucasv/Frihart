//! Privacy policy engine.
//!
//! Pure logic. No sockets, no files, no window handles. Chrome displays
//! decisions. Network and content obey them.

#![forbid(unsafe_code)]

use frihart_config::{Prefs, ReferrerPref, TimezonePref};

/// A request for a privilege. Every I/O-adjacent crate should ask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    OutboundHttp { https: bool },
    CookieWrite { third_party: bool },
    CookieRead { third_party: bool },
    Referrer { cross_site: bool },
    Storage { third_party: bool },
    Fingerprint(FingerprintSurface),
    JavaScript,
    WebRtc,
    ClientHints,
    Media,
    WebGl,
    Canvas,
}

/// High-entropy surfaces we treat as identifiers until proven otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FingerprintSurface {
    UserAgent,
    Language,
    Timezone,
    Screen,
    Fonts,
    Canvas,
    WebGl,
    Audio,
    WebRtc,
}

/// What the policy engine concluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny { reason: &'static str },
}

impl Decision {
    pub fn allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Self::Allow => None,
            Self::Deny { reason } => Some(*reason),
        }
    }
}

/// How much of a referrer may be sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferrerAllowance {
    None,
    Origin,
    Full,
}

/// Snapshot of the constitution, bound to the current prefs.
#[derive(Clone, Debug)]
pub struct Policy {
    prefs: Prefs,
}

impl Policy {
    pub fn new(prefs: Prefs) -> Self {
        Self { prefs }
    }

    pub fn from_prefs(prefs: &Prefs) -> Self {
        Self {
            prefs: prefs.clone(),
        }
    }

    pub fn prefs(&self) -> &Prefs {
        &self.prefs
    }

    pub fn decide(&self, kind: ResourceKind) -> Decision {
        match kind {
            ResourceKind::OutboundHttp { https } => {
                if !https && self.prefs.privacy.https_only {
                    Decision::Deny {
                        reason: "HTTPS-only mode is on",
                    }
                } else {
                    Decision::Allow
                }
            }
            ResourceKind::CookieWrite { third_party }
            | ResourceKind::CookieRead { third_party }
            | ResourceKind::Storage { third_party } => {
                if third_party && !self.prefs.privacy.third_party_cookies {
                    Decision::Deny {
                        reason: "third-party cookies and storage are blocked",
                    }
                } else {
                    Decision::Allow
                }
            }
            ResourceKind::Referrer { cross_site } => match self.prefs.privacy.send_referrer {
                ReferrerPref::Never => Decision::Deny {
                    reason: "referrers are disabled",
                },
                ReferrerPref::OriginOnly if cross_site => Decision::Deny {
                    reason: "cross-site referrers are omitted",
                },
                _ => Decision::Allow,
            },
            ResourceKind::JavaScript => {
                if self.prefs.privacy.javascript {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "JavaScript is off",
                    }
                }
            }
            ResourceKind::WebRtc => {
                if self.prefs.privacy.webrtc {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "WebRTC is off",
                    }
                }
            }
            ResourceKind::ClientHints => {
                if self.prefs.network.client_hints {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "client hints are disabled",
                    }
                }
            }
            ResourceKind::Media => {
                if self.prefs.content.media {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "media playback is off",
                    }
                }
            }
            ResourceKind::WebGl => {
                if self.prefs.content.webgl {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "WebGL is off",
                    }
                }
            }
            ResourceKind::Canvas => {
                if self.prefs.content.canvas {
                    Decision::Allow
                } else {
                    Decision::Deny {
                        reason: "canvas is off",
                    }
                }
            }
            ResourceKind::Fingerprint(surface) => self.decide_fingerprint(surface),
        }
    }

    fn decide_fingerprint(&self, surface: FingerprintSurface) -> Decision {
        if !self.prefs.privacy.resist_fingerprinting {
            return Decision::Allow;
        }
        match surface {
            FingerprintSurface::UserAgent | FingerprintSurface::Language => Decision::Allow,
            FingerprintSurface::Timezone => match self.prefs.privacy.timezone {
                TimezonePref::Utc => Decision::Deny {
                    reason: "timezone is clamped to UTC",
                },
                TimezonePref::System => Decision::Allow,
            },
            FingerprintSurface::Screen => Decision::Deny {
                reason: "screen metrics are limited to the window",
            },
            FingerprintSurface::Fonts => Decision::Deny {
                reason: "system font enumeration is denied",
            },
            FingerprintSurface::Canvas => Decision::Deny {
                reason: "canvas fingerprinting is denied",
            },
            FingerprintSurface::WebGl => Decision::Deny {
                reason: "WebGL fingerprinting is denied",
            },
            FingerprintSurface::Audio => Decision::Deny {
                reason: "audio fingerprinting is denied",
            },
            FingerprintSurface::WebRtc => Decision::Deny {
                reason: "WebRTC is denied",
            },
        }
    }

    pub fn referrer_allowance(&self, cross_site: bool) -> ReferrerAllowance {
        match self.prefs.privacy.send_referrer {
            ReferrerPref::Never => ReferrerAllowance::None,
            ReferrerPref::OriginOnly => {
                if cross_site {
                    ReferrerAllowance::None
                } else {
                    ReferrerAllowance::Origin
                }
            }
            ReferrerPref::OriginWhenCrossSite => {
                if cross_site {
                    ReferrerAllowance::Origin
                } else {
                    ReferrerAllowance::Full
                }
            }
        }
    }

    pub fn user_agent(&self) -> &str {
        &self.prefs.network.user_agent
    }

    pub fn send_gpc(&self) -> bool {
        self.prefs.privacy.send_gpc
    }

    pub fn send_dnt(&self) -> bool {
        self.prefs.privacy.send_dnt
    }

    pub fn https_only(&self) -> bool {
        self.prefs.privacy.https_only
    }

    pub fn javascript(&self) -> bool {
        self.prefs.privacy.javascript
    }

    /// Human-readable summary for `about:privacy`.
    pub fn summary_lines(&self) -> Vec<(&'static str, String, bool)> {
        vec![
            (
                "HTTPS-only",
                bool_label(self.prefs.privacy.https_only),
                self.prefs.privacy.https_only,
            ),
            (
                "Resist fingerprinting",
                bool_label(self.prefs.privacy.resist_fingerprinting),
                self.prefs.privacy.resist_fingerprinting,
            ),
            (
                "Third-party cookies",
                bool_label(self.prefs.privacy.third_party_cookies),
                !self.prefs.privacy.third_party_cookies,
            ),
            (
                "Partition first-party state",
                bool_label(self.prefs.privacy.partition_first_party_state),
                self.prefs.privacy.partition_first_party_state,
            ),
            (
                "Send Do Not Track",
                bool_label(self.prefs.privacy.send_dnt),
                !self.prefs.privacy.send_dnt,
            ),
            (
                "Send Global Privacy Control",
                bool_label(self.prefs.privacy.send_gpc),
                self.prefs.privacy.send_gpc,
            ),
            (
                "JavaScript",
                bool_label(self.prefs.privacy.javascript),
                !self.prefs.privacy.javascript,
            ),
            (
                "WebRTC",
                bool_label(self.prefs.privacy.webrtc),
                !self.prefs.privacy.webrtc,
            ),
            (
                "Client Hints",
                bool_label(self.prefs.network.client_hints),
                !self.prefs.network.client_hints,
            ),
            (
                "Native blocker",
                bool_label(self.prefs.privacy.blocker),
                self.prefs.privacy.blocker,
            ),
            (
                "Containers",
                bool_label(self.prefs.privacy.containers),
                self.prefs.privacy.containers,
            ),
        ]
    }
}

fn bool_label(v: bool) -> String {
    if v { "on".into() } else { "off".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frihart_config::Prefs;

    fn policy() -> Policy {
        Policy::new(Prefs::default())
    }

    #[test]
    fn https_only_blocks_cleartext() {
        let p = policy();
        assert!(
            !p.decide(ResourceKind::OutboundHttp { https: false })
                .allowed()
        );
        assert!(
            p.decide(ResourceKind::OutboundHttp { https: true })
                .allowed()
        );
    }

    #[test]
    fn third_party_cookies_blocked() {
        let p = policy();
        assert!(
            !p.decide(ResourceKind::CookieWrite { third_party: true })
                .allowed()
        );
        assert!(
            p.decide(ResourceKind::CookieWrite { third_party: false })
                .allowed()
        );
    }

    #[test]
    fn javascript_and_webrtc_off() {
        let p = policy();
        assert!(!p.decide(ResourceKind::JavaScript).allowed());
        assert!(!p.decide(ResourceKind::WebRtc).allowed());
        assert!(!p.decide(ResourceKind::ClientHints).allowed());
    }

    #[test]
    fn dnt_off_gpc_on() {
        let p = policy();
        assert!(!p.send_dnt());
        assert!(p.send_gpc());
    }

    #[test]
    fn cross_site_referrer_dropped() {
        let p = policy();
        assert_eq!(p.referrer_allowance(true), ReferrerAllowance::None);
        assert_eq!(p.referrer_allowance(false), ReferrerAllowance::Origin);
    }

    #[test]
    fn fingerprint_surfaces_denied_by_default() {
        let p = policy();
        assert!(
            !p.decide(ResourceKind::Fingerprint(FingerprintSurface::Canvas))
                .allowed()
        );
        assert!(
            !p.decide(ResourceKind::Fingerprint(FingerprintSurface::Fonts))
                .allowed()
        );
        assert!(
            !p.decide(ResourceKind::Fingerprint(FingerprintSurface::WebGl))
                .allowed()
        );
    }
}
