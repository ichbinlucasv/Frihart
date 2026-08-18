use frihart_config::Prefs;
use frihart_core::{APP_NAME, FROZEN_USER_AGENT, VERSION};
use frihart_privacy::Policy;
use frihart_profile::Profile;
use url::Url;

use crate::document::{Block, Document, InternalPage};

/// A preference exposed as a toggle on an internal page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefToggle {
    HttpsOnly,
    ResistFingerprinting,
    PersistHistory,
    PersistCookies,
    SendGpc,
    Javascript,
    WebRtc,
    RestoreSession,
    ThirdPartyCookies,
    Blocker,
    Containers,
    DarkMode,
    Translate,
    DismissWelcome,
    Autofill,
}

impl PrefToggle {
    pub fn apply(self, prefs: &mut Prefs) {
        match self {
            Self::HttpsOnly => prefs.privacy.https_only = !prefs.privacy.https_only,
            Self::ResistFingerprinting => {
                prefs.privacy.resist_fingerprinting = !prefs.privacy.resist_fingerprinting;
            }
            Self::PersistHistory => prefs.privacy.persist_history = !prefs.privacy.persist_history,
            Self::PersistCookies => prefs.privacy.persist_cookies = !prefs.privacy.persist_cookies,
            Self::SendGpc => prefs.privacy.send_gpc = !prefs.privacy.send_gpc,
            Self::Javascript => prefs.privacy.javascript = !prefs.privacy.javascript,
            Self::WebRtc => prefs.privacy.webrtc = !prefs.privacy.webrtc,
            Self::RestoreSession => prefs.general.restore_session = !prefs.general.restore_session,
            Self::ThirdPartyCookies => {
                prefs.privacy.third_party_cookies = !prefs.privacy.third_party_cookies;
            }
            Self::Blocker => prefs.privacy.blocker = !prefs.privacy.blocker,
            Self::Containers => prefs.privacy.containers = !prefs.privacy.containers,
            Self::DarkMode => prefs.content.dark_mode = !prefs.content.dark_mode,
            Self::Translate => prefs.translate.enabled = !prefs.translate.enabled,
            Self::DismissWelcome => prefs.general.welcome_seen = true,
            Self::Autofill => prefs.autofill.enabled = !prefs.autofill.enabled,
        }
    }
}

pub fn is_known(name: &str) -> bool {
    matches!(
        name,
        "blank"
            | "home"
            | "newtab"
            | "settings"
            | "privacy"
            | "config"
            | "license"
            | "credits"
            | "keyboard"
            | "roadmap"
            | "about"
            | "frihart"
            | "bookmarks"
            | "history"
            | "containers"
            | "blocker"
            | "translate"
            | "search"
            | "tor"
            | "vpn"
            | "extensions"
            | "addons"
            | "welcome"
            | "support"
            | "pay"
            | "shred"
            | "profiles"
            | "pass"
            | "passwords"
            | "autofill"
            | "engine"
            | "processes"
            | "print"
            | "downloads"
            | "linux"
            | "distros"
            | "campaigns"
            | "script"
            | "js"
    )
}

pub fn page(name: &str, url: &Url, prefs: &Prefs, profile: &Profile) -> Document {
    match name {
        "blank" => Document::Blank,
        "home" | "newtab" => home(url, prefs, profile),
        "settings" => settings(url, prefs),
        "privacy" => privacy(url, prefs),
        "config" => config(url, prefs),
        "license" => license(url),
        "credits" => credits(url),
        "keyboard" => keyboard(url),
        "roadmap" => roadmap(url),
        "about" | "frihart" => about(url),
        "bookmarks" => bookmarks(url, profile),
        "history" => history(url, profile),
        "containers" => containers(url, profile),
        "blocker" => blocker(url, prefs),
        "translate" => translate(url, prefs),
        "search" => search(url, prefs),
        "tor" => tor(url, prefs),
        "vpn" => vpn(url, prefs),
        "extensions" | "addons" => extensions(url, profile),
        "welcome" => welcome(url, prefs),
        "support" | "pay" => support(url, prefs),
        "shred" => shred_page(url),
        "profiles" => profiles(url, profile),
        "pass" | "passwords" => passwords(url, prefs),
        "autofill" => autofill_page(url, prefs, profile),
        "engine" => engine_page(url),
        "processes" => processes_page(url),
        "print" => print_page(url),
        "downloads" => downloads_page(url, profile),
        "linux" | "distros" => linux_page(url),
        "campaigns" => campaigns_page(url),
        "script" | "js" => script_page(url, prefs),
        other => Document::internal(InternalPage {
            title: "Unknown page".into(),
            url: url.clone(),
            blocks: vec![
                Block::Hero {
                    title: format!("about:{other} is not a page"),
                    subtitle: "Internal pages are listed on about:home.".into(),
                },
                Block::Link {
                    label: "Home".into(),
                    href: "about:home".into(),
                },
            ],
        }),
    }
}

