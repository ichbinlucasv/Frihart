# Handoff — continue Frihart

Tag: **v0.1.0**. Head: `main` on Codeberg (primary) and GitHub (mirror).

Do **not** start H (Windows/macOS/Android) or I (media decode, i18n
depth, print/PDF, extension runtime).

**Engines (Lucas, 2026-10-21).** Servo is the clearnet Compat engine and
Boa is the light JS bridge on the own engine, so the browser can be
finished before Campaign J. The own Rust engine stays in this same
project and keeps Tor, I2P and reader tabs. The day it covers the Compat
job, set `content.own_replaces_compat` and Servo is no longer selected.
Boa leaves when Campaign J covers the same scripts. Tor and I2P never
use Servo. The default build still refuses untrusted script.

## Who / what

- **Frihart** — original Rust privacy browser. Not a Firefox/LibreWolf/
  Chromium fork. LibreWolf is the **stance**. Look: **black + yellow**.
- **Libertarian:** user owns the machine and the binary (`MIT OR
  Apache-2.0`). No telemetry, no account, no password vault.
- **Linux first:** Arch/CachyOS reference, then Fedora, Mint, Tails,
  Qubes. Same binary. Linux free; other OS via Supporter plan later, no license server.
- **Three engines, one chrome.** Own engine for Tor, I2P and reader.
  Servo (feature `servo`, brings SpiderMonkey) for clearnet Compat.
  Boa for light JS on the own engine (feature not linked yet). Campaign
  J replaces Boa, then Servo. Do not embed V8, JavaScriptCore, QuickJS,
  Gecko, Blink, WebKit or CEF.

Repos:

- Primary: `ssh://git@codeberg.org/ichbinlucasv/Frihart.git`
- Mirror: `git@github.com:ichbinlucasv/Frihart.git`
- Author: Lucas `<codeberg.ecx3s@passmail.com>`

## Closed (do not reopen unless a bug)

| ID | What “closed” means |
| --- | --- |
| A / Phase 0 | Constitution, workspace, crates, about: chrome |
| B / Phase 1 | Black/yellow shell, containers, blocker, wipe/shred, profiles, bookmarks |
| C / Phase 2 | rustls, partitioned cookies, HTTPS-only, Tor SOCKS fail-closed, downloads 0600 never exec |

Leftovers that did **not** block close: multi-window, HTTP/2, partitioned HTTP cache.

## Open

**D Engine (long pole)**  
HTML → CSS → style → layout → display list → chrome paint. Tables are
a column grid. `hr`, caption, definition lists (HTML5 `div` wrappers;
a `dt`/`dd` that is one link is that hit). `dfn`/`acronym`/`cite` stay
visible. CSS: `em`/`rem`/`vw`/
`vh`/`%`, `font-weight`, `font-family` (engine slots), `list-style`,
`white-space`, `border`, `:link`. Nested `<strong>` is its
own fragment. Forms GET/POST (secrets skipped). JS off. Img is a box.
UTF-8 text (not Latin-1 bytes). Heading with one destination (link-only
or link + extra title) is one hit. `svg`/`path`/`canvas` skipped.
`about:sites` claimed: **example.com**, **RFC 1918**, **suckless.org**,
**GNU philosophy**, **kernel.org**, **docs.kernel.org**, **ietf.org**,
**rfc-editor.org** (index), **w3.org** (landing), **w3.org/TR** (index),
**webarch**, **RFC 9110**, **WCAG 2.2**, **RFC 8446**, **RFC 5280**,
**RFC 8032** (Ed25519), **RFC 7748** (Curve25519 / X25519),
**RFC 5869** (HKDF), **openbsd.org**, **openssh.com**
(→ openssh.org), **libressl.org**, **openbsdfoundation.org**, **openbgpd.org**, **opensmtpd.org**, **openntpd.org**, **openiked.org**, **bearssl.org**, **wireguard.com**, **gnupg.org**, **noiseprotocol.org**, **curl.se**, **signal.org**, **c2sp.org/age**, **torproject.org**, **i2pd.website**, **mullvad.net**, **privacyguides.org**, **sequoia-pgp.org**. Settings is
the LibreWolf-stance page (native prefs, black/yellow). CSS
`font-family` is engine slots only; `list-style` / `white-space` are in.
Letterboxing paint exists (pref off).

**E Isolation**  
One long-lived `--content-worker` per `IsolationKey`. Child applies
`no_new_privs` + landlock + seccomp-bpf + rlimits (256M / 128 fds /
nproc 0 / no core). Chrome never applies the sandbox. Audit test:
a landlock child **cannot** `open()` a profile `prefs.toml`. The
isolation key and cookie jar include the circuit (direct / tor /
i2p). Do not split a network process yet.

**F Linux homes**  
Detect + Tails/Qubes-DVM ephemeral default + packaging notes. Packages
not published.

**G Script**  
Refuse-only on the default build. Pref flip is not a grant.
`javascript:` refused. Servo JS is per-site and clearnet-only when the
`servo` feature is built. Boa evaluates one source when `frihart-js` is
built with feature `boa`; the chrome does not call it yet. The own
engine folder is `crates/frihart-own` and the browser does not select it.

## Plan (do these, in this order)

1. **Bridges (OPSEC).** `about:bridges` + `tor.require_bridges` + `allow_tor_fetch` are in. Next OPSEC slice: ControlPort bridge-status query so require=on can pass when Tor reports Active.
2. **This / next session — D.** The named queue is empty. Pick one more
   static document (not Wikipedia / GitHub / mail) and claim it from live
   HTML. Forty public claims; CSS leftovers done. **RFC 8439**
   claimed 2026-10-17 (ChaCha20-Poly1305, pre page). **sequoia-pgp.org**
   claimed 2026-10-16. **libsodium.org** Forbidden
   (apex→doc→gitbook.io) — unclaimed / NeedsJs. **age-encryption.org**
   Forbidden (apex → GitHub) — unclaimed / NeedsJs.
