//! Typed IPC. In-process now; process split later (Phase 6).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use frihart_core::{ContainerId, IsolationKey, TabId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessKind {
    Chrome,
    Network,
    Content,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub from: ProcessKind,
    pub to: ProcessKind,
    pub message: Message,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Navigate {
        tab: u64,
        url: String,
        isolation: IsolationWire,
    },
    Fetch {
        url: String,
        isolation: IsolationWire,
    },
    FetchOk {
        status: u16,
        bytes: usize,
    },
    FetchErr {
        reason: String,
    },
    PaintReady {
        ops: usize,
    },
    ContentCrashed {
        isolation: IsolationWire,
    },
    KillContent,
    Ping,
    Pong,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationWire {
    pub scheme: String,
    pub host: String,
    pub container: u32,
}

impl IsolationWire {
    pub fn from_key(key: &IsolationKey) -> Self {
        Self {
            scheme: key.scheme.clone(),
            host: key.host.clone(),
            container: key.container.0,
        }
    }

    pub fn to_key(&self) -> IsolationKey {
        IsolationKey::new(
            self.scheme.clone(),
            self.host.clone(),
            ContainerId(self.container),
        )
    }
}

pub fn encode(msg: &Message) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

pub fn decode(bytes: &[u8]) -> Result<Message, serde_json::Error> {
    serde_json::from_slice(bytes)
}

pub fn encode_envelope(env: &Envelope) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(env)
}

/// In-process bus. Same types as the future socket.
#[derive(Default)]
pub struct Bus {
    pub log: Vec<Envelope>,
}

impl Bus {
    pub fn send(&mut self, env: Envelope) {
        self.log.push(env);
    }

    pub fn last_to(&self, to: ProcessKind) -> Option<&Envelope> {
        self.log.iter().rev().find(|e| e.to == to)
    }
}

pub fn navigate_msg(tab: TabId, url: &str, key: &IsolationKey) -> Message {
    Message::Navigate {
        tab: tab.0,
        url: url.into(),
        isolation: IsolationWire::from_key(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = IsolationKey::new("https", "ex.test", ContainerId::PERSONAL);
        let m = navigate_msg(TabId(1), "about:home", &key);
        let b = encode(&m).unwrap();
        assert_eq!(decode(&b).unwrap(), m);
    }

    #[test]
    fn bus_records_chrome_to_net() {
        let mut bus = Bus::default();
        bus.send(Envelope {
            from: ProcessKind::Chrome,
            to: ProcessKind::Network,
            message: Message::Ping,
        });
        assert!(matches!(
            bus.last_to(ProcessKind::Network).map(|e| &e.message),
            Some(Message::Ping)
        ));
    }
}