fn home(url: &Url, prefs: &Prefs, profile: &Profile) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: APP_NAME.into(),
            subtitle: "Libertarian. Inspired by LibreWolf. Original Rust. Black and yellow.".into(),
        },
        Block::Paragraph(format!(
            "Linux first — Arch, CachyOS, Fedora, Mint, then Tails and Qubes. \
             Campaign B/C/D. Version {VERSION}. Profile “{}”.",
            profile.name()
        )),
        Block::Heading("Pages".into()),
        Block::Link {
            label: "Settings".into(),
            href: "about:settings".into(),
        },
        Block::Link {
            label: "Privacy".into(),
            href: "about:privacy".into(),
        },
        Block::Link {
            label: "Support".into(),
            href: "about:support".into(),
        },
        Block::Link {
            label: "Wipe / shred".into(),
            href: "about:shred".into(),
        },
        Block::Link {
            label: "Profiles".into(),
            href: "about:profiles".into(),
        },
        Block::Link {
            label: "Passwords".into(),
            href: "about:pass".into(),
        },
        Block::Link {
            label: "Engine".into(),
            href: "about:engine".into(),
        },
        Block::Link {
            label: "Campaigns".into(),
            href: "about:campaigns".into(),
        },
        Block::Link {
            label: "Linux homes".into(),
            href: "about:linux".into(),
        },
        Block::Link {
            label: "Script (off)".into(),
            href: "about:script".into(),
        },
        Block::Link {
            label: "Processes".into(),
            href: "about:processes".into(),
        },
        Block::Link {
            label: "Print".into(),
            href: "about:print".into(),
        },
        Block::Link {
            label: "Downloads".into(),
            href: "about:downloads".into(),
        },
        Block::Link {
            label: "Preferences (about:config)".into(),
            href: "about:config".into(),
        },
        Block::Link {
            label: "Keyboard shortcuts".into(),
            href: "about:keyboard".into(),
        },
        Block::Link {
            label: "Roadmap".into(),
            href: "about:roadmap".into(),
        },
        Block::Link {
            label: "Containers".into(),
            href: "about:containers".into(),
        },
        Block::Link {
            label: "Blocker".into(),
            href: "about:blocker".into(),
        },
        Block::Link {
            label: "Translator".into(),
            href: "about:translate".into(),
        },
        Block::Link {
            label: "Search".into(),
            href: "about:search".into(),
        },
        Block::Link {
            label: "Tor".into(),
            href: "about:tor".into(),
        },
        Block::Link {
            label: "VPN".into(),
            href: "about:vpn".into(),
        },
        Block::Link {
            label: "Extensions (community)".into(),
            href: "about:extensions".into(),
        },
        Block::Link {
            label: "Bookmarks".into(),
            href: "about:bookmarks".into(),
        },
        Block::Link {
            label: "History".into(),
            href: "about:history".into(),
        },
        Block::Link {
            label: "License".into(),
            href: "about:license".into(),
        },
    ];

    if !profile.bookmarks().items.is_empty() {
        blocks.push(Block::Heading("Bookmarks".into()));
        for mark in &profile.bookmarks().items {
            blocks.push(Block::Link {
                label: mark.title.clone(),
                href: mark.url.clone(),
            });
        }
    }

    if prefs.privacy.persist_history {
        let recent = profile.history().recent(8);
        if !recent.is_empty() {
            blocks.push(Block::Heading("Recent".into()));
            for entry in recent {
                let label = if entry.title.is_empty() {
                    entry.url.clone()
                } else {
                    format!("{}  ·  {}", entry.title, entry.url)
                };
                blocks.push(Block::Link {
                    label,
                    href: entry.url.clone(),
                });
            }
        }
    }

    blocks.push(Block::Note(
        "https:// URLs fetch over rustls. HTML goes through the subset pipeline. \
         JS stays off. Tor tabs refuse clearnet."
            .into(),
    ));

    Document::internal(InternalPage {
        title: "Home".into(),
        url: url.clone(),
        blocks,
    })
}

fn welcome(url: &Url, prefs: &Prefs) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Frihart".into(),
            subtitle: "Linux is free. No accounts. No login store.".into(),
        },
        Block::Paragraph(frihart_core::price_label().into()),
        Block::Paragraph(
            "Any OS that is not Linux: €100 lifetime. Pay with Monero, Bitcoin, \
             or fiat. Local key only. Nothing phones home."
                .into(),
        ),
        Block::Heading("Support".into()),
        Block::Paragraph(
            "Monero, Bitcoin, or fiat if you set an address in prefs. This page \
             is local."
                .into(),
        ),
    ];
    blocks.extend(support_blocks(prefs));
    blocks.extend([
        Block::Heading("Data".into()),
        Block::Link {
            label: "Wipe session".into(),
            href: "frihart:wipe".into(),
        },
        Block::Link {
            label: "Shred profile".into(),
            href: "frihart:shred".into(),
        },
        toggle(
            PrefToggle::DismissWelcome,
            "Don't show this again",
            "Stored only in your profile.",
            prefs.general.welcome_seen,
        ),
        Block::Link {
            label: "Continue".into(),
            href: "about:home".into(),
        },
    ]);
    Document::internal(InternalPage {
        title: "Welcome".into(),
        url: url.clone(),
        blocks,
    })
}

fn support(url: &Url, prefs: &Prefs) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Support".into(),
            subtitle: frihart_core::price_label().into(),
        },
        Block::List(vec![
            "Linux: free".into(),
            "Android / Windows / macOS / other: €100 lifetime".into(),
            "Pay: Monero, Bitcoin, or fiat".into(),
        ]),
        Block::Note(
            "No Frihart account. No license server. Paid ports will use a key \
             you keep on disk."
                .into(),
        ),
        Block::Heading("Donate".into()),
    ];
    blocks.extend(support_blocks(prefs));
    blocks.push(Block::Link {
        label: "Home".into(),
        href: "about:home".into(),
    });
    Document::internal(InternalPage {
        title: "Support".into(),
        url: url.clone(),
        blocks,
    })
}

fn support_blocks(prefs: &Prefs) -> Vec<Block> {
    let mut blocks = Vec::new();
    if prefs.support.xmr.is_empty()
        && prefs.support.btc.is_empty()
        && prefs.support.fiat_url.is_empty()
    {
        blocks.push(Block::Note(
            "Set support.xmr, support.btc, and optional support.fiat_url in prefs.toml.".into(),
        ));
    }
    if !prefs.support.xmr.is_empty() {
        blocks.push(Block::KeyValue {
            key: "XMR".into(),
            value: prefs.support.xmr.clone(),
        });
    }
    if !prefs.support.btc.is_empty() {
        blocks.push(Block::KeyValue {
            key: "BTC".into(),
            value: prefs.support.btc.clone(),
        });
    }
    if !prefs.support.fiat_url.is_empty() {
        blocks.push(Block::Link {
            label: "Fiat".into(),
            href: prefs.support.fiat_url.clone(),
        });
    }
    blocks
}

fn shred_page(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Shred".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Wipe / shred".into(),
                subtitle: "This profile only. Other profiles stay.".into(),
            },
            Block::Paragraph(
                "Wipe: like a new session. Tabs, cookies, history gone. Bookmarks stay.".into(),
            ),
            Block::Paragraph("Reset: this profile like new. Prefs default. Bookmarks stay.".into()),
            Block::Paragraph(
                "Shred: overwrite this profile on disk. Other named profiles stay.".into(),
            ),
            Block::Link {
                label: "Wipe".into(),
                href: "frihart:wipe".into(),
            },
            Block::Link {
                label: "Reset this profile".into(),
                href: "frihart:reset".into(),
            },
            Block::Link {
                label: "Shred this profile".into(),
                href: "frihart:shred".into(),
            },
            Block::Link {
                label: "Profiles".into(),
                href: "about:profiles".into(),
            },
        ],
    })
}