2. **Keep claiming static documents** one host/path per session until
   the named list feels like a daily driver for docs/RFCs/homepages.
   Do not claim Wikipedia, GitHub, mail, or any JS app.
3. **CSS first subset:** `list-style`, `white-space`, `font-family`
   slots, letterboxing paint, `th` bold — all in.
4. **E — isolation you can audit.** Landlock child cannot `open()`
   `prefs.toml` (test in). IsolationKey + cookie jar include the
   circuit (direct/tor/i2p). Do **not** split a network process yet.
5. **F — install on one Linux.** Build the Arch PKGBUILD locally on
   CachyOS/Arch. Fedora spec and Debian files exist; do not publish
   Tails/Qubes packages until a stranger can install from those files.
6. **v0.2.0** when: named static list is claimed (25 already), worker
   sandbox on, `cargo test --workspace` green, one reference distro
   package actually installs (`pacman -U` the `.pkg.tar.zst`).
7. **Boa M1** after the next static claim: feature `boa`, inline
   scripts, no network, HostApi still refused, off on Tor and I2P.
8. **H and I stay parked** until Linux is a daily driver.

## Recommendations

1. **One crate-visible slice per session.** Engine morning. Chrome only
   if something is unreadable.
2. **Claim sites by opening live HTML**, never by adding a URL to a
   list. Compatibility is per document, not “the web.”
3. **Servo and Boa are accepted bridges.** They exist so the browser
   ships before Campaign J. Pref flip on the default build is still not
   a grant.
4. **Do not embed** V8, JavaScriptCore, QuickJS, Gecko, Blink, WebKit
   or CEF. SpiderMonkey arrives only inside the feature-gated Servo
   Compat engine, and never on Tor or I2P. Do not execute Firefox add-ons.
5. **Do not split the network process** until the claim list is useful.
   Isolation theater is worse than documented in-process rustls.
6. **Do not publish Tails/Qubes packages** until Arch/Fedora/Debian
   install from the files we already have.
7. **Keep H and I parked.** Ports before Linux is useful waste the
   one-to-two-year commitment.
8. **Unsafe stays in `sandbox.rs` only.**
9. **Policy before I/O.** Tor fail-closed. Downloads never execute.
   Passwords never stored.
10. **Do not reopen** claimed sites unless they regress.

## Claimed (do not reopen unless they regress)

example.com, RFC 1918 HTML, suckless.org, GNU philosophy, kernel.org,
docs.kernel.org, ietf.org, rfc-editor.org index, w3.org landing,
w3.org/TR index, webarch, RFC 9110 HTML, WCAG 2.2, RFC 8446 HTML,
RFC 5280 HTML, RFC 8032 HTML (Ed25519), RFC 7748 HTML (Curve25519 /
X25519), RFC 5869 HTML (HKDF), openbsd.org, openssh.com
(→ openssh.org), libressl.org, openbsdfoundation.org, openbgpd.org, opensmtpd.org, openntpd.org, openiked.org, bearssl.org, wireguard.com, gnupg.org, noiseprotocol.org, curl.se, signal.org, c2sp.org/age, torproject.org, i2pd.website, mullvad.net, privacyguides.org, tails.net, sequoia-pgp.org, RFC 8439 HTML
(ChaCha20-Poly1305).

## Next session — start here

**D: claim one more static document** (not Wikipedia / GitHub / mail).
Forty public sites (RFC 8439 last Claimed). Servo is the Compat engine
for clearnet. The own engine is Reader, Tor, and I2P, and it replaces
Servo when `content.own_replaces_compat` is set. On 2026-10-17 the Servo
request gate refuses `Referer` and a `Cookie` that is not same-site with
the document. The Compat document is still a `data:` URL, so `Cookie`
is refused. Do not make Servo the product engine.
**libsodium.org** Forbidden (apex/www 307 → doc.libsodium.org 302 →
libsodium.gitbook.io/doc) — NeedsJs, left unclaimed.
**age-encryption.org** Forbidden (apex → GitHub) — NeedsJs, left
unclaimed.
Arch package builds; install is
`sudo pacman -U`. Default build still refuses script. Next JS slice is
Boa M1, feature-gated, not the product default.

## Commands

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -- about:campaigns
cargo run -- about:sites
cargo run -- https://example.com/
cargo run -- --tor
cargo run -- --i2p
```

## Docs to read first

`PHILOSOPHY.md`, `docs/stance.md`, `ROADMAP.md`, `ARCHITECTURE.md`,
`docs/opsec.md`, `docs/distros.md`, `docs/engine.md`,
`docs/css-subset.md`, `docs/sites.md`, `docs/defaults.md`, `docs/stance.md`,
`docs/community.md`.

## Hard rules

- Policy before I/O (`frihart-privacy`).
- No telemetry crate, no phone-home.
- Tor never falls back to clearnet.
- I2P never falls back to clearnet.
- Downloads never execute (`0600`).
- Passwords never stored. Proton Pass / KeePassXC / etc. are external.
- Wipe = session like new (bookmarks stay). Reset = prefs default
  (bookmarks stay). Shred = this profile only.
- Capability ladder. No Electron/Chromium embed.
- `unsafe` only in `frihart-platform/src/sandbox.rs`.

## About pages that matter

`about:home`, `about:campaigns`, `about:engine`, `about:sites`,
`about:processes`, `about:linux`, `about:script`, `about:shred`,
`about:tor`, `about:i2p`, `about:dns`, `about:stack`.
