# Handoff — continue Frihart

Tag: **v0.1.0**. Head: `main` on Codeberg (primary) and GitHub (mirror).

Do **not** start H (Windows/macOS/Android) or I (media decode, i18n
depth, print/PDF, extension runtime). JS stays off. Do not write a JS
engine.

## Who / what

- **Frihart** — original Rust privacy browser. Not a Firefox/LibreWolf/
  Chromium fork. LibreWolf is the **stance**. Look: **black + yellow**.
- **Libertarian:** user owns the machine and the binary (`MIT OR
  Apache-2.0`). No telemetry, no account, no password vault.
- **Linux first:** Arch/CachyOS reference, then Fedora, Mint, Tails,
  Qubes. Same binary. Linux free; other OS €100 later, no license server.
- **The engine is Frihart, in Rust** (locked). Never embed V8,
  SpiderMonkey, JavaScriptCore, QuickJS, Gecko, Blink, WebKit, Servo,
  or CEF. Campaign G, if it ever starts, is a small interpreter we write
  here. It has **not** been started.

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
a column grid. `hr`, caption, definition lists. CSS: `em`/`rem`/`vw`/
`vh`/`%`, `font-weight`, `border`, `:link`. Nested `<strong>` is its
own fragment. Forms GET/POST (secrets skipped). JS off. Img is a box.
UTF-8 text (not Latin-1 bytes). Heading with one destination (link-only
or link + extra title) is one hit. `svg`/`path`/`canvas` skipped.
`about:sites` claimed: **example.com**, **RFC 1918**, **suckless.org**,
**GNU philosophy**, **kernel.org**, **docs.kernel.org**, **ietf.org**,
**rfc-editor.org** (index), **w3.org** (landing). Settings is the
LibreWolf-stance page (native prefs, black/yellow).

**E Isolation**  
One long-lived `--content-worker` per `IsolationKey`. Child applies
`no_new_privs` + landlock + seccomp-bpf + rlimits (256M / 128 fds /
nproc 0 / no core). Chrome never applies the sandbox.

**F Linux homes**  
Detect + Tails/Qubes-DVM ephemeral default + packaging notes. Packages
not published.

**G Script**  
Refuse-only. Pref flip is not a grant. `javascript:` refused.
**Locked:** no foreign JS engine. A future G is Frihart Rust only.

## Plan (do these, in this order)

1. **This / next session — D only.** Fetch live `https://www.w3.org/TR/`
   (the TR index, not the already-claimed landing). Lay out those exact
   bytes. Fix what the subset mangles. Claim only if honestly readable.
   Write why on `about:sites` and `docs/sites.md`.
2. **Keep claiming static documents** one host/path per session until
   the named list feels like a daily driver for docs/RFCs/homepages.
   Do not claim Wikipedia, GitHub, mail, or any JS app.
3. **Document CSS leftovers (still D, after a few more claims):**
   `font-family` from the engine font list only (no web fonts);
   `th` vs `td` weight if a table page needs it; letterboxing paint
   (pref exists, paint does not).
4. **E — isolation you can audit.** After more claims, add a test that
   the content worker cannot `open()` the profile `prefs.toml`. Do
   **not** split a network process yet.
5. **F — install on one Linux.** Build the Arch PKGBUILD locally on
   CachyOS/Arch. Fedora spec and Debian files exist; do not publish
   Tails/Qubes packages until a stranger can install from those files.
6. **v0.2.0** when: named static list is claimed (9 already), worker
   sandbox on, `cargo test --workspace` green, one reference distro
   package actually installs.
7. **Then — and only then — discuss G.** Recommendation: stay refuse
   another year. If G starts, it is a tiny Frihart interpreter in Rust.
8. **H and I stay parked** until Linux is a daily driver.

## Recommendations

1. **One crate-visible slice per session.** Engine morning. Chrome only
   if something is unreadable.
2. **Claim sites by opening live HTML**, never by adding a URL to a
   list. Compatibility is per document, not “the web.”
3. **Do not start a JS engine.** Locked: Frihart Rust only. Starting G
   now eats a year and leaks. Pref flip is not a grant.
4. **Do not embed** V8, SpiderMonkey, JavaScriptCore, QuickJS, Gecko,
   Blink, WebKit, Servo, or CEF. Do not execute Firefox add-ons.
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
docs.kernel.org, ietf.org, rfc-editor.org index, w3.org landing.

## Next session — start here

**D: fetch live `https://www.w3.org/TR/` (or another unclaimed static
document), lay out those bytes, claim only if readable.** Nine public
sites are claimed. Do not start a JS engine. The engine stays Frihart
in Rust; G stays refuse. Do not start H/I.

## Commands

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -- about:campaigns
cargo run -- about:sites
cargo run -- https://example.com/
cargo run -- --tor
```

## Docs to read first

`PHILOSOPHY.md`, `ROADMAP.md`, `ARCHITECTURE.md`, `docs/opsec.md`,
`docs/distros.md`, `docs/engine.md`, `docs/css-subset.md`,
`docs/sites.md`, `docs/defaults.md`.

## Hard rules

- Policy before I/O (`frihart-privacy`).
- No telemetry crate, no phone-home.
- Tor never falls back to clearnet.
- Downloads never execute (`0600`).
- Passwords never stored. Proton Pass / KeePassXC / etc. are external.
- Wipe = session like new (bookmarks stay). Reset = prefs default
  (bookmarks stay). Shred = this profile only.
- Capability ladder. No Electron/Chromium embed.
- `unsafe` only in `frihart-platform/src/sandbox.rs`.

## About pages that matter

`about:home`, `about:campaigns`, `about:engine`, `about:sites`,
`about:processes`, `about:linux`, `about:script`, `about:shred`,
`about:tor`.
