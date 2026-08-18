//! Browser session state: tabs, URL bar, navigation.

use frihart_autofill::Identity;
use frihart_blocker::FilterEngine;
use frihart_content::{Document, FetchRequest, PageItem, PrefToggle, SessionHistory, fetch, load};
use frihart_core::{
    ContainerId, IsolationKey, TabId, WindowId, display_url, looks_like_destination,
    parse_user_input,
};
use frihart_gfx::DisplayList;
use frihart_ipc::Supervisor;
use frihart_net::{CookieJar, FetchMode, RustlsClient};
use frihart_pipeline::LayoutJob;
use frihart_profile::Profile;
use frihart_search::{by_id, primary, resolve};
use url::Url;

/// How this tab talks to the network.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Circuit {
    Direct,
    Private,
    Tor,
}

impl Circuit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Private => "private",
            Self::Tor => "tor",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Hit {
    Tab(usize),
    CloseTab(usize),
    NewTab,
    Back,
    Forward,
    Reload,
    UrlBar,
    PrivacyBadge,
    ContainerBadge,
    FindBar,
    ContentLink(String),
    ContentToggle(PrefToggle),
    Field(usize),
    Autofill,
    PassLaunch,
}

pub struct Tab {
    /// Stable id for the future process model. Unused in the single-process chrome.
    #[allow(dead_code)]
    pub id: TabId,
    pub session: SessionHistory,
    pub document: Document,
    pub scroll_y: f32,
    pub container: ContainerId,
    pub circuit: Circuit,
    pub frame: Option<DisplayList>,
    pub frame_w: f32,
    pub sandboxed: bool,
}

impl Tab {
    pub fn title(&self) -> String {
        let raw = self.document.title();
        if raw.is_empty() {
            "New tab".into()
        } else {
            raw.to_string()
        }
    }

    pub fn url(&self) -> Url {
        self.session.current().url.clone()
    }

    pub fn url_display(&self) -> String {
        display_url(&self.url())
    }
}

pub struct Browser {
    /// Stable id for the future multi-window chrome.
    #[allow(dead_code)]
    pub window_id: WindowId,
    pub profile: Profile,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub url_focused: bool,
    pub url_text: String,
    pub url_cursor: usize,
    pub hover: Option<Hit>,
    pub cursor: (f64, f64),
    pub status: String,
    pub find_open: bool,
    pub find_focused: bool,
    pub find_text: String,
    pub find_status: String,
    client: RustlsClient,
    jar: CookieJar,
    blocker: FilterEngine,
    pub field_focus: Option<usize>,
    identity: Identity,
    pub supervisor: Supervisor,
    workers: crate::worker::WorkerPool,
}

