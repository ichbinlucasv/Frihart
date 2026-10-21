//! Frihart's own engine.
//!
//! This folder is the engine Lucas keeps beside Servo and Boa. The browser
//! does not select it. Servo (clearnet Compat) and Boa (light JS, feature
//! `boa`) are how the product ships. When this engine can do that job,
//! `content.own_replaces_compat` retires Servo and this crate takes the tab.
//!
//! [`READY_TO_REPLACE_COMPAT`] stays false on purpose. Layout today goes
//! through the existing pipeline (the reader engine). Script today is the
//! arithmetic slice in [`script`], which is ours and is not JavaScript.

#![forbid(unsafe_code)]

mod script;

pub use script::{ScriptError, eval as eval_script};

/// False until this engine can replace Servo on clearnet Compat tabs.
pub const READY_TO_REPLACE_COMPAT: bool = false;

/// The future engine. Not wired into the chrome.
#[derive(Clone, Debug, Default)]
pub struct Engine {
    pub extra_css: String,
}

impl Engine {
    pub fn layout_html(&self, html: &str, viewport_w: f32) -> frihart_pipeline::Frame {
        frihart_pipeline::layout_html(html, &self.extra_css, viewport_w)
    }

    /// Our script slice. Callers must not treat a successful result as JavaScript.
    pub fn eval_script(&self, src: &str) -> Result<f64, ScriptError> {
        let _ = self;
        script::eval(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_selected_and_lays_out_a_title() {
        // The flag is a const on purpose. This test fails the build the day
        // someone flips it without also changing the assertion.
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(!READY_TO_REPLACE_COMPAT);
        }
        let frame = Engine::default().layout_html("<title>Own</title><p>x</p>", 800.0);
        assert_eq!(frame.title, "Own");
        assert!(!frame.display.ops.is_empty());
    }

    #[test]
    fn script_slice_runs_in_this_crate() {
        let engine = Engine::default();
        assert_eq!(engine.eval_script("(10-4)/2").unwrap(), 3.0);
    }
}
