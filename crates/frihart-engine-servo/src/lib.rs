//! Servo Compat engine adapter (docs/ENGINE_HYBRID.md, campaign S).
//!
//! Without the `servo` feature this crate links no Servo code and
//! [`ServoEngine::render_html`] reports `Unavailable`. With it, Servo 0.7
//! renders the page off screen (CPU, no window) and the frame comes back as
//! one [`frihart_gfx::DisplayOp::Image`], which the chrome blits like any
//! other display list.
//!
//! In every build this engine refuses Tor and I2P tabs.

#![forbid(unsafe_code)]

pub mod prefs;

#[cfg(feature = "servo")]
mod headless;

use frihart_engine::{Engine, EngineError, EngineKind, Rendered, TabNetwork};

/// True when this build was compiled with the `servo` feature.
pub const AVAILABLE: bool = cfg!(feature = "servo");

/// Default off-screen viewport height for a Compat render, in pixels.
pub const DEFAULT_VIEWPORT_H: u32 = 768;

/// Largest viewport side accepted, in pixels (bounds the frame buffer).
pub const MAX_VIEWPORT_SIDE: u32 = 8192;

#[derive(Default)]
pub struct ServoEngine {
    #[cfg(feature = "servo")]
    inner: Option<headless::Headless>,
}

impl ServoEngine {
    /// Clamp a requested viewport to what the adapter accepts.
    pub fn clamp_viewport(w: f32, h: u32) -> (u32, u32) {
        let w = if w.is_finite() { w.round() as i64 } else { 0 };
        let w = w.clamp(1, MAX_VIEWPORT_SIDE as i64) as u32;
        (w, h.clamp(1, MAX_VIEWPORT_SIDE))
    }
}

impl Engine for ServoEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Servo
    }
    fn allowed_on(&self, net: TabNetwork) -> bool {
        // Servo must never handle Tor/I2P until proven not to leak.
        net == TabNetwork::Clearnet
    }
    #[cfg(not(feature = "servo"))]
    fn render_html(&mut self, _html: &str, _viewport_w: f32) -> Result<Rendered, EngineError> {
        Err(EngineError::Unavailable(
            "this build has no Servo Compat engine (cargo feature `servo`)",
        ))
    }
    #[cfg(feature = "servo")]
    fn render_html(&mut self, html: &str, viewport_w: f32) -> Result<Rendered, EngineError> {
        let (w, h) = Self::clamp_viewport(viewport_w, DEFAULT_VIEWPORT_H);
        if self.inner.is_none() {
            self.inner = Some(headless::Headless::new(w, h)?);
        }
        let frame = self.inner.as_mut().expect("just set").render(html, w, h)?;
        let mut display = frihart_gfx::DisplayList::default();
        display.ops.push(frihart_gfx::DisplayOp::Image {
            x: 0.0,
            y: 0.0,
            width: frame.width,
            height: frame.height,
            rgba: frame.rgba,
        });
        Ok(Rendered {
            title: frame.title,
            display,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_tor_and_i2p() {
        let e = ServoEngine::default();
        assert!(!e.allowed_on(TabNetwork::Tor));
        assert!(!e.allowed_on(TabNetwork::I2p));
        assert!(e.allowed_on(TabNetwork::Clearnet));
    }

    #[test]
    fn viewport_is_clamped() {
        assert_eq!(ServoEngine::clamp_viewport(800.4, 600), (800, 600));
        assert_eq!(ServoEngine::clamp_viewport(f32::NAN, 0), (1, 1));
        assert_eq!(
            ServoEngine::clamp_viewport(1e9, u32::MAX),
            (MAX_VIEWPORT_SIDE, MAX_VIEWPORT_SIDE)
        );
    }

    #[cfg(not(feature = "servo"))]
    #[test]
    fn default_build_reports_unavailable() {
        let mut e = ServoEngine::default();
        assert!(matches!(
            e.render_html("<p>x</p>", 800.0),
            Err(EngineError::Unavailable(_))
        ));
        const { assert!(!AVAILABLE) };
    }

    /// One test drives Servo end to end: Servo keeps per-process state
    /// (SpiderMonkey), so a single engine renders twice.
    #[cfg(feature = "servo")]
    #[test]
    fn servo_renders_grid_and_runs_js_twice() {
        let mut e = ServoEngine::default();
        let html = "<!doctype html><title>Compat</title><style>body{margin:0;background:#000}\
            main{display:grid;grid-template-columns:1fr 1fr}div{height:40px;background:#ffd700}\
            </style><main><div></div><div id=b></div></main>\
            <script>document.title='JS '+(6*7);document.getElementById('b').style.background='#00f'</script>";
        let r = e.render_html(html, 200.0).unwrap();
        assert_eq!(r.title, "JS 42");
        let frihart_gfx::DisplayOp::Image {
            width,
            height,
            rgba,
            ..
        } = &r.display.ops[0]
        else {
            panic!("expected an image op");
        };
        assert_eq!((*width, *height), (200, DEFAULT_VIEWPORT_H));
        assert!(r.display.ops[0].image_is_well_formed());
        let px = |x: usize, y: usize| {
            let i = (y * *width as usize + x) * 4;
            (rgba[i], rgba[i + 1], rgba[i + 2])
        };
        assert_eq!(px(10, 10), (0xff, 0xd7, 0x00), "left grid cell is gold");
        assert_eq!(px(150, 10), (0, 0, 0xff), "JS turned the right cell blue");
        assert_eq!(px(10, 100), (0, 0, 0), "below the grid is the body");

        let r2 = e
            .render_html("<title>second</title><body style=background:#fff>", 120.0)
            .unwrap();
        assert_eq!(r2.title, "second");

        // A subresource must go to the dead proxy, never straight to its host.
        let target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let port = target.local_addr().unwrap().port();
        let html = format!("<title>leak</title><img src=\"http://127.0.0.1:{port}/x.png\">");
        let r3 = e.render_html(&html, 120.0).unwrap();
        assert_eq!(r3.title, "leak");
        assert!(
            target.accept().is_err(),
            "Servo connected to the image host directly"
        );
    }
}