impl Browser {
    pub fn new(mut profile: Profile, initial: Option<String>, tor: bool) -> Self {
        let start = initial.unwrap_or_else(|| {
            if profile.prefs().general.welcome_seen {
                profile.prefs().general.homepage.clone()
            } else {
                "about:welcome".into()
            }
        });
        let mut tab = open_tab(&mut profile, &start);
        if tor {
            tab.circuit = Circuit::Tor;
        } else if profile.is_ephemeral() {
            tab.circuit = Circuit::Private;
        }
        let url_text = tab.url_display();
        let identity = Identity::load(&profile.root().join("autofill.toml")).unwrap_or_default();
        let enabled = profile.prefs().privacy.blocker;
        let jar = if profile.is_ephemeral() || !profile.prefs().privacy.persist_cookies {
            CookieJar::default()
        } else {
            CookieJar::load(&profile.root().join("cookies.json")).unwrap_or_default()
        };
        Self {
            window_id: WindowId::new(),
            profile,
            tabs: vec![tab],
            active: 0,
            url_focused: false,
            url_text,
            url_cursor: 0,
            hover: None,
            cursor: (0.0, 0.0),
            status: String::new(),
            find_open: false,
            find_focused: false,
            find_text: String::new(),
            find_status: String::new(),
            client: RustlsClient::new(),
            jar,
            blocker: FilterEngine::new(enabled),
            field_focus: None,
            identity,
            supervisor: Supervisor::default(),
            workers: crate::worker::WorkerPool::default(),
        }
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    pub fn is_private(&self) -> bool {
        self.profile.is_ephemeral()
    }

    pub fn new_tab(&mut self) {
        let url = self.profile.prefs().general.new_tab_url.clone();
        let container = self.active_tab().container;
        let circuit = self.active_tab().circuit;
        let mut tab = open_tab(&mut self.profile, &url);
        tab.container = container;
        tab.circuit = match circuit {
            Circuit::Tor => Circuit::Tor,
            Circuit::Private => Circuit::Private,
            Circuit::Direct => Circuit::Direct,
        };
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.sync_url_bar();
        self.url_focused = false;
        self.status.clear();
    }

    pub fn close_tab(&mut self, index: usize) -> bool {
        if self.tabs.len() == 1 {
            return true;
        }
        if index >= self.tabs.len() {
            return false;
        }
        self.tabs.remove(index);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if self.active > index {
            self.active -= 1;
        }
        self.sync_url_bar();
        self.reap_workers();
        false
    }

    fn reap_workers(&mut self) {
        let keep: Vec<IsolationKey> = self
            .tabs
            .iter()
            .map(|t| IsolationKey::from_url(&t.url(), t.container))
            .collect();
        self.workers.retain_keys(&keep);
    }

    pub fn activate(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
            self.sync_url_bar();
            self.url_focused = false;
        }
    }

    pub fn cycle(&mut self, backward: bool) {
        if self.tabs.is_empty() {
            return;
        }
        let n = self.tabs.len();
        self.active = if backward {
            (self.active + n - 1) % n
        } else {
            (self.active + 1) % n
        };
        self.sync_url_bar();
        self.url_focused = false;
    }

    pub fn focus_url(&mut self) {
        self.sync_url_bar();
        self.url_cursor = self.url_text.len();
        self.url_focused = true;
        self.status.clear();
    }

    pub fn blur_url(&mut self) {
        self.url_focused = false;
        self.sync_url_bar();
    }

    pub fn sync_url_bar(&mut self) {
        self.url_text = self.active_tab().url_display();
        self.url_cursor = self.url_text.len();
    }

    pub fn insert_url_text(&mut self, text: &str) {
        let insert_at = self.url_cursor.min(self.url_text.len());
        self.url_text.insert_str(insert_at, text);
        self.url_cursor = insert_at + text.len();
    }

    pub fn backspace_url(&mut self) {
        if self.url_cursor == 0 {
            return;
        }
        let prev = self.url_text[..self.url_cursor]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.url_text.replace_range(prev..self.url_cursor, "");
        self.url_cursor = prev;
    }

    pub fn delete_url(&mut self) {
        if self.url_cursor >= self.url_text.len() {
            return;
        }
        let next = self.url_text[self.url_cursor..]
            .chars()
            .next()
            .map(|c| self.url_cursor + c.len_utf8())
            .unwrap_or(self.url_cursor);
        self.url_text.replace_range(self.url_cursor..next, "");
    }

