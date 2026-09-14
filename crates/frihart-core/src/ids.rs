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

/// First-class identity container. Cookies, cache, and storage never
/// cross this boundary once the network stack exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ContainerId(pub u32);

impl ContainerId {
    pub const PERSONAL: Self = Self(1);
    pub const WORK: Self = Self(2);
    pub const BANKING: Self = Self(3);
    pub const SHOPPING: Self = Self(4);

    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug.trim().to_ascii_lowercase().as_str() {
            "personal" | "1" => Some(Self::PERSONAL),
            "work" | "2" => Some(Self::WORK),
            "banking" | "3" => Some(Self::BANKING),
            "shopping" | "4" => Some(Self::SHOPPING),
            _ => None,
        }
    }

    pub fn slug(self) -> &'static str {
        match self.0 {
            1 => "personal",
            2 => "work",
            3 => "banking",
            4 => "shopping",
            _ => "unknown",
        }
    }

    pub fn defaults() -> [Self; 4] {
        [Self::PERSONAL, Self::WORK, Self::BANKING, Self::SHOPPING]
    }
}

impl fmt::Display for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.slug())
    }
}

/// Network circuit. Private windows still use Direct on the wire;
/// amnesia is the profile, not a fourth net.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum CircuitKind {
    #[default]
    Direct,
    Tor,
    I2p,
}

impl CircuitKind {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Tor => "tor",
            Self::I2p => "i2p",
        }
    }

    pub fn from_slug(s: &str) -> Self {
        match s {
            "tor" => Self::Tor,
            "i2p" => Self::I2p,
            _ => Self::Direct,
        }
    }
}

/// Site isolation key: scheme + host + container + circuit.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct IsolationKey {
    pub scheme: String,
    pub host: String,
    pub container: ContainerId,
    pub circuit: CircuitKind,
}

impl IsolationKey {
    pub fn new(scheme: impl Into<String>, host: impl Into<String>, container: ContainerId) -> Self {
        Self {
            scheme: scheme.into(),
            host: host.into(),
            container,
            circuit: CircuitKind::Direct,
        }
    }

    pub fn with_circuit(mut self, circuit: CircuitKind) -> Self {
        self.circuit = circuit;
        self
    }

    pub fn from_url(url: &url::Url, container: ContainerId) -> Self {
        Self::from_url_on(url, container, CircuitKind::Direct)
    }

    pub fn from_url_on(url: &url::Url, container: ContainerId, circuit: CircuitKind) -> Self {
        Self::new(
            url.scheme(),
            url.host_str().unwrap_or("").to_ascii_lowercase(),
            container,
        )
        .with_circuit(circuit)
    }

    pub fn label(&self) -> String {
        format!(
            "{}://{} · {} · {}",
            self.scheme,
            self.host,
            self.container,
            self.circuit.slug()
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containers_do_not_share_isolation() {
        let a = IsolationKey::new("https", "bank.example", ContainerId::BANKING);
        let b = IsolationKey::new("https", "bank.example", ContainerId::SHOPPING);
        assert_ne!(a, b);
    }

    #[test]
    fn scheme_splits_isolation() {
        let a = IsolationKey::from_url(
            &url::Url::parse("https://a.test").unwrap(),
            ContainerId::PERSONAL,
        );
        let b = IsolationKey::from_url(
            &url::Url::parse("http://a.test").unwrap(),
            ContainerId::PERSONAL,
        );
        let c = IsolationKey::from_url(
            &url::Url::parse("https://b.test").unwrap(),
            ContainerId::PERSONAL,
        );
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn circuit_splits_isolation() {
        let url = url::Url::parse("https://a.test").unwrap();
        let direct = IsolationKey::from_url(&url, ContainerId::PERSONAL);
        let tor = IsolationKey::from_url_on(&url, ContainerId::PERSONAL, CircuitKind::Tor);
        let i2p = IsolationKey::from_url_on(&url, ContainerId::PERSONAL, CircuitKind::I2p);
        assert_ne!(direct, tor);
        assert_ne!(tor, i2p);
        assert_eq!(direct.circuit, CircuitKind::Direct);
    }
}
