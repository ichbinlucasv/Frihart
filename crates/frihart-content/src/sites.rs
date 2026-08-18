//! Honest “sites we claim” list. Compatibility is per document, not “the web”.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimStatus {
    Internal,
    Claimed,
    Target,
    NeedsJs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteClaim {
    pub url: &'static str,
    pub name: &'static str,
    pub status: ClaimStatus,
    pub note: &'static str,
}

impl ClaimStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Claimed => "claimed",
            Self::Target => "target",
            Self::NeedsJs => "needs JS — not claimed",
        }
    }
}

pub fn claims() -> &'static [SiteClaim] {
    &[
        SiteClaim {
            url: "about:home",
            name: "Frihart home",
            status: ClaimStatus::Internal,
            note: "our chrome",
        },
        SiteClaim {
            url: "about:engine",
            name: "Engine spine",
            status: ClaimStatus::Internal,
            note: "pipeline status",
        },
        SiteClaim {
            url: "https://example.com/",
            name: "example.com",
            status: ClaimStatus::Claimed,
            note: "h1 1.5em, two paragraphs, IANA :link; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc1918.html",
            name: "RFC 1918 HTML",
            status: ClaimStatus::Claimed,
            note: "9 pre pages, span.h1 title, 10/8 172.16/12 192.168/16; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.gnu.org/philosophy/",
            name: "GNU philosophy",
            status: ClaimStatus::Target,
            note: "mostly static essays",
        },
        SiteClaim {
            url: "https://suckless.org/",
            name: "suckless.org",
            status: ClaimStatus::Claimed,
            note: "title, News, dwm/dmenu links as https; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.kernel.org/",
            name: "kernel.org",
            status: ClaimStatus::Target,
            note: "simple landing + docs",
        },
        SiteClaim {
            url: "https://en.wikipedia.org/",
            name: "Wikipedia",
            status: ClaimStatus::NeedsJs,
            note: "infobox / more CSS first; do not claim",
        },
        SiteClaim {
            url: "https://github.com/",
            name: "GitHub",
            status: ClaimStatus::NeedsJs,
            note: "JS app",
        },
        SiteClaim {
            url: "https://mail.proton.me/",
            name: "Proton Mail",
            status: ClaimStatus::NeedsJs,
            note: "JS app",
        },
    ]
}

pub fn claimed_count() -> usize {
    claims()
        .iter()
        .filter(|s| matches!(s.status, ClaimStatus::Internal | ClaimStatus::Claimed))
        .count()
}

pub fn public_claimed() -> usize {
    claims()
        .iter()
        .filter(|s| matches!(s.status, ClaimStatus::Claimed))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_honest() {
        assert!(claimed_count() >= 3);
        assert_eq!(public_claimed(), 3);
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::NeedsJs && s.url.contains("github"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("example.com"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc1918"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("suckless"))
        );
        assert!(claims().iter().all(|s| s.status != ClaimStatus::NeedsJs
            || s.note.contains("not claim")
            || s.note.contains("JS")));
    }
}
