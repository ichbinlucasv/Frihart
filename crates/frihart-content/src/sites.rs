//! Honest “sites we claim” list. Compatibility is per document, not “the web”.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimStatus {
    Internal,
    Claimed,
    /// Next named document; may be unused when the queue is empty.
    #[allow(dead_code)]
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
            status: ClaimStatus::Claimed,
            note: "title, four freedoms, essay list; mdash/ldquo; live HTML 2026-08-18",
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
            status: ClaimStatus::Claimed,
            note: "title, nav links, releases table (mainline/stable); live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://docs.kernel.org/",
            name: "docs.kernel.org",
            status: ClaimStatus::Claimed,
            note: "Sphinx index, toctree links, no ¶; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.ietf.org/",
            name: "ietf.org",
            status: ClaimStatus::Claimed,
            note: "Welcome, IETF 126/127, standards copy; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/",
            name: "rfc-editor.org",
            status: ClaimStatus::Claimed,
            note: "SSR index, official home, latest RFC cards as links; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.w3.org/",
            name: "w3.org",
            status: ClaimStatus::Claimed,
            note: "Making the web work, TPAC 2026, ARIA news link; live HTML 2026-08-18",
        },
        SiteClaim {
            url: "https://www.w3.org/TR/",
            name: "w3.org/TR",
            status: ClaimStatus::Claimed,
            note: "1236 reports / 288 families, RDF 1.2 Turtle heading-link, Tags/Deliverers; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.w3.org/TR/webarch/",
            name: "w3.org/TR/webarch",
            status: ClaimStatus::Claimed,
            note: "REC 2004, Abstract, Identification/Interaction/Data Formats, This version link; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc9110.html",
            name: "RFC 9110 HTML",
            status: ClaimStatus::Claimed,
            note: "HTTP Semantics, Abstract, Methods/GET, pre ABNF; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.w3.org/TR/WCAG22/",
            name: "WCAG 2.2",
            status: ClaimStatus::Claimed,
            note: "REC 2024, Abstract, Perceivable/Operable/Understandable, Guideline 1.1; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc8446.html",
            name: "RFC 8446 HTML",
            status: ClaimStatus::Claimed,
            note: "TLS 1.3, Abstract, Handshake/ClientHello, pre pages; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc5280.html",
            name: "RFC 5280 HTML",
            status: ClaimStatus::Claimed,
            note: "X.509 PKI, Abstract, Certificate/CRL, pre pages; live HTML 2026-09-14",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc8032.html",
            name: "RFC 8032 HTML",
            status: ClaimStatus::Claimed,
            note: "EdDSA / Ed25519, Abstract, edwards25519, pre pages; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc7748.html",
            name: "RFC 7748 HTML",
            status: ClaimStatus::Claimed,
            note: "Curve25519 / X25519, Abstract, Elliptic Curves for Security, pre pages; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.rfc-editor.org/rfc/rfc5869.html",
            name: "RFC 5869 HTML",
            status: ClaimStatus::Claimed,
            note: "HKDF, Abstract, Extract-and-Expand / HKDF-Extract, pre pages; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openbsd.org/",
            name: "openbsd.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Security, FREE, OpenSSH, remote holes / 7.9; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openssh.org/",
            name: "openssh.com",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Security, OpenSSH 10.5, ssh/scp/sftp; openssh.com→openssh.org; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.libressl.org/",
            name: "libressl.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals, LibreSSL 4.3.2, libcrypto/libssl/libtls; libressl.org→www; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openbsdfoundation.org/",
            name: "openbsdfoundation.org",
            status: ClaimStatus::Claimed,
            note: "title, Funding for OpenBSD, Bylaws/Donations/Activities, 2026 Campaign; openbsdfoundation.org→www; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openbgpd.org/",
            name: "openbgpd.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Manual Pages, FREE BGP / Border Gateway Protocol; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.opensmtpd.org/",
            name: "opensmtpd.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Manual Pages, FREE SMTP / RFC 5321; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openntpd.org/",
            name: "openntpd.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Manual Pages, FREE Network Time Protocol; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.openiked.org/",
            name: "openiked.org",
            status: ClaimStatus::Claimed,
            note: "title, About/Project Goals/Manual Pages, FREE IKEv2 / Internet Key Exchange; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.bearssl.org/",
            name: "bearssl.org",
            status: ClaimStatus::Claimed,
            note: "title, Overview/Status/Download, SSL/TLS in C, constant-time; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.wireguard.com/",
            name: "wireguard.com",
            status: ClaimStatus::Claimed,
            note: "title, Installation/Quick Start/Protocol, Cryptokey Routing; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://gnupg.org/",
            name: "gnupg.org",
            status: ClaimStatus::Claimed,
            note: "title, Software/Download/Documentation, OpenPGP / GnuPG; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://noiseprotocol.org/",
            name: "noiseprotocol.org",
            status: ClaimStatus::Claimed,
            note: "title, Resources/Specs/Code, Noise Protocol Framework; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://curl.se/",
            name: "curl.se",
            status: ClaimStatus::Claimed,
            note: "title, Download/Documentation/libcurl, command line tool and library; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://signal.org/",
            name: "signal.org",
            status: ClaimStatus::Claimed,
            note: "title, Get Signal/Blog/Developers/Donate, Speak Freely / Signal Protocol; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://c2sp.org/age",
            name: "c2sp.org/age",
            status: ClaimStatus::Claimed,
            note: "title age|C2SP, h1 age, file encryption format / recipients; /age→/age@v1.1.0 same host; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://www.torproject.org/",
            name: "torproject.org",
            status: ClaimStatus::Claimed,
            note: "title Anonymity Online, Browse Privately/Explore Freely, Download Tor Browser; apex 301→www; live HTML 2026-09-22",
        },
        SiteClaim {
            url: "https://age-encryption.org/",
            name: "age-encryption.org",
            status: ClaimStatus::NeedsJs,
            note: "Forbidden: apex 302 → github.com/FiloSottile/age; do not claim",
        },
        SiteClaim {
            url: "https://libsodium.org/",
            name: "libsodium.org",
            status: ClaimStatus::NeedsJs,
            note: "Forbidden: apex/www 307 → doc.libsodium.org 302 → libsodium.gitbook.io/doc; do not claim",
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
        assert_eq!(public_claimed(), 34);
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
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("gnu.org/philosophy"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("www.kernel.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("docs.kernel.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("www.ietf.org"))
        );
        assert!(claims().iter().any(|s| s.status == ClaimStatus::Claimed
            && s.url.contains("rfc-editor.org")
            && !s.url.contains("rfc1918")));
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url == "https://www.w3.org/")
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url == "https://www.w3.org/TR/")
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("webarch"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc9110"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("WCAG22"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc8446"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc5280"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc8032"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc7748"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("rfc5869"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("openbsd.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("openssh.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("libressl.org"))
        );
        assert!(
            claims().iter().any(
                |s| s.status == ClaimStatus::Claimed && s.url.contains("openbsdfoundation.org")
            )
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("openbgpd.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("opensmtpd.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("openntpd.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("openiked.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("bearssl.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("wireguard.com"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("gnupg.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("noiseprotocol.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("curl.se"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("signal.org"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("c2sp.org/age"))
        );
        assert!(
            claims()
                .iter()
                .any(|s| s.status == ClaimStatus::Claimed && s.url.contains("torproject.org"))
        );
        assert!(
            claims().iter().any(|s| {
                s.status == ClaimStatus::NeedsJs
                    && s.url.contains("age-encryption.org")
                    && s.note.contains("Forbidden")
            })
        );
        assert!(
            claims().iter().any(|s| {
                s.status == ClaimStatus::NeedsJs
                    && s.url.contains("libsodium.org")
                    && s.note.contains("Forbidden")
                    && s.note.contains("gitbook")
            })
        );
        assert!(claims().iter().all(|s| s.status != ClaimStatus::NeedsJs
            || s.note.contains("not claim")
            || s.note.contains("JS")));
    }
}