fn profiles(url: &Url, profile: &Profile) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Profiles".into(),
            subtitle: format!("active: {}", profile.name()),
        },
        Block::Paragraph(
            "Each profile has its own bookmarks, cookies, and prefs. Wipe one \
             without touching the others."
                .into(),
        ),
        Block::Heading("Switch".into()),
    ];
    for name in frihart_profile::list_profiles() {
        blocks.push(Block::Link {
            label: name.clone(),
            href: format!("frihart:profile/{name}"),
        });
    }
    blocks.extend([
        Block::Heading("Create".into()),
        Block::Link {
            label: "work".into(),
            href: "frihart:profile-new/work".into(),
        },
        Block::Link {
            label: "travel".into(),
            href: "frihart:profile-new/travel".into(),
        },
        Block::Note("Or type frihart:profile-new/yourname in the URL bar.".into()),
    ]);
    Document::internal(InternalPage {
        title: "Profiles".into(),
        url: url.clone(),
        blocks,
    })
}

fn engine_page(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Engine".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Engine map".into(),
                subtitle: "HTML → CSS → style → layout → display list.".into(),
            },
            Block::List(vec![
                "1 chrome / profiles / wipe / HiDPI scale".into(),
                "2 rustls / cookies / HTTPS-only / downloads dest".into(),
                "3 html tokenizer + arena DOM + tables".into(),
                "4 css / style / block layout / gfx ops".into(),
                "5 forms GET + POST encode (secrets skipped)".into(),
                "6 ipc envelopes (in-process bus)".into(),
                "7 js types; exec and fingerprint APIs off".into(),
                "8–10 platform dirs (Win/Mac/Android roots)".into(),
                "11 media sniff; autoplay off".into(),
                "12 pipeline Frame from HTML bytes".into(),
                "13 i18n chrome catalog (en default)".into(),
                "14 print → local PostScript".into(),
                "15 extension host; runtime dormant".into(),
            ]),
            Block::Note(
                "Put extra CSS in this profile's user.css. Origin: UA, then user, \
                 then author <style>."
                    .into(),
            ),
            Block::Link {
                label: "Processes".into(),
                href: "about:processes".into(),
            },
        ],
    })
}

fn processes_page(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Processes".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Processes".into(),
                subtitle: "Campaign E. One content slot per isolation key.".into(),
            },
            Block::List(vec![
                "chrome — only process that may touch the full profile".into(),
                "network — rustls, cookies (same process until split)".into(),
                "content — one slot per scheme + host + container".into(),
                "https://a.test and https://b.test never share a slot".into(),
                "http and https of the same host never share a slot".into(),
                "content may not read prefs.toml or open a raw socket".into(),
            ]),
            Block::KeyValue {
                key: "landlock abi".into(),
                value: match frihart_platform::landlock_abi() {
                    Some(v) => format!("{v}"),
                    None => "unavailable".into(),
                },
            },
            Block::Note(
                "One long-lived `frihart --content-worker` per isolation key. \
                 no_new_privs + landlock. Chrome never applies the sandbox. \
                 Crash falls back in-process. Fields live on the display list. \
                 seccomp comes next."
                    .into(),
            ),
        ],
    })
}

fn campaigns_page(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Campaigns".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Campaigns".into(),
                subtitle: "A B C closed (v0.1.0). D+E next. H and I parked.".into(),
            },
            Block::List(vec![
                "A Foundation — closed".into(),
                "B Chrome — closed".into(),
                "C Network OPSEC — closed".into(),
                "D Engine — open (you are here)".into(),
                "E Isolation — open (landlock in child; no process split yet)".into(),
                "F Linux homes — open (detect + Tails private default)".into(),
                "G Script — refuse-only".into(),
            ]),
            Block::Note(
                "H Other OS (Windows, macOS, Android) and I Depth (media, i18n, \
                 print, extension runtime) are parked."
                    .into(),
            ),
            Block::Link {
                label: "Linux homes".into(),
                href: "about:linux".into(),
            },
        ],
    })
}

fn linux_page(url: &Url) -> Document {
    let home = frihart_platform::detect_linux_home();
    Document::internal(InternalPage {
        title: "Linux".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Linux homes".into(),
                subtitle: format!("detected: {}", home.label()),
            },
            Block::KeyValue {
                key: "amnesic default".into(),
                value: if home.prefer_ephemeral() {
                    "yes — use --private or a ram profile".into()
                } else {
                    "no".into()
                },
            },
            Block::KeyValue {
                key: "Tor is the network".into(),
                value: if home.tor_is_the_network() {
                    "yes — use Tails SOCKS, never a second daemon".into()
                } else {
                    "optional system tor".into()
                },
            },
            Block::List(vec![
                "Arch / CachyOS — reference".into(),
                "Fedora — RPM family".into(),
                "Mint — Debian family".into(),
                "Tails — amnesic, system Tor only".into(),
                "Qubes — AppVM / DisposableVM, no NIC".into(),
            ]),
            Block::Note("docs/distros.md in the source tree.".into()),
        ],
    })
}

fn script_page(url: &Url, prefs: &Prefs) -> Document {
    Document::internal(InternalPage {
        title: "Script".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Script".into(),
                subtitle: "Campaign G. Execution is off.".into(),
            },
            Block::KeyValue {
                key: "pref javascript".into(),
                value: if prefs.privacy.javascript {
                    "on — still not executed".into()
                } else {
                    "off".into()
                },
            },
            Block::KeyValue {
                key: "untrusted eval".into(),
                value: if frihart_js::untrusted_eval_allowed() {
                    "allowed".into()
                } else {
                    "refused".into()
                },
            },
            Block::List(vec![
                "eval denied".into(),
                "wasm later".into(),
                "canvas / WebGL / audio / battery / plugins denied".into(),
            ]),
            Block::Note(
                "Flipping the pref does not start a JS engine. Fingerprint APIs \
                 stay denied after a runtime exists."
                    .into(),
            ),
        ],
    })
}

fn print_page(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Print".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Print".into(),
                subtitle: "Display list → local PostScript. No cloud print.".into(),
            },
            Block::Paragraph(
                "frihart-print writes a .ps job from the pipeline display list. \
                 PDF comes after the list is trusted."
                    .into(),
            ),
        ],
    })
}

