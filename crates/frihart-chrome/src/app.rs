//! winit event loop. Linux (Wayland / X11) is the reference target.

use std::num::NonZeroU32;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, Window, WindowId};

use frihart_core::{FrihartError, Result};
use frihart_platform::window_title;
use frihart_profile::Profile;

use crate::paint::{HitRegion, hit_test, paint};
use crate::raster::Framebuffer;
use crate::state::{Browser, Hit};
use crate::text::TextEngine;

struct WindowSurface {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

struct Handler {
    browser: Browser,
    window: Option<WindowSurface>,
    text: TextEngine,
    hits: Vec<HitRegion>,
    modifiers: ModifiersState,
}

/// Open the Linux chrome and run until the user quits.
pub fn run(profile: Profile, initial_url: Option<String>, tor: bool) -> Result<()> {
    let browser = Browser::new(profile, initial_url, tor);
    let event_loop = EventLoop::new()
        .map_err(|e| FrihartError::Message(format!("could not create event loop: {e}")))?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut handler = Handler {
        browser,
        window: None,
        text: TextEngine::new(),
        hits: Vec::new(),
        modifiers: ModifiersState::default(),
    };
    event_loop
        .run_app(&mut handler)
        .map_err(|e| FrihartError::Message(format!("event loop: {e}")))
}

impl Handler {
    fn ensure_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let title = window_title(&self.browser.active_tab().title());
        let attrs = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1120.0, 780.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(_) => {
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(_) => {
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(_) => {
                event_loop.exit();
                return;
            }
        };
        let _ = context;
        self.window = Some(WindowSurface { window, surface });
        if let Some(ws) = &self.window {
            ws.window.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let Some(ws) = self.window.as_mut() else {
            return;
        };
        let size = ws.window.inner_size();
        let width = size.width;
        let height = size.height;
        if width == 0 || height == 0 {
            return;
        }
        let Some(w) = NonZeroU32::new(width) else {
            return;
        };
        let Some(h) = NonZeroU32::new(height) else {
            return;
        };
        if ws.surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = ws.surface.buffer_mut() else {
            return;
        };
        let mut fb = Framebuffer::new(width, height);
        let scale = ws.window.scale_factor() as f32;
        self.hits = paint(&mut fb, &mut self.text, &self.browser, scale);
        if buffer.len() == fb.pixels.len() {
            buffer.copy_from_slice(&fb.pixels);
        }
        let _ = buffer.present();
        ws.window
            .set_title(&window_title(&self.browser.active_tab().title()));
    }

    fn request_redraw(&self) {
        if let Some(ws) = &self.window {
            ws.window.request_redraw();
        }
    }

    fn handle_hit(&mut self, hit: Hit, event_loop: &ActiveEventLoop) {
        match hit {
            Hit::Tab(i) => self.browser.activate(i),
            Hit::CloseTab(i) => {
                if self.browser.close_tab(i) {
                    event_loop.exit();
                }
            }
            Hit::NewTab => self.browser.new_tab(),
            Hit::Back => self.browser.go_back(),
            Hit::Forward => self.browser.go_forward(),
            Hit::Reload => self.browser.reload(),
            Hit::UrlBar => self.browser.focus_url(),
            Hit::PrivacyBadge => {
                if let Ok(url) = frihart_core::parse_user_input("about:blocker") {
                    self.browser.navigate(url);
                }
            }
            Hit::ContainerBadge => self.browser.cycle_container(),
            Hit::FindBar => {
                self.browser.find_open = true;
                self.browser.find_focused = true;
                self.browser.url_focused = false;
            }
            Hit::ContentLink(href) => {
                if let Ok(url) = frihart_core::parse_user_input(&href) {
                    self.browser.navigate(url);
                }
            }
            Hit::ContentToggle(toggle) => self.browser.apply_toggle(toggle),
            Hit::Field(i) => self.browser.focus_field(i),
            Hit::Autofill => self.browser.autofill(),
            Hit::PassLaunch => self.browser.launch_pass(""),
        }
        self.request_redraw();
    }

    fn key(&mut self, event_loop: &ActiveEventLoop, event: winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();

        if ctrl {
            if let Key::Character(ch) = &event.logical_key {
                match ch.to_lowercase().as_str() {
                    "t" => {
                        self.browser.new_tab();
                        self.request_redraw();
                        return;
                    }
                    "w" => {
                        if self.browser.close_tab(self.browser.active) {
                            event_loop.exit();
                        }
                        self.request_redraw();
                        return;
                    }
                    "l" => {
                        self.browser.focus_url();
                        self.request_redraw();
                        return;
                    }
                    "r" => {
                        self.browser.reload();
                        self.request_redraw();
                        return;
                    }
                    "q" => {
                        event_loop.exit();
                        return;
                    }
                    "f" => {
                        self.browser.open_find();
                        self.request_redraw();
                        return;
                    }
                    "d" => {
                        self.browser.bookmark_current();
                        self.request_redraw();
                        return;
                    }
                    "c" if shift => {
                        self.browser.cycle_container();
                        self.request_redraw();
                        return;
                    }
                    "o" if shift => {
                        self.browser.new_tor_tab();
                        self.request_redraw();
                        return;
                    }
                    "a" if shift => {
                        self.browser.autofill();
                        self.request_redraw();
                        return;
                    }
                    "k" => {
                        self.browser.focus_url();
                        self.browser.url_text.clear();
                        self.browser.url_cursor = 0;
                        self.browser.status.clear();
                        self.request_redraw();
                        return;
                    }
                    "u" if self.browser.url_focused => {
                        self.browser.url_text.clear();
                        self.browser.url_cursor = 0;
                        self.request_redraw();
                        return;
                    }
                    digit if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() => {
                        let n = digit.as_bytes()[0] - b'0';
                        if (1..=9).contains(&n) {
                            self.browser.activate(usize::from(n) - 1);
                            self.request_redraw();
                        }
                        return;
                    }
                    _ => {}
                }
            }
            if let Key::Named(NamedKey::Tab) = event.logical_key {
                self.browser.cycle(shift);
                self.request_redraw();
                return;
            }
        }

        if alt {
            match event.logical_key {
                Key::Named(NamedKey::ArrowLeft) => {
                    self.browser.go_back();
                    self.request_redraw();
                    return;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.browser.go_forward();
                    self.request_redraw();
                    return;
                }
                _ => {}
            }
        }

        match event.logical_key {
            Key::Named(NamedKey::F5) => {
                self.browser.reload();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Escape) => {
                if self.browser.find_open {
                    self.browser.close_find();
                } else if self.browser.field_focus.is_some() {
                    self.browser.field_focus = None;
                } else {
                    self.browser.blur_url();
                }
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Enter) if self.browser.find_focused => {
                self.browser.run_find();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Backspace) if self.browser.find_focused => {
                self.browser.backspace_find();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Enter) if self.browser.field_focus.is_some() => {
                self.browser.submit_form();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Enter) if self.browser.url_focused => {
                self.browser.commit_url();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Backspace) if self.browser.field_focus.is_some() => {
                self.browser.backspace_field();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Backspace) if self.browser.url_focused => {
                self.browser.backspace_url();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Delete) if self.browser.url_focused => {
                self.browser.delete_url();
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::ArrowLeft) if self.browser.url_focused && !alt => {
                self.browser.move_url_cursor(-1);
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::ArrowRight) if self.browser.url_focused && !alt => {
                self.browser.move_url_cursor(1);
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::Home) if self.browser.url_focused => {
                self.browser.url_cursor = 0;
                self.request_redraw();
                return;
            }
            Key::Named(NamedKey::End) if self.browser.url_focused => {
                self.browser.url_cursor = self.browser.url_text.len();
                self.request_redraw();
                return;
            }
            _ => {}
        }

        if self.browser.field_focus.is_some() && !ctrl && !alt {
            if let Some(txt) = event.text.as_ref() {
                if !txt.chars().any(|c| c.is_control()) {
                    self.browser.insert_field_text(txt);
                    self.request_redraw();
                    return;
                }
            }
        }

        if self.browser.find_focused && !ctrl && !alt {
            if let Some(txt) = event.text.as_ref() {
                if !txt.chars().any(|c| c.is_control()) {
                    self.browser.insert_find_text(txt);
                    self.request_redraw();
                    return;
                }
            }
        }

        if self.browser.url_focused && !ctrl && !alt {
            if let Some(txt) = event.text.as_ref() {
                if !txt.chars().any(|c| c.is_control()) {
                    self.browser.insert_url_text(txt);
                    self.request_redraw();
                }
            }
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.ensure_window(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.browser.cursor = (position.x, position.y);
                let next = hit_test(&self.hits, position.x as i32, position.y as i32);
                let icon = match next {
                    Some(Hit::ContentLink(_)) | Some(Hit::ContentToggle(_)) => CursorIcon::Pointer,
                    Some(Hit::UrlBar) | Some(Hit::Field(_)) => CursorIcon::Text,
                    Some(Hit::Autofill) | Some(Hit::PassLaunch) => CursorIcon::Pointer,
                    _ => CursorIcon::Default,
                };
                if let Some(ws) = &self.window {
                    ws.window.set_cursor(icon);
                }
                let changed = match (&self.browser.hover, &next) {
                    (None, None) => false,
                    (Some(a), Some(b)) => !hits_eq(a, b),
                    _ => true,
                };
                self.browser.hover = next;
                if changed {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(hit) = hit_test(
                    &self.hits,
                    self.browser.cursor.0 as i32,
                    self.browser.cursor.1 as i32,
                ) {
                    if !matches!(hit, Hit::UrlBar | Hit::FindBar) {
                        self.browser.blur_url();
                        self.browser.find_focused = false;
                    }
                    self.handle_hit(hit, event_loop);
                } else {
                    self.browser.blur_url();
                    self.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.window.scale_factor() as f32)
                    .unwrap_or(1.0);
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 36.0 * scale,
                    MouseScrollDelta::PixelDelta(p) => -p.y as f32,
                };
                self.browser.scroll(dy);
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => self.key(event_loop, event),
            _ => {}
        }
    }
}

fn hits_eq(a: &Hit, b: &Hit) -> bool {
    match (a, b) {
        (Hit::Tab(i), Hit::Tab(j)) | (Hit::CloseTab(i), Hit::CloseTab(j)) => i == j,
        (Hit::NewTab, Hit::NewTab)
        | (Hit::Back, Hit::Back)
        | (Hit::Forward, Hit::Forward)
        | (Hit::Reload, Hit::Reload)
        | (Hit::UrlBar, Hit::UrlBar)
        | (Hit::PrivacyBadge, Hit::PrivacyBadge)
        | (Hit::ContainerBadge, Hit::ContainerBadge)
        | (Hit::FindBar, Hit::FindBar)
        | (Hit::Autofill, Hit::Autofill)
        | (Hit::PassLaunch, Hit::PassLaunch) => true,
        (Hit::Field(i), Hit::Field(j)) => i == j,
        (Hit::ContentLink(x), Hit::ContentLink(y)) => x == y,
        (Hit::ContentToggle(x), Hit::ContentToggle(y)) => x == y,
        _ => false,
    }
}
