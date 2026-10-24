//! Off-screen Servo: one `Servo` and one CPU rendering surface per engine,
//! one short-lived `WebView` per render.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use frihart_engine::EngineError;

use servo::{
    EventLoopWaker, LoadStatus, Preferences, RenderingContext, RgbaImage, Servo, ServoBuilder,
    SoftwareRenderingContext, StorageType, WebResourceLoad, WebResourceResponse, WebView,
    WebViewBuilder, WebViewDelegate,
};

use frihart_net::SharedClearnetDns;
use frihart_net::compat_proxy::{CompatPolicy, CompatProxy};

use crate::gate::{GateStats, check_request};
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

struct Watch {
    complete: Cell<bool>,
    gate: Arc<GateStats>,
}
impl WebViewDelegate for Watch {
    /// Every HTTP(S) load Servo makes passes here before it leaves Servo;
    /// loads that break policy are cancelled (a network error for the page).
    fn load_web_resource(&self, _: WebView, load: WebResourceLoad) {
        let req = load.request();
        let names = req.headers.keys().map(|k| k.as_str());
        let ua = req
            .headers
            .get("user-agent")
            .map(|v| v.to_str().unwrap_or("\u{fffd}"));
        let origin = req
            .headers
            .get("origin")
            .map(|v| v.to_str().unwrap_or("\u{fffd}"));
        // The document is a data: URL (see `render`), so it has no host.
        // A Cookie or Authorization inside the TLS tunnel is refused until
        // a real document host is known and shares a site with this request.
        // Origin may be `null`; any other origin needs that document host.
        // A `.onion` or `.i2p` request or origin host is refused here as
        // well, before the proxy.
        let verdict = self.gate.record(check_request(
            req.url.scheme(),
            names,
            ua,
            None,
            req.url.host_str(),
            origin,
        ));
        if verdict.is_err() {
            let url = req.url.clone();
            load.intercept(WebResourceResponse::new(url)).cancel();
        }
        // Allowed: dropping `load` lets Servo continue (through the proxy).
    }

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
        // Servo has no master "scripts off" switch; Wasm tracks our JS flag.
        js_wasm_enabled: p.javascript,
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
    proxy: CompatProxy,
    gate: Arc<GateStats>,
}

impl Headless {
    pub fn new(w: u32, h: u32, dns: SharedClearnetDns) -> Result<Self, EngineError> {
        // One TLS provider for the whole process (ring, as frihart-net).
        // Err means one is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
        // HTTPS goes through Frihart's Compat proxy (policy + DNS in
        // Frihart). Clearnet names use the shared plan (Quad9 DoH unless
        // the user opted into the system resolver). Plain HTTP stays on
        // the dead proxy and fails closed.
        let proxy = CompatProxy::spawn(CompatPolicy::sharing_dns(
            frihart_blocker::FilterEngine::builtin(),
            dns,
        ))
        .map_err(|e| EngineError::Failed(format!("compat proxy: {e}")))?;
        // Baseline is JS-off (fail closed). Per-render script stripping
        // enforces the site override; leak surfaces stay locked here.
        let prefs = HardenedPrefs {
            https_proxy: proxy.uri(),
            javascript: false,
            ..HardenedPrefs::default()
        };
        if !prefs.leak_surfaces_locked() {
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
            proxy,
            gate: Arc::default(),
        })
    }

    /// Requests the Compat proxy allowed and refused so far.
    pub fn proxy_counts(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering;
        let s = self.proxy.stats();
        (
            s.allowed.load(Ordering::Relaxed),
            s.refused.load(Ordering::Relaxed),
        )
    }

    /// Requests the in-Servo gate allowed and refused so far.
    pub fn gate_counts(&self) -> (u64, u64) {
        self.gate.counts()
    }

    /// Forget everything Servo stored while rendering: cookies, local and
    /// session storage, and the HTTP cache. Servo's jar is one global jar,
    /// not partitioned per first party, so no state may outlive a page.
    fn forget_site_state(&self) {
        let sdm = self.servo.site_data_manager();
        sdm.clear_cookies(None);
        let sites = sdm.site_data(StorageType::all());
        let names: Vec<String> = sites.iter().map(|s| s.name()).collect();
        if !names.is_empty() {
            let refs: Vec<&str> = names.iter().map(String::as_str).collect();
            sdm.clear_site_data(&refs, StorageType::all());
        }
        self.servo.network_manager().clear_cache();
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

    pub fn render(
        &mut self,
        html: &str,
        w: u32,
        h: u32,
        javascript: bool,
    ) -> Result<Frame, EngineError> {
        if self.ctx.size() != dpi::PhysicalSize::new(w, h) {
            self.ctx.resize(dpi::PhysicalSize::new(w, h));
        }
        // A page that failed half way must not leave state for the next one.
        self.forget_site_state();
        let html = if javascript {
            html.to_string()
        } else {
            crate::prefs::strip_scripts(html)
        };
        let watch = Rc::new(Watch {
            complete: Cell::new(false),
            gate: self.gate.clone(),
        });
        let webview = WebViewBuilder::new(&self.servo, self.ctx.clone())
            .url(data_url(&html)?)
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
        self.forget_site_state();
        Ok(Frame {
            title,
            width: img.width(),
            height: img.height(),
            rgba: img.into_raw(),
        })
    }
}