    pub fn move_url_cursor(&mut self, delta: i32) {
        if delta < 0 {
            let left = self.url_text[..self.url_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.url_cursor = left;
        } else {
            let right = self.url_text[self.url_cursor..]
                .chars()
                .next()
                .map(|c| self.url_cursor + c.len_utf8())
                .unwrap_or(self.url_text.len());
            self.url_cursor = right;
        }
    }

    pub fn commit_url(&mut self) {
        let raw = self.url_text.clone();
        if !looks_like_destination(&raw) {
            self.search(&raw);
            self.url_focused = false;
            return;
        }
        match parse_user_input(&raw) {
            Ok(url) => self.navigate(url),
            Err(_) => self.status = "err".into(),
        }
        self.url_focused = false;
    }

    pub fn search(&mut self, query: &str) {
        let id = if !self.profile.prefs().general.search_url.is_empty() {
            None
        } else {
            Some(self.profile.prefs().search.primary.clone())
        };
        let url = if let Some(ref override_url) = {
            let custom = self.profile.prefs().general.search_url.clone();
            if custom.is_empty() {
                None
            } else {
                Some(custom.replace("{q}", query).replace("%s", query))
            }
        } {
            parse_user_input(override_url).ok()
        } else {
            let engine = id.as_deref().and_then(by_id).unwrap_or_else(primary);
            resolve(engine, query)
        };
        match url {
            Some(url) => {
                self.status.clear();
                self.navigate(url);
            }
            None => self.status = "err".into(),
        }
    }

    pub fn new_tor_tab(&mut self) {
        let url = self.profile.prefs().general.new_tab_url.clone();
        let container = self.active_tab().container;
        let mut tab = open_tab(&mut self.profile, &url);
        tab.container = container;
        tab.circuit = Circuit::Tor;
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.sync_url_bar();
        self.url_focused = false;
        self.status = "tor".into();
    }

    pub fn cycle_container(&mut self) {
        if !self.profile.prefs().privacy.containers {
            self.status = "off".into();
            return;
        }
        let next = self.profile.containers().cycle(self.active_tab().container);
        self.active_tab_mut().container = next;
        let name = self
            .profile
            .container(next)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| next.slug().to_string());
        self.status = name;
    }

    pub fn assign_container(&mut self, slug: &str) {
        if let Some(id) = ContainerId::from_slug(slug) {
            self.active_tab_mut().container = id;
            self.status = id.slug().into();
        }
    }

    pub fn remove_bookmark(&mut self, url: &str) {
        if self.profile.bookmarks_mut().remove_url(url) {
            let _ = self.profile.save_bookmarks();
        }
        self.status.clear();
        if self
            .active_tab()
            .url_display()
            .starts_with("about:bookmarks")
        {
            self.reload();
        }
    }

    pub fn bookmark_current(&mut self) {
        let url = self.active_tab().url_display();
        let title = self.active_tab().title();
        self.profile.bookmarks_mut().add(title, url);
        if self.profile.save_bookmarks().is_err() {
            self.status = "err".into();
            return;
        }
        self.status.clear();
    }

    pub fn open_find(&mut self) {
        self.find_open = true;
        self.find_focused = true;
        self.url_focused = false;
        self.status.clear();
    }

    pub fn close_find(&mut self) {
        self.find_open = false;
        self.find_focused = false;
        self.find_status.clear();
    }

    pub fn insert_find_text(&mut self, text: &str) {
        self.find_text.push_str(text);
        self.run_find();
    }

    pub fn backspace_find(&mut self) {
        self.find_text.pop();
        self.run_find();
    }

    pub fn run_find(&mut self) {
        if self.find_text.is_empty() {
            self.find_status.clear();
            return;
        }
        let needle = self.find_text.clone();
        if let Some(frame) = &self.active_tab().frame {
            if let Some(hit) = frame.find(&needle) {
                self.find_status = "ok".into();
                self.active_tab_mut().scroll_y = hit.y.max(0.0);
                return;
            }
        }
        let hay = self.active_tab().document.searchable_text();
        let needle_l = needle.to_ascii_lowercase();
        let hay_l = hay.to_ascii_lowercase();
        if let Some(pos) = hay_l.find(&needle_l) {
            self.find_status = "ok".into();
            self.active_tab_mut().scroll_y = (pos as f32 * 0.15).min(2000.0);
        } else {
            self.find_status = "none".into();
        }
    }