fn downloads_page(url: &Url, profile: &Profile) -> Document {
    let dir = frihart_platform::downloads_dir();
    let mut blocks = vec![
        Block::Hero {
            title: "Downloads".into(),
            subtitle: "Written to your Downloads folder. Never executed.".into(),
        },
        Block::KeyValue {
            key: "directory".into(),
            value: dir.display().to_string(),
        },
    ];
    let log = if profile.is_ephemeral() {
        frihart_net::DownloadLog::default()
    } else {
        frihart_net::DownloadLog::load(&profile.root().join("downloads.json")).unwrap_or_default()
    };
    if log.items.is_empty() {
        blocks.push(Block::Paragraph("No downloads in this profile yet.".into()));
    } else {
        blocks.push(Block::Heading("This profile".into()));
        for rec in log.items.iter().take(32) {
            blocks.push(Block::KeyValue {
                key: rec.dest.clone(),
                value: format!("{} · {} bytes", rec.url, rec.bytes),
            });
        }
    }
    blocks.push(Block::Note(
        "Non-HTML responses (PDF, zip, images) are saved as 0600 files. Frihart \
         will not run them."
            .into(),
    ));
    Document::internal(InternalPage {
        title: "Downloads".into(),
        url: url.clone(),
        blocks,
    })
}

fn autofill_page(url: &Url, prefs: &Prefs, profile: &Profile) -> Document {
    let id =
        frihart_autofill::Identity::load(&profile.root().join("autofill.toml")).unwrap_or_default();
    Document::internal(InternalPage {
        title: "Autofill".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Autofill".into(),
                subtitle: "Identity only. Passwords never stored.".into(),
            },
            Block::KeyValue {
                key: "enabled".into(),
                value: if prefs.autofill.enabled {
                    "on".into()
                } else {
                    "off".into()
                },
            },
            Block::KeyValue {
                key: "name".into(),
                value: empty_dash(&id.name),
            },
            Block::KeyValue {
                key: "email".into(),
                value: empty_dash(&id.email),
            },
            Block::KeyValue {
                key: "organization".into(),
                value: empty_dash(&id.organization),
            },
            Block::KeyValue {
                key: "address".into(),
                value: empty_dash(&id.address),
            },
            Block::Note(
                "Edit autofill.toml in this profile (0600). Ctrl+Shift+A fills the page.".into(),
            ),
            Block::Link {
                label: "Password managers".into(),
                href: "about:pass".into(),
            },
        ],
    })
}

fn empty_dash(s: &str) -> String {
    if s.is_empty() {
        "(empty)".into()
    } else {
        s.into()
    }
}

fn passwords(url: &Url, prefs: &Prefs) -> Document {
    let found = frihart_platform::detect_pass_managers();
    let mut blocks = vec![
        Block::Hero {
            title: "Passwords".into(),
            subtitle:
                "Frihart does not store logins. Use Proton Pass or another manager you chose."
                    .into(),
        },
        Block::Paragraph(
            "No password vault in this browser. No collection. Launch a local \
             manager you already installed."
                .into(),
        ),
        Block::KeyValue {
            key: "preferred".into(),
            value: if prefs.pass.manager.is_empty() {
                "none".into()
            } else {
                prefs.pass.manager.clone()
            },
        },
        Block::Heading("Detected".into()),
    ];
    if found.is_empty() {
        blocks.push(Block::Paragraph(
            "None on PATH. Install Proton Pass, KeePassXC, Bitwarden, or pass.".into(),
        ));
    } else {
        for mgr in found {
            blocks.push(Block::Link {
                label: mgr.name.into(),
                href: format!("frihart:pass/{}", mgr.id),
            });
        }
    }
    blocks.push(Block::Link {
        label: "Launch preferred".into(),
        href: "frihart:pass".into(),
    });
    Document::internal(InternalPage {
        title: "Passwords".into(),
        url: url.clone(),
        blocks,
    })
}

fn settings(url: &Url, prefs: &Prefs) -> Document {
    Document::internal(InternalPage {
        title: "Settings".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Settings".into(),
                subtitle: "Defaults protect you. Every toggle is local.".into(),
            },
            Block::Heading("Look".into()),
            toggle(
                PrefToggle::DarkMode,
                "Dark mode",
                "Black pages, yellow accents. This is the product look, not a theme store.",
                prefs.content.dark_mode,
            ),
            Block::Heading("Protection".into()),
            toggle(
                PrefToggle::Blocker,
                "Native blocker (uBlock-class)",
                "On by default. Built into Frihart — not an add-on, not a store listing.",
                prefs.privacy.blocker,
            ),
            toggle(
                PrefToggle::Containers,
                "Identity containers",
                "Tabs belong to Personal, Work, Banking, or Shopping. Cookies never cross.",
                prefs.privacy.containers,
            ),
            toggle(
                PrefToggle::Autofill,
                "Identity autofill",
                "Fills name/email/address you saved. Never passwords.",
                prefs.autofill.enabled,
            ),
            toggle(
                PrefToggle::Translate,
                "Built-in translator",
                "DeepL by default. Add your API key in prefs.toml. No Google.",
                prefs.translate.enabled,
            ),
            toggle(
                PrefToggle::HttpsOnly,
                "HTTPS-only mode",
                "Refuse cleartext HTTP. Exceptions will be per-site, when the network stack exists.",
                prefs.privacy.https_only,
            ),
            toggle(
                PrefToggle::ResistFingerprinting,
                "Resist fingerprinting",
                "Clamp or deny high-entropy APIs. Frihart will not impersonate Chrome.",
                prefs.privacy.resist_fingerprinting,
            ),
            toggle(
                PrefToggle::ThirdPartyCookies,
                "Allow third-party cookies",
                "Off. Cross-site cookies are tracking. Turn this on only if you mean it.",
                prefs.privacy.third_party_cookies,
            ),
            toggle(
                PrefToggle::SendGpc,
                "Send Global Privacy Control",
                "A single bit some jurisdictions treat as a legal signal. Do Not Track is not sent.",
                prefs.privacy.send_gpc,
            ),
            Block::Heading("Local data".into()),
            toggle(
                PrefToggle::PersistHistory,
                "Remember history",
                "Stored only in your profile. Never uploaded. Private windows ignore this.",
                prefs.privacy.persist_history,
            ),
            toggle(
                PrefToggle::PersistCookies,
                "Remember cookies",
                "First-party only, when the network stack exists.",
                prefs.privacy.persist_cookies,
            ),
            toggle(
                PrefToggle::RestoreSession,
                "Restore previous session",
                "Off by default. The browser opens what you open.",
                prefs.general.restore_session,
            ),
            Block::Heading("Attack surface".into()),
            toggle(
                PrefToggle::Javascript,
                "JavaScript",
                "There is no engine yet. This pref exists so the permission model is real from day one.",
                prefs.privacy.javascript,
            ),
            toggle(
                PrefToggle::WebRtc,
                "WebRTC",
                "Off. IP-leak surface. Stays off until implemented and reviewed.",
                prefs.privacy.webrtc,
            ),
            Block::Note(
                "More knobs live on about:config. There is no search-engine picker \
                 because there is no default search deal."
                    .into(),
            ),
            Block::Link {
                label: "All preferences".into(),
                href: "about:config".into(),
            },
        ],
    })
}

