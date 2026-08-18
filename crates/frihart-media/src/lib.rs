//! Media decode contracts. Autoplay stays off.

#![forbid(unsafe_code)]

use frihart_core::{FrihartError, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Gif,
    Webp,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Policy {
    pub autoplay: bool,
    pub decode: bool,
}

pub fn sniff(bytes: &[u8]) -> Kind {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Kind::Png
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Kind::Jpeg
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Kind::Gif
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Kind::Webp
    } else {
        Kind::Unknown
    }
}

pub fn decode(bytes: &[u8], policy: Policy) -> Result<(u32, u32, Vec<u8>)> {
    let _ = sniff(bytes);
    if !policy.decode {
        return Err(FrihartError::Message("media decode later".into()));
    }
    Err(FrihartError::Message("media decode later".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_magic() {
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0, 0, 0, 0]), Kind::Png);
        assert_eq!(sniff(b"GIF89a...."), Kind::Gif);
        assert_eq!(sniff(b"xxxx"), Kind::Unknown);
        assert!(!Policy::default().autoplay);
        assert!(decode(b"", Policy::default()).is_err());
    }
}
