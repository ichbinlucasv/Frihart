//! Browser session state: tabs, URL bar, navigation.

use frihart_blocker::FilterEngine;
use frihart_content::{Document, FetchRequest, PrefToggle, SessionHistory, fetch, load};
use frihart_core::{ContainerId, TabId, display_url, looks_like_destination, parse_user_input};
use frihart_net::{CookieJar, FetchMode, RustlsClient};
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
        let enabled = profile.prefs().privacy.blocker;
        let jar = if profile.is_ephemeral() || !profile.prefs().privacy.persist_cookies {
            CookieJar::default()
        } else {
            CookieJar::load(&profile.root().join("cookies.json")).unwrap_or_default()
        };
        Self {
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
        false
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
        let hay = self.active_tab().document.searchable_text();
        let needle = self.find_text.to_ascii_lowercase();
        let hay_l = hay.to_ascii_lowercase();
        if let Some(pos) = hay_l.find(&needle) {
            self.find_status = "ok".into();
            self.active_tab_mut().scroll_y = (pos as f32 * 0.15).min(2000.0);
        } else {
            self.find_status = "none".into();
        }
    }

    pub fn navigate(&mut self, url: Url) {
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
            if spec == "shred" {
                self.shred();
                return;
            }
        }
        let doc = self.open_url(&url);
        let title = doc.title().to_string();
        if self.active_tab().circuit == Circuit::Direct {
            let _ = self.profile.record_visit(url.as_str(), &title);
        }
        {
            let tab = self.active_tab_mut();
            tab.session.push(url, title);
            tab.document = doc;
            tab.scroll_y = 0.0;
        }
        self.sync_url_bar();
        self.persist_jar();
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
        }
        self.sync_url_bar();
        self.persist_jar();
        self.status.clear();
    }

    fn open_url(&mut self, url: &Url) -> Document {
        let (target, _view_source) = strip_view_source(url);
        if target.scheme() == "about" {
            return load(&target, &self.profile);
        }
        let mode = match self.active_tab().circuit {
            Circuit::Tor => FetchMode::Tor,
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
        }
        self.sync_url_bar();
        self.status = "wiped".into();
    }

    pub fn shred(&mut self) {
        self.jar.clear();
        let _ = self.profile.shred();
        self.wipe();
        self.status = "shredded".into();
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
    }
}
