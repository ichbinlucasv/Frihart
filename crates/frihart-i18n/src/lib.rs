//! Chrome locale. Default English. No network.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    pub lang: &'static str,
}

impl Default for Locale {
    fn default() -> Self {
        Self { lang: "en" }
    }
}

pub fn t(lang: &str, key: &str) -> String {
    match (lang, key) {
        (_, "wipe") => "Wipe".into(),
        (_, "reset") => "Reset".into(),
        (_, "shred") => "Shred".into(),
        (_, "profiles") => "Profiles".into(),
        (_, "autofill") => "Autofill".into(),
        (_, "bookmarks") => "Bookmarks".into(),
        (_, "history") => "History".into(),
        (_, "settings") => "Settings".into(),
        (_, "privacy") => "Privacy".into(),
        (_, "engine") => "Engine".into(),
        (_, "print") => "Print".into(),
        (_, "downloads") => "Downloads".into(),
        (_, "processes") => "Processes".into(),
        (_, "new-tab") => "New tab".into(),
        (_, "fill-identity") => "Fill identity".into(),
        (_, "password-manager") => "Password manager".into(),
        _ => key.to_string(),
    }
}

impl Locale {
    pub fn get(self, key: &str) -> String {
        t(self.lang, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_key() {
        assert_eq!(t("en", "wipe"), "Wipe");
        assert_eq!(t("xx", "unknown-key"), "unknown-key");
        assert_eq!(Locale::default().get("engine"), "Engine");
    }
}
