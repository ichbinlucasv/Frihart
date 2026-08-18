# Frihart defaults

Every shipped default lives here. If you change a default, change this
file in the same commit and say why.

The implementing type is `frihart_config::Prefs`.

## General

| Pref | Default | Why |
| --- | --- | --- |
| `general.homepage` | `about:home` | No third-party start page. No sponsored tiles. |
| `general.restore_session` | `false` | The user opens what they open. Session restore is opt-in. |
| `general.new_tab_url` | `about:newtab` | Same reason as homepage. |
| `general.search_url` | empty | Optional override. Empty means use `search.primary`. |
| `general.show_status_bar` | `true` | Destinations should be visible before click. |
| `general.welcome_seen` | `false` | First run opens about:welcome once. |

## Privacy

| Pref | Default | Why |
| --- | --- | --- |
| `privacy.https_only` | `true` | Cleartext HTTP is the exception, not the web. |
| `privacy.resist_fingerprinting` | `true` | Identity is not a site entitlement. |
| `privacy.third_party_cookies` | `false` | Cross-site cookies are tracking. |
| `privacy.partition_first_party_state` | `true` | Embedded first-party state is still a tracking vector. |
| `privacy.send_referrer` | `origin-only` | Path-bearing referrers leak URLs. Cross-site referrers are dropped. |
| `privacy.send_dnt` | `false` | DNT is a fingerprint with no legal force. |
| `privacy.send_gpc` | `true` | GPC is meaningful in some jurisdictions and is a single bit. |
| `privacy.persist_history` | `true` | Local history is the user's. It never leaves the disk. Easy to disable or wipe. |
| `privacy.persist_cookies` | `true` | Same. First-party only. |
| `privacy.webrtc` | `false` | IP-leak surface. Stays off until implemented and reviewed. |
| `privacy.javascript` | `false` | No engine yet. Remains a permission after an engine exists. |
| `privacy.timezone` | `utc` | System timezone is identifying. |
| `privacy.language` | `en` | A single language; not the full system list. |
| `privacy.containers` | `true` | Identity isolation is a default, not an add-on. |
| `privacy.blocker` | `true` | uBlock-class host blocking is on at install. |
| `privacy.store_logins` | `false` | Forced. Frihart does not collect passwords or logins. |

## Translate

| Pref | Default | Why |
| --- | --- | --- |
| `translate.enabled` | `true` | The UI exists. Network calls wait for a key and a user action. |
| `translate.provider` | `deepl` | Product default. LibreTranslate remains an option. |
| `translate.endpoint` | DeepL free API | Official DeepL. No Google. |
| `translate.api_key` | empty | You paste your key. It never leaves the profile except to DeepL. |
| `translate.source` | `auto` | Detect when a backend exists. |
| `translate.target` | `en` | User-changeable. |

## Search

| Pref | Default | Why |
| --- | --- | --- |
| `search.primary` | `swisscows` | Swiss, no-tracking search. Chosen as the product default. |
| `search.secondary` | `duckduckgo` | Fallback. Also privacy-focused. |

Also shipped, selectable: Startpage, Mojeek, Qwant, MetaGer, Brave Search.

## Tor

| Pref | Default | Why |
| --- | --- | --- |
| `tor.enabled` | `true` | Tor tabs are available. They need a system `tor` daemon. |
| `tor.socks_host` | `127.0.0.1` | Local daemon only. |
| `tor.socks_port` | `9050` | Standard Tor SOCKS. |

## VPN

| Pref | Default | Why |
| --- | --- | --- |
| `vpn.provider` | `none` | We do not connect a VPN until you pick Proton or Mullvad. |

## Extensions

| Pref | Default | Why |
| --- | --- | --- |
| `extensions.enabled` | `true` | Sideload and list Firefox-style add-ons. They do not execute yet. |

## Network

| Pref | Default | Why |
| --- | --- | --- |
| `network.user_agent` | frozen Frihart UA | No OS micro-version, no engine version salad. |
| `network.client_hints` | `false` | Client hints exist to fingerprint. |
| `network.doh_mode` | `off` | System DNS first. DoH is opt-in and user-chosen. |
| `network.doh_url` | empty | We do not pick a national or corporate resolver for you. |
| `network.http2` | `true` (when implemented) | Performance without a privacy cost by itself. |
| `network.http3` | `false` until reviewed | QUIC exposes a different fingerprint; later. |

## Content

| Pref | Default | Why |
| --- | --- | --- |
| `content.images` | `true` | Needed for a usable web; still first-party by policy. |
| `content.media` | `false` until implemented | No silent decoder attack surface. |
| `content.webgl` | `false` | GPU fingerprinting. |
| `content.canvas` | `false` until implemented | Classic fingerprint surface. |
| `content.dark_mode` | `true` | Black and yellow is the product, not a theme pack. |

## What is intentionally absent

There is no pref for telemetry, crash-report upload, usage ping, or
"help improve Frihart." Adding one is a philosophy violation, not a
product question.
