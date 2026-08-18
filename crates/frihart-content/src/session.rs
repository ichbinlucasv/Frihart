use url::Url;

/// One entry in a tab's back/forward list.
#[derive(Clone, Debug)]
pub struct SessionEntry {
    pub url: Url,
    pub title: String,
    pub scroll_y: f32,
}

/// Per-tab session history. Independent of the on-disk history store.
#[derive(Clone, Debug)]
pub struct SessionHistory {
    entries: Vec<SessionEntry>,
    index: usize,
}

impl SessionHistory {
    pub fn new(url: Url, title: impl Into<String>) -> Self {
        Self {
            entries: vec![SessionEntry {
                url,
                title: title.into(),
                scroll_y: 0.0,
            }],
            index: 0,
        }
    }

    pub fn current(&self) -> &SessionEntry {
        &self.entries[self.index]
    }

    pub fn current_mut(&mut self) -> &mut SessionEntry {
        &mut self.entries[self.index]
    }

    pub fn can_go_back(&self) -> bool {
        self.index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.index + 1 < self.entries.len()
    }

    pub fn push(&mut self, url: Url, title: impl Into<String>) {
        self.entries.truncate(self.index + 1);
        self.entries.push(SessionEntry {
            url,
            title: title.into(),
            scroll_y: 0.0,
        });
        self.index = self.entries.len() - 1;
    }

    pub fn back(&mut self) -> Option<&SessionEntry> {
        if !self.can_go_back() {
            return None;
        }
        self.index -= 1;
        Some(self.current())
    }

    pub fn forward(&mut self) -> Option<&SessionEntry> {
        if !self.can_go_forward() {
            return None;
        }
        self.index += 1;
        Some(self.current())
    }

    pub fn update_title(&mut self, title: impl Into<String>) {
        self.entries[self.index].title = title.into();
    }

    pub fn set_scroll(&mut self, y: f32) {
        self.entries[self.index].scroll_y = y.max(0.0);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn back_and_forward() {
        let mut h = SessionHistory::new(url("about:home"), "Home");
        h.push(url("about:settings"), "Settings");
        h.push(url("about:privacy"), "Privacy");
        assert!(h.can_go_back());
        assert!(!h.can_go_forward());
        h.back();
        assert_eq!(h.current().title, "Settings");
        h.back();
        assert_eq!(h.current().title, "Home");
        assert!(!h.can_go_back());
        h.forward();
        assert_eq!(h.current().title, "Settings");
    }

    #[test]
    fn push_truncates_forward() {
        let mut h = SessionHistory::new(url("about:home"), "Home");
        h.push(url("about:settings"), "Settings");
        h.back();
        h.push(url("about:privacy"), "Privacy");
        assert!(!h.can_go_forward());
        assert_eq!(h.len(), 2);
        assert_eq!(h.current().title, "Privacy");
    }
}
