use std::io;

/// Error type for crate boundaries.
#[derive(Debug, thiserror::Error)]
pub enum FrihartError {
    #[error("{0}")]
    Message(String),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("config: {0}")]
    Config(String),

    #[error("profile: {0}")]
    Profile(String),

    #[error("profile is locked by another process (pid {pid})")]
    ProfileLocked { pid: u32 },

    #[error("unsupported on this platform: {0}")]
    Unsupported(&'static str),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("navigation: {0}")]
    Navigation(String),

    #[error("network: {0}")]
    Network(String),
}

impl FrihartError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn profile(msg: impl Into<String>) -> Self {
        Self::Profile(msg.into())
    }

    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, FrihartError>;
