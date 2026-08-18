# Handoff — continue Frihart

Last session closed **campaigns A, B, C** (crate phases **0, 1, 2**).
Tag: **v0.1.0**. Head: `main` on Codeberg (primary) and GitHub (mirror).

Do **not** start H (Windows/macOS/Android) or I (media decode, i18n
depth, print/PDF, extension runtime). JS stays off.

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

## Open — work here

**D Engine (long pole)**  
HTML subset → CSS (class/id/descendant, user.css) → block layout +
cosmic-text wrap → display list → chrome paint. Tables are a **column
grid**. `hr` is a rule fill. `<caption>` and definition lists work.
Forms GET/POST encode (secrets skipped). Identity autofill only. JS
off. Img is a box (no decode). `about:sites` + `docs/sites.md` is the
honest claim list (nothing on the public internet claimed yet).

Done recently: find-in-page on the display list; **one worker process
per isolation key** (newline JSON jobs); form fields are display-list
ops (no second paint path). Crash falls back in-process.

Next D slices:

1. Verify a Target site from `about:sites` by actually opening it
2. More CSS (`border`, `em`/`rem`, `font-weight`)

**E Isolation**  
One long-lived `--content-worker` per `IsolationKey`. Child applies
`no_new_privs` + landlock + **seccomp-bpf** (EPERM on socket / connect /
clone / exec / ptrace / mount). Chrome never applies the sandbox.
Closing the last tab for a key kills that worker.

Next E slices:

1. Resource limits (fds, memory) per worker
2. Network process split (crate seam exists)

**F Linux homes**  
Detect Arch/Cachy/Manjaro/Endeavour, Fedora, Mint, Tails, Qubes
(`about:linux`). Tails and Qubes-DVM default to ephemeral unless
`--profile`. Desktop file has a Private action. Packaging notes exist;
full Tails/Qubes packages are not published.

**G Script**  
Refuse-only. Pref flip does not execute and does not open cookie /
storage / WebRTC / WebSocket. `javascript:` URLs (including
`javascript://`) parse then refuse — they never become a search query.
`about:script` lists every `HostApi`.

## Commands

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -- about:campaigns
cargo run -- about:sites
cargo run -- --tor
```

Tails/Qubes-DVM: omit `--profile` → memory-only profile.

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