fn privacy(url: &Url, prefs: &Prefs) -> Document {
    let policy = Policy::from_prefs(prefs);
    let mut blocks = vec![
        Block::Hero {
            title: "Privacy".into(),
            subtitle: "The constitution, as currently configured.".into(),
        },
        Block::Paragraph(
            "No telemetry. No accounts. No password or login store. Startup \
             makes zero network connections."
                .into(),
        ),
        Block::KeyValue {
            key: "store_logins".into(),
            value: "false".into(),
        },
        Block::Link {
            label: "Wipe / shred".into(),
            href: "about:shred".into(),
        },
        Block::Heading("Active policy".into()),
    ];
    for (key, value, _good) in policy.summary_lines() {
        blocks.push(Block::KeyValue {
            key: key.into(),
            value,
        });
    }
    blocks.extend([
        Block::Heading("Identity we will send (Phase 2)".into()),
        Block::KeyValue {
            key: "User-Agent".into(),
            value: policy.user_agent().into(),
        },
        Block::KeyValue {
            key: "Language".into(),
            value: prefs.privacy.language.clone(),
        },
        Block::KeyValue {
            key: "Timezone".into(),
            value: format!("{:?}", prefs.privacy.timezone).to_ascii_lowercase(),
        },
        Block::KeyValue {
            key: "DoH".into(),
            value: if prefs.network.doh_url.is_empty() {
                format!("{} (no resolver configured)", prefs.network.doh_mode)
            } else {
                prefs.network.doh_mode.clone()
            },
        },
        Block::Note(
            "DNT is off on purpose. It is a tracking bit. GPC is on. Client \
             Hints are never sent."
                .into(),
        ),
        Block::Link {
            label: "Change settings".into(),
            href: "about:settings".into(),
        },
    ]);
    Document::internal(InternalPage {
        title: "Privacy".into(),
        url: url.clone(),
        blocks,
    })
}

fn config(url: &Url, prefs: &Prefs) -> Document {
    let socks_port = prefs.tor.socks_port.to_string();
    let mut blocks = vec![
        Block::Hero {
            title: "about:config".into(),
            subtitle: "Typed preferences. Not a string soup.".into(),
        },
        Block::Note(
            "Toggles that are safe to flip from chrome live on about:settings. \
             This page is the full snapshot."
                .into(),
        ),
        Block::Heading("general".into()),
        kv("homepage", &prefs.general.homepage),
        kv("new_tab_url", &prefs.general.new_tab_url),
        kv(
            "search_url",
            if prefs.general.search_url.is_empty() {
                "(empty — use search.primary)"
            } else {
                &prefs.general.search_url
            },
        ),
        kv("restore_session", bool_str(prefs.general.restore_session)),
        kv("show_status_bar", bool_str(prefs.general.show_status_bar)),
        kv("welcome_seen", bool_str(prefs.general.welcome_seen)),
        Block::Heading("privacy".into()),
        kv("https_only", bool_str(prefs.privacy.https_only)),
        kv(
            "resist_fingerprinting",
            bool_str(prefs.privacy.resist_fingerprinting),
        ),
        kv(
            "third_party_cookies",
            bool_str(prefs.privacy.third_party_cookies),
        ),
        kv(
            "partition_first_party_state",
            bool_str(prefs.privacy.partition_first_party_state),
        ),
        kv(
            "send_referrer",
            &format!("{:?}", prefs.privacy.send_referrer),
        ),
        kv("send_dnt", bool_str(prefs.privacy.send_dnt)),
        kv("send_gpc", bool_str(prefs.privacy.send_gpc)),
        kv("persist_history", bool_str(prefs.privacy.persist_history)),
        kv("persist_cookies", bool_str(prefs.privacy.persist_cookies)),
        kv("webrtc", bool_str(prefs.privacy.webrtc)),
        kv("javascript", bool_str(prefs.privacy.javascript)),
        kv("timezone", &format!("{:?}", prefs.privacy.timezone)),
        kv("language", &prefs.privacy.language),
        kv("containers", bool_str(prefs.privacy.containers)),
        kv("blocker", bool_str(prefs.privacy.blocker)),
        kv("store_logins", bool_str(prefs.privacy.store_logins)),
        Block::Heading("network".into()),
        kv("user_agent", &prefs.network.user_agent),
        kv("client_hints", bool_str(prefs.network.client_hints)),
        kv("doh_mode", &prefs.network.doh_mode),
        kv(
            "doh_url",
            if prefs.network.doh_url.is_empty() {
                "(empty)"
            } else {
                &prefs.network.doh_url
            },
        ),
        kv("http2", bool_str(prefs.network.http2)),
        kv("http3", bool_str(prefs.network.http3)),
        Block::Heading("content".into()),
        kv("images", bool_str(prefs.content.images)),
        kv("media", bool_str(prefs.content.media)),
        kv("webgl", bool_str(prefs.content.webgl)),
        kv("canvas", bool_str(prefs.content.canvas)),
        kv("dark_mode", bool_str(prefs.content.dark_mode)),
        Block::Heading("translate".into()),
        kv("enabled", bool_str(prefs.translate.enabled)),
        kv("provider", &prefs.translate.provider),
        kv("endpoint", &prefs.translate.endpoint),
        kv(
            "api_key",
            if prefs.translate.api_key.is_empty() {
                "(empty — paste your DeepL key in prefs.toml)"
            } else {
                "(set, not shown)"
            },
        ),
        kv("source", &prefs.translate.source),
        kv("target", &prefs.translate.target),
        Block::Heading("search".into()),
        kv("primary", &prefs.search.primary),
        kv("secondary", &prefs.search.secondary),
        Block::Heading("tor".into()),
        kv("enabled", bool_str(prefs.tor.enabled)),
        kv("socks_host", &prefs.tor.socks_host),
        kv("socks_port", &socks_port),
        Block::Heading("vpn".into()),
        kv("provider", &prefs.vpn.provider),
        Block::Heading("extensions".into()),
        kv("enabled", bool_str(prefs.extensions.enabled)),
        Block::Heading("pass".into()),
        kv(
            "manager",
            if prefs.pass.manager.is_empty() {
                "none"
            } else {
                &prefs.pass.manager
            },
        ),
        Block::Heading("support".into()),
        kv(
            "xmr",
            if prefs.support.xmr.is_empty() {
                "(empty)"
            } else {
                "(set)"
            },
        ),
        kv(
            "btc",
            if prefs.support.btc.is_empty() {
                "(empty)"
            } else {
                "(set)"
            },
        ),
        kv(
            "fiat_url",
            if prefs.support.fiat_url.is_empty() {
                "(empty)"
            } else {
                "(set)"
            },
        ),
    ];
    let _ = FROZEN_USER_AGENT;
    blocks.push(Block::Link {
        label: "Settings".into(),
        href: "about:settings".into(),
    });
    Document::internal(InternalPage {
        title: "Config".into(),
        url: url.clone(),
        blocks,
    })
}

