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
`about:sites`: **example.com** and **RFC 1918** are claimed (live
HTML 2026-08-18). GNU / suckless / kernel.org stay targets.

**E Isolation**  
One long-lived `--content-worker` per `IsolationKey`. Child applies
`no_new_privs` + landlock + seccomp-bpf + rlimits (256M / 128 fds /
nproc 0 / no core). Chrome never applies the sandbox.

**F Linux homes**  
Detect + Tails/Qubes-DVM ephemeral default + packaging notes. Packages
not published.

**G Script**  
Refuse-only. Pref flip is not a grant. `javascript:` refused.

## Plan to finish (honest)

A general-purpose engine is a decade. “Finish” here means **v0.2:
Linux daily-driver for an honest, named list of static documents**.
Not Chrome. Not Wikipedia. Not mail.

### Milestone 1 — first claimed public site

1. ~~Open `https://example.com/`~~ **done**.
2. ~~Open RFC 1918 HTML~~ **done** (pre pages + `span.h1` title;
   also 5120×1440).
3. Repeat for GNU philosophy or suckless.org.
4. Do **not** claim a site you have not opened.

### Milestone 2 — document CSS (still D)

6. `%` width against the viewport (containing block = layout width).
7. `font-family` only from the engine font list (no web fonts).
8. `white-space` / `pre-wrap` already partly exists via `preserve`.
9. Nested inline style (`<strong>` inside a paragraph) as a separate
   fragment, so UA bold actually paints.
10. `th` vs `td` weight in the table grid.

### Milestone 3 — isolation you can audit (E)

11. Network stays in chrome until a **network process** exists. Next
    real split: chrome talks rustls only through `frihart-ipc` to a
    `--net-worker` that holds cookies. Do not start this until 1–5
    work; it is a large cut.
12. Worker death must reload **that** tab only (already mostly true).
13. Audit test: worker cannot `open()` the profile `prefs.toml`
    (landlock should already deny `$HOME`).

### Milestone 4 — Linux you can install (F)

14. Build the Arch PKGBUILD on CachyOS/Arch and install locally.
15. Same for Fedora spec and Debian package.
16. Tails/Qubes stay notes until 14–15 are real packages a stranger
    can install.

### Milestone 5 — v0.2 tag

17. Tag **v0.2.0** when: at least **3 named public static sites** are
    claimed, worker sandbox is on, `cargo test --workspace` green,
    Linux package files install on one reference distro.
18. Then — and only then — discuss G (a tiny interpreter) or stay
    refuse-only another year. Recommendation: **stay refuse**.

### Parked until Linux is a daily driver

- **H** Windows → macOS → Android
- **I** media decode, i18n depth, print/PDF, extension JS runtime
- Multi-window, HTTP/2 (leftovers, not blockers)

## Recommendations

1. **One crate-visible slice per session.** Engine morning, chrome
   only if something is unreadable.
2. **Claim sites by opening them**, not by adding URLs to a list.
3. **Do not start a JS engine.** It will eat a year and leak.
4. **Do not split the network process** until three sites are claimed.
   Isolation theater is worse than a documented in-process rustls.
5. **Do not publish Tails/Qubes packages** until Arch/Fedora/Debian
   install from the files we already have.
6. **Keep H and I parked.** Ports before Linux is useful waste the
   one-to-two-year commitment.
7. **Unsafe stays in `sandbox.rs` only.**
8. **Policy before I/O.** Tor fail-closed. Downloads never execute.
   Passwords never stored.

## Next session — start here

**D: open `https://suckless.org/` (or GNU philosophy), lay out the
live HTML, fix what the subset mangles, claim only if readable.**
Do not start H/I. JS stays off. Do not execute Firefox add-ons.

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