    /// Layout HTML in a sandboxed worker when possible. Safe to call often.
    pub fn prepare_frame(&mut self, width: f32) {
        let width = width.max(80.0);
        let extra = self.profile.user_css();
        let html = match &self.active_tab().document {
            Document::Page(page) if !page.html.is_empty() => page.html.clone(),
            _ => {
                let tab = self.active_tab_mut();
                tab.frame = None;
                tab.sandboxed = false;
                return;
            }
        };
        let tab = self.active_tab();
        if tab.frame.is_some() && (tab.frame_w - width).abs() < 8.0 {
            return;
        }
        let key = IsolationKey::from_url(&tab.url(), tab.container);
        let out = self.workers.layout(
            key,
            &LayoutJob {
                html,
                extra_css: extra,
                viewport_w: width,
            },
        );
        let tab = self.active_tab_mut();
        tab.frame = Some(out.display);
        tab.frame_w = width;
        tab.sandboxed = out.sandboxed;
    }

    pub fn navigate(&mut self, url: Url) {
        let url = if url.scheme() == "https" || url.scheme() == "http" {
            frihart_net::strip_tracking(&url)
        } else {
            url
        };
        if url.scheme() == "frihart" {
            let spec = url.as_str().trim_start_matches("frihart:");
            if let Some(slug) = spec.strip_prefix("container/") {
                self.assign_container(slug);
                return;
            }
            if spec == "wipe-history" || spec == "wipe" {
                self.wipe();
                return;
            }
            if spec == "reset" {
                self.reset_profile();
                return;
            }
            if spec == "shred" {
                self.shred();
                return;
            }
            if let Some(name) = spec.strip_prefix("profile-new/") {
                self.create_profile(name);
                return;
            }
            if let Some(name) = spec.strip_prefix("profile/") {
                self.switch_profile(name);
                return;
            }
            if spec == "pass" || spec.starts_with("pass/") {
                let id = spec.strip_prefix("pass/").unwrap_or("");
                self.launch_pass(id);
                return;
            }
            if let Some(target) = spec.strip_prefix("unbookmark/") {
                self.remove_bookmark(target);
                return;
            }
        }
        let doc = self.open_url(&url);
        let title = doc.title().to_string();
        let container = self.active_tab().container;
        let key = IsolationKey::from_url(&url, container);
        if self.active_tab().circuit == Circuit::Direct {
            let _ = self.profile.record_visit(url.as_str(), &title);
        }
        {
            let tab = self.active_tab_mut();
            tab.session.push(url, title);
            tab.document = doc;
            tab.scroll_y = 0.0;
            tab.frame = None;
            tab.sandboxed = false;
        }
        self.sync_url_bar();
        self.persist_jar();
        self.field_focus = None;
        let _ = self.supervisor.slot_for(key);
        self.prepare_frame(self.active_tab().frame_w.max(960.0));
        self.status.clear();
    }

    pub fn reload(&mut self) {
        let url = self.active_tab().url();
        let doc = self.open_url(&url);
        let title = doc.title().to_string();
        {
            let tab = self.active_tab_mut();
            tab.session.update_title(&title);
            tab.document = doc;
            tab.frame = None;
            tab.sandboxed = false;
        }
        self.sync_url_bar();
        self.persist_jar();
        self.prepare_frame(self.active_tab().frame_w.max(960.0));
        self.status.clear();
    }

    fn open_url(&mut self, url: &Url) -> Document {
        let cleaned = if url.scheme() == "https" || url.scheme() == "http" {
            frihart_net::strip_tracking(url)
        } else {
            url.clone()
        };
        let (target, _view_source) = strip_view_source(&cleaned);
        if target.scheme() == "about" {
            return load(&target, &self.profile);
        }
        let mode = match self.active_tab().circuit {
            Circuit::Tor => {
                let tor = &self.profile.prefs().tor;
                FetchMode::Tor {
                    socks: format!("{}:{}", tor.socks_host, tor.socks_port),
                }
            }
            _ => FetchMode::Direct,
        };
        let container = self.active_tab().container;
        fetch(FetchRequest {
            url: &target,
            profile: &self.profile,
            client: &self.client,
            jar: &mut self.jar,
            blocker: &self.blocker,
            container,
            mode,
        })
    }

