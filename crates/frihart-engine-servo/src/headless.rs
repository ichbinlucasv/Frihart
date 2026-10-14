//! Off-screen Servo: one `Servo` and one CPU rendering surface per engine,
//! one short-lived `WebView` per render.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use frihart_engine::EngineError;
use servo::{
    EventLoopWaker, LoadStatus, Preferences, RenderingContext, RgbaImage, Servo, ServoBuilder,
    SoftwareRenderingContext, WebView, WebViewBuilder, WebViewDelegate,
};

use crate::prefs::HardenedPrefs;

/// Longest a single render may take before it is abandoned.
const RENDER_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct Frame {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Servo calls this from its threads; the caller polls, so nothing to do.
struct PollWaker;
impl EventLoopWaker for PollWaker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(PollWaker)
    }
    fn wake(&self) {}
}

#[derive(Default)]
struct Watch {
    complete: Cell<bool>,
}
impl WebViewDelegate for Watch {
    fn notify_load_status_changed(&self, _: WebView, status: LoadStatus) {
        if status == LoadStatus::Complete {
            self.complete.set(true);
        }
    }
    fn notify_new_frame_ready(&self, webview: WebView) {
        webview.paint();
    }
}

fn to_servo_prefs(p: &HardenedPrefs) -> Preferences {
    Preferences {
        user_agent: p.user_agent.clone(),
        network_http_proxy_uri: p.http_proxy.clone(),
        network_https_proxy_uri: p.https_proxy.clone(),
        devtools_server_enabled: p.devtools_server,
        dom_webrtc_enabled: p.webrtc,
        dom_geolocation_enabled: p.geolocation,
        dom_bluetooth_enabled: p.bluetooth,
        dom_gamepad_enabled: p.gamepad,
        dom_webgpu_enabled: p.webgpu,
        dom_notification_enabled: p.notifications,
        dom_serviceworker_enabled: p.service_workers,
        ..Preferences::default()
    }
}

fn data_url(html: &str) -> Result<url::Url, EngineError> {
    let mut o = String::with_capacity(html.len() * 3 + 24);
    o.push_str("data:text/html;charset=utf-8,");
    for b in html.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    url::Url::parse(&o).map_err(|e| EngineError::Failed(format!("data URL: {e}")))
}

pub(crate) struct Headless {
    servo: Servo,
    ctx: Rc<SoftwareRenderingContext>,
}

impl Headless {
    pub fn new(w: u32, h: u32) -> Result<Self, EngineError> {
        let prefs = HardenedPrefs::default();
        if !prefs.is_locked_down() {
            return Err(EngineError::Refused(
                "Servo preferences are not locked down",
            ));
        }
        let ctx = SoftwareRenderingContext::new(dpi::PhysicalSize::new(w, h))
            .map_err(|e| EngineError::Failed(format!("software surface: {e:?}")))?;
        ctx.make_current()
            .map_err(|e| EngineError::Failed(format!("make current: {e:?}")))?;
        let servo = ServoBuilder::default()
            .preferences(to_servo_prefs(&prefs))
            .event_loop_waker(Box::new(PollWaker))
            .build();
        Ok(Self {
            servo,
            ctx: Rc::new(ctx),
        })
    }

    fn spin_until(&self, deadline: Instant, done: impl Fn() -> bool) -> bool {
        while !done() {
            if Instant::now() >= deadline {
                return false;
            }
            self.servo.spin_event_loop();
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }

    pub fn render(&mut self, html: &str, w: u32, h: u32) -> Result<Frame, EngineError> {
        if self.ctx.size() != dpi::PhysicalSize::new(w, h) {
            self.ctx.resize(dpi::PhysicalSize::new(w, h));
        }
        let watch = Rc::new(Watch::default());
        let webview = WebViewBuilder::new(&self.servo, self.ctx.clone())
            .url(data_url(html)?)
            .delegate(watch.clone())
            .build();
        let deadline = Instant::now() + RENDER_TIMEOUT;
        if !self.spin_until(deadline, || watch.complete.get()) {
            return Err(EngineError::Failed("Servo load timed out".into()));
        }
        let shot: Rc<RefCell<Option<Result<RgbaImage, String>>>> = Rc::default();
        let sink = shot.clone();
        webview.take_screenshot(None, move |r| {
            *sink.borrow_mut() = Some(r.map_err(|e| format!("{e:?}")));
        });
        if !self.spin_until(deadline, || shot.borrow().is_some()) {
            return Err(EngineError::Failed("Servo screenshot timed out".into()));
        }
        let img = shot
            .borrow_mut()
            .take()
            .expect("checked above")
            .map_err(EngineError::Failed)?;
        let title = webview.page_title().unwrap_or_default();
        drop(webview);
        Ok(Frame {
            title,
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        })
    }
}
