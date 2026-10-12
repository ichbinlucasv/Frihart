# Frihart design system

Black chrome, yellow accents, quiet content. The network a tab uses is always visible.
Concept images: [`docs/screenshots/concept/`](screenshots/concept/) (HTML/CSS sources in `src/`).
Palette tokens match `crates/frihart-chrome/src/theme.rs`.

## Palette
| Token | Hex | Use |
|---|---|---|
| BG_CHROME | `#000000` | tab strip, status bar |
| BG_TAB_ACTIVE | `#161616` | active tab, toolbar |
| BG_CONTENT | `#0A0A0A` | page area |
| HAIRLINE | `#2A2A2A` | borders |
| ACCENT | `#F5C400` (dim `#B38F00`) | brand, toggles on, links |
| TEXT_CONTENT | `#F2F2F2` · muted `#A09040` | text |
| BAD | `#C04040` | forced-off, errors, circuit down |
| tor (new) | `#C4A8FF` | Tor tab pill |
| i2p (new) | `#7EE0C6` | I2P tab pill |

## Typography
Inter / DejaVu Sans for chrome (13–14 px), JetBrains / DejaVu Sans Mono for URLs, pref keys and the status bar.

## Spacing and shape
4 px grid. Tabs: 10 px top radius. URL field and cards: 10–14 px radius. Pills fully round.

## Components
- **Tab network pill**: `DIRECT` / `TOR` / `I2P` on every tab and in the URL field.
- **Fail-closed card**: on Tor/I2P tabs, "If Tor drops, this tab stops. No clearnet fallback. DNS inside Tor."
- **Shields panel** (🛡 + blocked count): blocker, uniform identity headers, letterboxing, JS (off, no engine), third-party cookies, tracking-param strip.
- **Letterbox frame**: dashed outline with the bucket size the page sees.
- **Settings rows**: label + pref key in mono + toggle; a red "Forced off" card for telemetry, Safe Browsing, DRM, stored logins.
- **Status bar**: JS off · HTTPS-only · letterbox size · blocker · "no telemetry · no account".

## Rules
Only show features that exist (see `docs/defaults.md`). No sponsored tiles, no account UI.