fn license(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "License".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "License".into(),
                subtitle: "MIT OR Apache-2.0".into(),
            },
            Block::Paragraph(
                "Frihart is dual-licensed under the MIT License and the Apache \
                 License, Version 2.0. You may choose either. The point is that \
                 you own the binary you run."
                    .into(),
            ),
            Block::Paragraph(
                "Copyright 2026 Frihart contributors. The full texts live in \
                 LICENSE-MIT and LICENSE-APACHE in the source tree."
                    .into(),
            ),
            Block::Link {
                label: "Credits".into(),
                href: "about:credits".into(),
            },
        ],
    })
}

fn credits(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Credits".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Credits".into(),
                subtitle: "Original product. Borrowed primitives.".into(),
            },
            Block::Paragraph(
                "Frihart is not a fork. The browser, chrome, policy engine, and \
                 (over time) document engine are written here."
                    .into(),
            ),
            Block::Heading("Inspiration".into()),
            Block::Paragraph(
                "LibreWolf is the inspiration for Frihart's stance: telemetry \
                 stripped, fingerprinting resisted, no sponsored defaults, the \
                 user is sovereign. LibreWolf is a Firefox fork. Frihart is not. \
                 We take the ethic as original Rust — containers and a \
                 uBlock-class blocker are native, not add-ons. Look: black and yellow."
                    .into(),
            ),
            Block::Heading("Primitives we did not reimplement".into()),
            Block::List(vec![
                "Rust, Cargo, and the language ecosystem".into(),
                "winit — windowing".into(),
                "softbuffer — presenting a software framebuffer".into(),
                "cosmic-text — text shaping and raster".into(),
                "the url crate — WHATWG URL parsing".into(),
                "serde / toml — preferences on disk".into(),
                "rustls (Phase 2) — TLS".into(),
            ]),
            Block::Note(
                "Using those libraries is not the same as shipping Gecko, Blink, \
                 WebKit, or Servo. See ARCHITECTURE.md."
                    .into(),
            ),
        ],
    })
}

fn keyboard(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Keyboard".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Keyboard".into(),
                subtitle: "The chrome is meant to be used without a mouse.".into(),
            },
            Block::KeyValue {
                key: "Ctrl+T".into(),
                value: "New tab".into(),
            },
            Block::KeyValue {
                key: "Ctrl+W".into(),
                value: "Close tab".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Tab / Ctrl+Shift+Tab".into(),
                value: "Cycle tabs".into(),
            },
            Block::KeyValue {
                key: "Ctrl+1 … Ctrl+9".into(),
                value: "Jump to tab".into(),
            },
            Block::KeyValue {
                key: "Ctrl+L".into(),
                value: "Focus URL bar".into(),
            },
            Block::KeyValue {
                key: "Enter".into(),
                value: "Go".into(),
            },
            Block::KeyValue {
                key: "Escape".into(),
                value: "Blur URL bar".into(),
            },
            Block::KeyValue {
                key: "Alt+Left / Alt+Right".into(),
                value: "Back / forward".into(),
            },
            Block::KeyValue {
                key: "Ctrl+R / F5".into(),
                value: "Reload".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Q".into(),
                value: "Quit".into(),
            },
            Block::KeyValue {
                key: "Ctrl+F".into(),
                value: "Find in page".into(),
            },
            Block::KeyValue {
                key: "Ctrl+D".into(),
                value: "Bookmark this page".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Shift+C".into(),
                value: "Cycle container".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Shift+P".into(),
                value: "Hint: start with --private".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Shift+O".into(),
                value: "New Tor tab".into(),
            },
            Block::KeyValue {
                key: "Ctrl+K".into(),
                value: "Search (Swisscows)".into(),
            },
            Block::KeyValue {
                key: "Ctrl+Shift+A".into(),
                value: "Autofill identity (never passwords)".into(),
            },
        ],
    })
}

fn roadmap(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: "Roadmap".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Roadmap".into(),
                subtitle: "A capability ladder, not a fake launch date.".into(),
            },
            Block::List(vec![
                "A Foundation — closed (v0.1.0)".into(),
                "B Chrome — closed".into(),
                "C Network OPSEC — closed".into(),
                "D Engine — open".into(),
                "E Isolation — open".into(),
                "F Linux homes — open".into(),
                "G Script — refuse-only".into(),
                "H Other OS — parked".into(),
                "I Depth — parked".into(),
            ]),
            Block::Paragraph(
                "The full plan, success criteria, and time ranges live in ROADMAP.md \
                 in the source tree. We do not skip isolation to paint more CSS."
                    .into(),
            ),
        ],
    })
}

fn about(url: &Url) -> Document {
    Document::internal(InternalPage {
        title: APP_NAME.into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: format!("{APP_NAME} {VERSION}"),
                subtitle: "Libertarian. Private by default. Linux first.".into(),
            },
            Block::Paragraph(
                "Frihart is not Firefox, not LibreWolf, and not Chromium. LibreWolf \
                 inspired the stance. The code is original Rust so we do not inherit \
                 Gecko's memory-unsafe weekly RCEs. Long project. Honest scope."
                    .into(),
            ),
            Block::KeyValue {
                key: "Version".into(),
                value: VERSION.into(),
            },
            Block::KeyValue {
                key: "License".into(),
                value: "MIT OR Apache-2.0".into(),
            },
            Block::KeyValue {
                key: "User-Agent (frozen)".into(),
                value: FROZEN_USER_AGENT.into(),
            },
            Block::Link {
                label: "Philosophy lives in the source tree".into(),
                href: "about:privacy".into(),
            },
        ],
    })
}

