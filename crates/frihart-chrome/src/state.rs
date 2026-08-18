//! Browser session state: tabs, URL bar, navigation.

use frihart_content::{Document, PrefToggle, SessionHistory, load};
use frihart_core::{TabId, display_url, parse_user_input};
use frihart_profile::Profile;
use url::Url;

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
}

impl Browser {
    pub fn new(mut profile: Profile, initial: Option<String>) -> Self {
        let start = initial.unwrap_or_else(|| profile.prefs().general.homepage.clone());
        let tab = open_tab(&mut profile, &start);
        let url_text = tab.url_display();
        Self {
            profile,
            tabs: vec![tab],
            active: 0,
            url_focused: false,
            url_text,
            url_cursor: 0,
            hover: None,
            cursor: (0.0, 0.0),
            status: "Ready.".into(),
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
        let tab = open_tab(&mut self.profile, &url);
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.sync_url_bar();
        self.url_focused = false;
        self.status = "New tab.".into();
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
        self.status = "Location.".into();
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
        match parse_user_input(&raw) {
            Ok(url) => self.navigate(url),
            Err(_) => self.status = format!("Could not parse “{raw}”."),
        }
        self.url_focused = false;
    }

    pub fn navigate(&mut self, url: Url) {
        let doc = load(&url, &self.profile);
        let title = doc.title().to_string();
        let _ = self.profile.record_visit(url.as_str(), &title);
        {
            let tab = self.active_tab_mut();
            tab.session.push(url, title);
            tab.document = doc;
            tab.scroll_y = 0.0;
        }
        self.sync_url_bar();
        self.status = "Ready.".into();
    }

    pub fn reload(&mut self) {
        let url = self.active_tab().url();
        let doc = load(&url, &self.profile);
        let title = doc.title().to_string();
        {
            let tab = self.active_tab_mut();
            tab.session.update_title(&title);
            tab.document = doc;
        }
        self.sync_url_bar();
        self.status = "Reloaded.".into();
    }

    pub fn go_back(&mut self) {
        let url = {
            let tab = self.active_tab_mut();
            tab.session.set_scroll(tab.scroll_y);
            match tab.session.back() {
                Some(entry) => entry.url.clone(),
                None => {
                    self.status = "No previous page.".into();
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
                    self.status = "No next page.".into();
                    return;
                }
            }
        };
        self.restore(&url);
    }

    fn restore(&mut self, url: &Url) {
        let doc = load(url, &self.profile);
        let scroll = self.active_tab().session.current().scroll_y;
        let tab = self.active_tab_mut();
        tab.document = doc;
        tab.scroll_y = scroll;
        self.sync_url_bar();
        self.status = "Ready.".into();
    }

    pub fn apply_toggle(&mut self, toggle: PrefToggle) {
        toggle.apply(self.profile.prefs_mut());
        if let Err(err) = self.profile.save_prefs() {
            self.status = format!("Could not save prefs: {err}");
            return;
        }
        self.reload();
        self.status = "Preference saved.".into();
    }

    pub fn scroll(&mut self, delta: f32) {
        let tab = self.active_tab_mut();
        tab.scroll_y = (tab.scroll_y + delta).max(0.0);
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
    }
}