    pub fn wipe(&mut self) {
        self.jar.clear();
        let _ = self.profile.wipe_session();
        self.persist_jar();
        self.workers.drop_all();
        self.reset_tabs();
        self.status = "wiped".into();
    }

    pub fn reset_profile(&mut self) {
        self.jar.clear();
        let _ = self.profile.reset_like_new();
        self.blocker = FilterEngine::new(self.profile.prefs().privacy.blocker);
        self.persist_jar();
        self.reset_tabs();
        self.status = "reset".into();
    }

    pub fn shred(&mut self) {
        self.jar.clear();
        let _ = self.profile.shred();
        self.workers.drop_all();
        self.reset_tabs();
        self.status = "shredded".into();
    }

    pub fn switch_profile(&mut self, name: &str) {
        if self.profile.is_ephemeral() {
            return;
        }
        let _ = self.profile.save_prefs();
        let _ = self.profile.save_bookmarks();
        self.persist_jar();
        let Ok(profile) = Profile::open_named(name) else {
            self.status = "err".into();
            return;
        };
        self.replace_profile(profile);
        self.status.clear();
    }

    pub fn create_profile(&mut self, name: &str) {
        if self.profile.is_ephemeral() {
            return;
        }
        let slug = frihart_profile::sanitize_name(name);
        let _ = Profile::open_named(&slug);
        self.switch_profile(&slug);
    }

    fn replace_profile(&mut self, profile: Profile) {
        let persist = !profile.is_ephemeral() && profile.prefs().privacy.persist_cookies;
        let enabled = profile.prefs().privacy.blocker;
        let jar = if persist {
            CookieJar::load(&profile.root().join("cookies.json")).unwrap_or_default()
        } else {
            CookieJar::default()
        };
        self.profile = profile;
        self.jar = jar;
        self.blocker = FilterEngine::new(enabled);
        self.identity =
            Identity::load(&self.profile.root().join("autofill.toml")).unwrap_or_default();
        self.field_focus = None;
        self.supervisor = Supervisor::default();
        self.workers.drop_all();
        self.reset_tabs();
    }

    fn reset_tabs(&mut self) {
        let home = parse_user_input("about:home")
            .unwrap_or_else(|_| Url::parse("about:home").expect("about:home"));
        let doc = load(&home, &self.profile);
        let title = doc.title().to_string();
        self.tabs.truncate(1);
        self.active = 0;
        {
            let tab = self.active_tab_mut();
            tab.session = SessionHistory::new(home, title);
            tab.document = doc;
            tab.scroll_y = 0.0;
            tab.frame = None;
            tab.sandboxed = false;
        }
        self.sync_url_bar();
    }

    pub fn launch_pass(&mut self, id: &str) {
        let wanted = if id.is_empty() {
            self.profile.prefs().pass.manager.as_str()
        } else {
            id
        };
        let found = frihart_platform::detect_pass_managers();
        let target = found
            .iter()
            .find(|m| m.id == wanted)
            .or_else(|| found.first());
        if let Some(mgr) = target {
            let _ = frihart_platform::launch_local(&mgr.path);
        }
        self.status.clear();
    }

    pub fn focus_field(&mut self, index: usize) {
        self.field_focus = Some(index);
        self.url_focused = false;
        self.find_focused = false;
    }

    pub fn insert_field_text(&mut self, text: &str) {
        let Some(i) = self.field_focus else {
            return;
        };
        let Document::Page(page) = &mut self.active_tab_mut().document else {
            return;
        };
        if let Some(PageItem::Field { value, secret, .. }) = page.items.get_mut(i) {
            if *secret {
                return;
            }
            value.push_str(text);
        }
    }

    pub fn backspace_field(&mut self) {
        let Some(i) = self.field_focus else {
            return;
        };
        let Document::Page(page) = &mut self.active_tab_mut().document else {
            return;
        };
        if let Some(PageItem::Field { value, secret, .. }) = page.items.get_mut(i) {
            if *secret {
                return;
            }
            value.pop();
        }
    }

