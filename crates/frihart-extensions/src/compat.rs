//! What a Firefox permission means on Frihart today.

/// How far we are from honouring a WebExtensions permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiSupport {
    /// Frihart already does this job natively (not via the add-on).
    Native,
    /// We will implement this API on our engine. Not runnable yet.
    Planned,
    /// Firefox-only or something we refuse (remote code, privileged debug).
    Unsupported,
}

impl ApiSupport {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "native in Frihart",
            Self::Planned => "planned WebExtensions API",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Classify a `permissions` / `host_permissions` entry.
pub fn classify(permission: &str) -> ApiSupport {
    match permission {
        "webRequest" | "webRequestBlocking" | "declarativeNetRequest" => ApiSupport::Native,
        "contextualIdentities" => ApiSupport::Native,
        "privacy" => ApiSupport::Native,
        "proxy" => ApiSupport::Planned,
        "tabs" | "activeTab" | "windows" | "sessions" => ApiSupport::Planned,
        "storage" | "unlimitedStorage" | "cookies" | "webNavigation" => ApiSupport::Planned,
        "bookmarks" | "history" | "downloads" | "notifications" | "menus" | "contextMenus" => {
            ApiSupport::Planned
        }
        "theme" | "alarms" | "idle" | "clipboardRead" | "clipboardWrite" => ApiSupport::Planned,
        "nativeMessaging" | "geckoProfiler" | "mozillaAddons" | "normandyAddonStudy" => {
            ApiSupport::Unsupported
        }
        "management" | "debugger" | "devtools" => ApiSupport::Unsupported,
        other if other.contains("://") || other.starts_with('<') || other.starts_with('*') => {
            ApiSupport::Planned
        }
        _ => ApiSupport::Planned,
    }
}

/// One-line reason an add-on is dormant even if installed.
pub fn dormant_reason(needs_javascript: bool) -> &'static str {
    if needs_javascript {
        "installed, not executed: Frihart has no JS engine yet (Phase 7)"
    } else {
        "installed, waiting on the matching native API"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocker_permissions_are_native() {
        assert_eq!(classify("webRequest"), ApiSupport::Native);
        assert_eq!(classify("contextualIdentities"), ApiSupport::Native);
    }

    #[test]
    fn privileged_firefox_apis_are_refused() {
        assert_eq!(classify("geckoProfiler"), ApiSupport::Unsupported);
        assert_eq!(classify("debugger"), ApiSupport::Unsupported);
    }
}
