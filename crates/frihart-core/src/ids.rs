use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TAB: AtomicU64 = AtomicU64::new(1);
static NEXT_WINDOW: AtomicU64 = AtomicU64::new(1);
static NEXT_DOCUMENT: AtomicU64 = AtomicU64::new(1);

/// Identifier of a tab inside a window.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TabId(pub u64);

impl TabId {
    pub fn new() -> Self {
        Self(NEXT_TAB.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TabId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tab-{}", self.0)
    }
}

/// Identifier of a top-level browser window.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct WindowId(pub u64);

impl WindowId {
    pub fn new() -> Self {
        Self(NEXT_WINDOW.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WindowId {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifier of a loaded document instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DocumentId(pub u64);

impl DocumentId {
    pub fn new() -> Self {
        Self(NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}