    pub fn submit_form(&mut self) {
        let Document::Page(page) = &self.active_tab().document else {
            return;
        };
        let mut fields = Vec::new();
        for item in &page.items {
            if let PageItem::Field {
                label,
                value,
                secret,
                ..
            } = item
            {
                fields.push(frihart_forms::Field {
                    name: label.clone(),
                    value: value.clone(),
                    secret: *secret,
                });
            }
        }
        let submit = frihart_forms::Submit {
            action: page.form_action.clone(),
            method: page.form_method.clone(),
            fields,
        };
        let Some(next) = submit.get_url(&page.url) else {
            return;
        };
        self.navigate(next);
    }

    pub fn autofill(&mut self) {
        if !self.profile.prefs().autofill.enabled {
            return;
        }
        let identity = self.identity.clone();
        let Document::Page(page) = &mut self.active_tab_mut().document else {
            return;
        };
        for item in &mut page.items {
            if let PageItem::Field {
                kind,
                value,
                secret,
                ..
            } = item
            {
                if *secret {
                    continue;
                }
                if let Some(v) = identity.value_for(*kind) {
                    *value = v.to_string();
                }
            }
        }
    }

    fn persist_jar(&self) {
        if self.profile.is_ephemeral() || !self.profile.prefs().privacy.persist_cookies {
            return;
        }
        let _ = self.jar.save(&self.profile.root().join("cookies.json"));
    }

    pub fn go_back(&mut self) {
        let url = {
            let tab = self.active_tab_mut();
            tab.session.set_scroll(tab.scroll_y);
            match tab.session.back() {
                Some(entry) => entry.url.clone(),
                None => {
                    self.status.clear();
                    return;
                }
            }
        };
        self.restore(&url);
    }

    pub fn go_forward(&mut self) {
        let url = {
            let tab = self.active_tab_mut();
            tab.session.set_scroll(tab.scroll_y);
            match tab.session.forward() {
                Some(entry) => entry.url.clone(),
                None => {
                    self.status.clear();
                    return;
                }
            }
        };
        self.restore(&url);
    }

    fn restore(&mut self, url: &Url) {
        let doc = self.open_url(url);
        let scroll = self.active_tab().session.current().scroll_y;
        let tab = self.active_tab_mut();
        tab.document = doc;
        tab.scroll_y = scroll;
        self.sync_url_bar();
        self.status.clear();
    }

    pub fn apply_toggle(&mut self, toggle: PrefToggle) {
        toggle.apply(self.profile.prefs_mut());
        if self.profile.save_prefs().is_err() {
            self.status = "err".into();
            return;
        }
        self.blocker = FilterEngine::new(self.profile.prefs().privacy.blocker);
        self.reload();
        self.status.clear();
    }

    pub fn scroll(&mut self, delta: f32) {
        let tab = self.active_tab_mut();
        tab.scroll_y = (tab.scroll_y + delta).max(0.0);
    }
}

fn strip_view_source(url: &Url) -> (Url, bool) {
    if url.scheme() != "view-source" {
        return (url.clone(), false);
    }
    let inner = url.as_str().trim_start_matches("view-source:");
    match Url::parse(inner) {
        Ok(u) => (u, true),
        Err(_) => (url.clone(), true),
    }
}

fn open_tab(profile: &mut Profile, input: &str) -> Tab {
    let url =
        parse_user_input(input).unwrap_or_else(|_| Url::parse("about:home").expect("about:home"));
    let document = load(&url, profile);
    let title = document.title().to_string();
    let _ = profile.record_visit(url.as_str(), &title);
    Tab {
        id: TabId::new(),
        session: SessionHistory::new(url, title),
        document,
        scroll_y: 0.0,
        container: ContainerId::PERSONAL,
        circuit: Circuit::Direct,
        frame: None,
        frame_w: 0.0,
        sandboxed: false,
    }
}