fn bookmarks(url: &Url, profile: &Profile) -> Document {
    let mut blocks = vec![Block::Hero {
        title: "Bookmarks".into(),
        subtitle: if profile.is_ephemeral() {
            "This is a private window. These bookmarks are not written to disk.".into()
        } else {
            "Stored in your profile as bookmarks.toml.".into()
        },
    }];
    if profile.bookmarks().items.is_empty() {
        blocks.push(Block::Paragraph("No bookmarks yet.".into()));
    } else {
        for mark in &profile.bookmarks().items {
            blocks.push(Block::Link {
                label: format!("{}  ·  {}", mark.title, mark.url),
                href: mark.url.clone(),
            });
            blocks.push(Block::Link {
                label: format!("remove {}", mark.title),
                href: format!("frihart:unbookmark/{}", mark.url),
            });
        }
    }
    Document::internal(InternalPage {
        title: "Bookmarks".into(),
        url: url.clone(),
        blocks,
    })
}

fn containers(url: &Url, profile: &Profile) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Containers".into(),
            subtitle: "First-class identity isolation. Inspired by Firefox Multi-Account Containers, built into Frihart — not an add-on.".into(),
        },
        Block::Paragraph(
            "Each tab belongs to a container. When the network stack exists, cookies, \
             cache, and storage are partitioned by container. A banking tab cannot \
             see a shopping tab."
                .into(),
        ),
        Block::Note(
            "Click a container to assign it to the current tab. Ctrl+Shift+C cycles. \
             New tabs inherit the active container."
                .into(),
        ),
        Block::Heading("Your containers".into()),
    ];
    for item in &profile.containers().items {
        blocks.push(Block::Link {
            label: format!("{}  ·  {}", item.name, item.slug),
            href: format!("frihart:container/{}", item.slug),
        });
        blocks.push(Block::KeyValue {
            key: "color".into(),
            value: format!("#{:06x}", item.color & 0x00ff_ffff),
        });
    }
    if !profile.prefs().privacy.containers {
        blocks.push(Block::Note(
            "Containers are disabled in settings. Isolation will not be applied.".into(),
        ));
    }
    Document::internal(InternalPage {
        title: "Containers".into(),
        url: url.clone(),
        blocks,
    })
}

fn blocker(url: &Url, prefs: &Prefs) -> Document {
    let engine = frihart_blocker::FilterEngine::new(prefs.privacy.blocker);
    let mut sample: Vec<String> = engine
        .sample(16)
        .into_iter()
        .map(|h| format!("||{h}^"))
        .collect();
    if sample.is_empty() {
        sample.push("(no rules)".into());
    }
    Document::internal(InternalPage {
        title: "Blocker".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Native blocker".into(),
                subtitle: "uBlock-class protection, shipped in the browser. Not an extension."
                    .into(),
            },
            Block::Paragraph(
                "LibreWolf ships uBlock Origin as a default add-on because it is a \
                 Firefox fork. Frihart is not Firefox, so we do not embed that \
                 extension. We build the same job into the engine: network blocking \
                 on by default, lists the user can replace, no store, no phone-home."
                    .into(),
            ),
            Block::KeyValue {
                key: "Enabled".into(),
                value: if engine.enabled() {
                    "on".into()
                } else {
                    "off".into()
                },
            },
            Block::KeyValue {
                key: "Built-in host rules".into(),
                value: engine.rule_count().to_string(),
            },
            Block::KeyValue {
                key: "List format (Phase 2)".into(),
                value: "EasyList, EasyPrivacy, uBlock filters — local files only".into(),
            },
            Block::Heading("Seed (sample)".into()),
            Block::List(sample),
            Block::Note(
                "Cosmetic filtering waits for the HTML engine. List updates are \
                 never pulled from Frihart servers. Distros or you ship new lists."
                    .into(),
            ),
            Block::Link {
                label: "Settings".into(),
                href: "about:settings".into(),
            },
        ],
    })
}

fn translate(url: &Url, prefs: &Prefs) -> Document {
    let key_state = if prefs.translate.api_key.is_empty() {
        "missing — add translate.api_key in prefs.toml"
    } else {
        "present (not displayed)"
    };
    Document::internal(InternalPage {
        title: "Translator".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Translator".into(),
                subtitle: "DeepL is the product default. No Google.".into(),
            },
            Block::Paragraph(
                "Page translation needs the document engine (Phase 4+). Phase 2 \
                 will POST to DeepL when you ask, and only then. The key stays \
                 in your profile."
                    .into(),
            ),
            Block::KeyValue {
                key: "Provider".into(),
                value: prefs.translate.provider.clone(),
            },
            Block::KeyValue {
                key: "Endpoint".into(),
                value: prefs.translate.endpoint.clone(),
            },
            Block::KeyValue {
                key: "API key".into(),
                value: key_state.into(),
            },
            Block::KeyValue {
                key: "Source / target".into(),
                value: format!("{} → {}", prefs.translate.source, prefs.translate.target),
            },
            Block::Note(
                "LibreTranslate remains available: set translate.provider = \
                 \"libretranslate\" and point endpoint at an instance you host. \
                 Google is not offered."
                    .into(),
            ),
            Block::Link {
                label: "about:config".into(),
                href: "about:config".into(),
            },
        ],
    })
}

fn search(url: &Url, prefs: &Prefs) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Search".into(),
            subtitle: "Swisscows first. DuckDuckGo second. No Google. No Bing.".into(),
        },
        Block::Paragraph(
            "Typing words in the URL bar that are not a destination becomes a \
             search against the primary engine (Swisscows)."
                .into(),
        ),
        Block::KeyValue {
            key: "Primary".into(),
            value: prefs.search.primary.clone(),
        },
        Block::KeyValue {
            key: "Secondary".into(),
            value: prefs.search.secondary.clone(),
        },
        Block::Heading("Engines we ship".into()),
    ];
    for engine in frihart_search::catalog() {
        let role = if engine.id == prefs.search.primary {
            "primary"
        } else if engine.id == prefs.search.secondary {
            "secondary"
        } else {
            "available"
        };
        blocks.push(Block::KeyValue {
            key: format!("{} ({role})", engine.name),
            value: engine.region.into(),
        });
    }
    blocks.push(Block::Note(
        "Change search.primary / search.secondary in prefs.toml. SearXNG is \
         welcome later as a user-supplied template. We do not take search money."
            .into(),
    ));
    Document::internal(InternalPage {
        title: "Search".into(),
        url: url.clone(),
        blocks,
    })
}

