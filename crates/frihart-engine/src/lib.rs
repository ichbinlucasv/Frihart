//! Engine abstraction for the hybrid design (docs/ENGINE_HYBRID.md).
//!
//! Every tab is rendered by exactly one [`Engine`]. Frihart's own engine
//! ([`OwnEngine`]) is the only engine allowed on Tor, I2P and reader tabs.
//! The Servo Compat engine lives in `frihart-engine-servo` behind the
//! `servo` cargo feature and is off by default.

#![forbid(unsafe_code)]

use frihart_gfx::DisplayList;

/// Which network a tab is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabNetwork {
    Clearnet,
    Tor,
    I2p,
}

/// What the user asked the tab to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabMode {
    /// Reader / document mode: always Frihart's own engine.
    Reader,
    /// Compat mode: modern sites with JS (Servo), clearnet only.
    Compat,
}

/// Stable engine identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    Own,
    Servo,
}

/// A rendered page as the chrome sees it.
#[derive(Clone, Debug)]
pub struct Rendered {
    pub title: String,
    pub display: DisplayList,
}

/// Errors any engine may report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    /// The engine is not compiled into this build.
    Unavailable(&'static str),
    /// The engine refused for a privacy reason (fail closed).
    Refused(&'static str),
    Failed(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Unavailable(s) => write!(f, "engine unavailable: {s}"),
            EngineError::Refused(s) => write!(f, "refused: {s}"),
            EngineError::Failed(s) => write!(f, "engine failed: {s}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// One document engine. Both engines implement this.
pub trait Engine {
    fn kind(&self) -> EngineKind;
    /// May this engine ever handle a tab on `net`?
    fn allowed_on(&self, net: TabNetwork) -> bool;
    /// Render already-fetched HTML. Networking stays in `frihart-net`.
    fn render_html(&mut self, html: &str, viewport_w: f32) -> Result<Rendered, EngineError>;
}

/// Frihart's own Rust engine (HTML → CSS → layout → display list).
#[derive(Default)]
pub struct OwnEngine {
    pub extra_css: String,
}

impl Engine for OwnEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Own
    }
    fn allowed_on(&self, _net: TabNetwork) -> bool {
        true
    }
    fn render_html(&mut self, html: &str, viewport_w: f32) -> Result<Rendered, EngineError> {
        let f = frihart_pipeline::layout_html(html, &self.extra_css, viewport_w);
        Ok(Rendered {
            title: f.title,
            display: f.display,
        })
    }
}

/// Per-tab engine selection. Fail closed: Tor/I2P and Reader always get
/// the own engine; Compat goes to Servo only on clearnet and only when it
/// is compiled in.
///
/// `own_replaces_compat` is the swap Lucas locked on 2026-10-21: Servo and
/// Boa ship the browser sooner, and the day Frihart's own Rust engine covers
/// the same job this flag retires Servo. Tor and I2P were already own-engine
/// only.
pub fn select_engine(
    net: TabNetwork,
    mode: TabMode,
    servo_available: bool,
    own_replaces_compat: bool,
) -> EngineKind {
    if own_replaces_compat {
        return EngineKind::Own;
    }
    match (net, mode) {
        (TabNetwork::Clearnet, TabMode::Compat) if servo_available => EngineKind::Servo,
        _ => EngineKind::Own,
    }
}

/// Mode for a newly opened tab (Campaign S M5).
///
/// Tor and I2P tabs are always Reader. A clearnet tab is Compat only when
/// the user turned `compat_default` on, Servo is compiled in, and Frihart's
/// own engine does not claim the site (claimed documents render well, with
/// no script and a smaller attack surface, in Reader).
pub fn default_tab_mode(
    net: TabNetwork,
    compat_default: bool,
    servo_available: bool,
    claimed_by_own_engine: bool,
    own_replaces_compat: bool,
) -> TabMode {
    if own_replaces_compat {
        return TabMode::Reader;
    }
    match net {
        TabNetwork::Clearnet if compat_default && servo_available && !claimed_by_own_engine => {
            TabMode::Compat
        }
        _ => TabMode::Reader,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_reader_unless_every_condition_holds() {
        assert_eq!(
            default_tab_mode(TabNetwork::Clearnet, true, true, false, false),
            TabMode::Compat
        );
        assert_eq!(
            default_tab_mode(TabNetwork::Clearnet, true, true, false, true),
            TabMode::Reader
        );
        for (c, s, k) in [
            (false, true, false),
            (true, false, false),
            (true, true, true),
            (false, false, true),
        ] {
            assert_eq!(
                default_tab_mode(TabNetwork::Clearnet, c, s, k, false),
                TabMode::Reader
            );
        }
        for net in [TabNetwork::Tor, TabNetwork::I2p] {
            assert_eq!(
                default_tab_mode(net, true, true, false, false),
                TabMode::Reader
            );
        }
    }

    #[test]
    fn tor_and_i2p_never_get_servo() {
        for net in [TabNetwork::Tor, TabNetwork::I2p] {
            for mode in [TabMode::Reader, TabMode::Compat] {
                assert_eq!(select_engine(net, mode, true, false), EngineKind::Own);
            }
        }
    }

    #[test]
    fn own_engine_swap_retires_servo() {
        assert_eq!(
            select_engine(TabNetwork::Clearnet, TabMode::Compat, true, true),
            EngineKind::Own
        );
        assert_eq!(
            select_engine(TabNetwork::Clearnet, TabMode::Compat, true, false),
            EngineKind::Servo
        );
    }

    #[test]
    fn compat_clearnet_uses_servo_only_when_built() {
        assert_eq!(
            select_engine(TabNetwork::Clearnet, TabMode::Compat, true, false),
            EngineKind::Servo
        );
        assert_eq!(
            select_engine(TabNetwork::Clearnet, TabMode::Compat, false, false),
            EngineKind::Own
        );
        assert_eq!(
            select_engine(TabNetwork::Clearnet, TabMode::Reader, true, false),
            EngineKind::Own
        );
    }

    #[test]
    fn own_engine_renders_title() {
        let mut e = OwnEngine::default();
        let r = e.render_html("<title>Hi</title><p>x</p>", 800.0).unwrap();
        assert_eq!(r.title, "Hi");
        assert!(e.allowed_on(TabNetwork::Tor));
    }
}