fn tor(url: &Url, prefs: &Prefs) -> Document {
    let presence = frihart_platform::detect_tor();
    let binary = presence
        .binary
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found on PATH (install the system tor package)".into());
    Document::internal(InternalPage {
        title: "Tor".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "Tor tab".into(),
                subtitle: "Anonymous circuit. Uses your system Tor. We do not ship the network."
                    .into(),
            },
            Block::Paragraph(
                "A Tor tab is stronger than a private tab. Private is local amnesia. \
                 Tor is amnesia plus a SOCKS circuit through the Tor daemon you \
                 installed (Arch: tor, Debian/Fedora: tor). Ctrl+Shift+O or --tor."
                    .into(),
            ),
            Block::KeyValue {
                key: "SOCKS".into(),
                value: format!("{}:{}", prefs.tor.socks_host, prefs.tor.socks_port),
            },
            Block::KeyValue {
                key: "tor binary".into(),
                value: binary,
            },
            Block::KeyValue {
                key: "Enabled".into(),
                value: if prefs.tor.enabled {
                    "on".into()
                } else {
                    "off".into()
                },
            },
            Block::Note(
                "Tor tabs dial only this SOCKS port. If the daemon is down, the \
                 tab fails. There is no clearnet fallback."
                    .into(),
            ),
        ],
    })
}

fn vpn(url: &Url, prefs: &Prefs) -> Document {
    let presence = frihart_platform::detect_vpn();
    let proton = presence
        .proton
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "ProtonVPN CLI not installed".into());
    let mullvad = presence
        .mullvad
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Mullvad CLI not installed".into());
    Document::internal(InternalPage {
        title: "VPN".into(),
        url: url.clone(),
        blocks: vec![
            Block::Hero {
                title: "VPN".into(),
                subtitle: "ProtonVPN and Mullvad first. Official clients, not a reimplementation."
                    .into(),
            },
            Block::Paragraph(
                "Frihart will not ship a VPN protocol. That is how you get leaks. \
                 We detect the official CLI, show status, and (Phase 2+) can \
                 connect/disconnect through it. Other providers can be added later \
                 the same way."
                    .into(),
            ),
            Block::KeyValue {
                key: "Preferred".into(),
                value: prefs.vpn.provider.clone(),
            },
            Block::KeyValue {
                key: "ProtonVPN".into(),
                value: proton,
            },
            Block::KeyValue {
                key: "Mullvad".into(),
                value: mullvad,
            },
            Block::Note(
                "Install `mullvad` or `protonvpn-cli` from your distro. Set \
                 vpn.provider to proton or mullvad in prefs.toml."
                    .into(),
            ),
        ],
    })
}

fn extensions(url: &Url, profile: &Profile) -> Document {
    let mut blocks = vec![
        Block::Hero {
            title: "Add-ons".into(),
            subtitle: "Firefox-compatible host, original engine. Not a Gecko fork.".into(),
        },
        Block::Paragraph(
            "Yes: Frihart can become compatible with Firefox extensions without \
             being a fork. An .xpi is a ZIP plus manifest.json plus JavaScript \
             that calls browser.*. We parse the package today. We will implement \
             those APIs on our own engine. We will not vendor Gecko."
                .into(),
        ),
        Block::Paragraph(
            "Installed add-ons are dormant until Phase 7 (JS) and Phase 3–5 \
             (DOM for popups and content scripts). Sideload only: \
             frihart --install-addon ./something.xpi"
                .into(),
        ),
        Block::Heading("Installed".into()),
    ];
    if profile.addons().items.is_empty() {
        blocks.push(Block::Paragraph(
            "None yet. Drop a Firefox .xpi or an unpacked folder onto \
             --install-addon. uBlock Origin will install and stay dormant; \
             Frihart already ships a native blocker for that job."
                .into(),
        ));
    } else {
        for addon in &profile.addons().items {
            blocks.push(Block::KeyValue {
                key: format!("{} {}", addon.name, addon.version),
                value: format!("{} · {}", addon.id, addon.run_state),
            });
            blocks.push(Block::Note(addon.dormant_reason().into()));
            for (perm, support) in addon.permission_report() {
                blocks.push(Block::KeyValue {
                    key: perm,
                    value: support.label().into(),
                });
            }
        }
    }
    blocks.extend([
        Block::Heading("Compatibility ladder".into()),
        Block::List(vec![
            "Now — parse XPI / unpacked, install into the profile, audit permissions".into(),
            "Phase 2 — webRequest maps onto frihart-net + the native blocker".into(),
            "Phase 3–5 — options pages and popups need HTML".into(),
            "Phase 6 — add-ons run out of process, cannot read prefs.toml".into(),
            "Phase 7 — JS runtime executes background and content scripts".into(),
        ]),
        Block::Note(
            "Full AMO compatibility is years of API work. We claim packages we \
             can parse and APIs we implement, never “any Firefox add-on works.”"
                .into(),
        ),
        Block::Link {
            label: "Architecture notes".into(),
            href: "about:credits".into(),
        },
    ]);
    Document::internal(InternalPage {
        title: "Add-ons".into(),
        url: url.clone(),
        blocks,
    })
}

fn history(url: &Url, profile: &Profile) -> Document {
    let mut blocks = vec![Block::Hero {
        title: "History".into(),
        subtitle: if profile.prefs().privacy.persist_history && !profile.is_ephemeral() {
            "Local only. Never uploaded.".into()
        } else {
            "History is not being recorded.".into()
        },
    }];
    let recent = profile.history().recent(50);
    blocks.push(Block::Link {
        label: "wipe".into(),
        href: "frihart:wipe-history".into(),
    });
    if recent.is_empty() {
        blocks.push(Block::Paragraph("empty".into()));
    } else {
        for entry in recent {
            blocks.push(Block::Link {
                label: format!("{}  ·  {}", entry.title, entry.url),
                href: entry.url.clone(),
            });
        }
    }
    Document::internal(InternalPage {
        title: "History".into(),
        url: url.clone(),
        blocks,
    })
}

fn toggle(id: PrefToggle, label: &str, description: &str, value: bool) -> Block {
    Block::Toggle {
        id,
        label: label.into(),
        description: description.into(),
        value,
    }
}

fn kv(key: &str, value: &str) -> Block {
    Block::KeyValue {
        key: key.into(),
        value: value.into(),
    }
}

fn bool_str<'a>(v: bool) -> &'a str {
    if v { "true" } else { "false" }
}
