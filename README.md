# Frihart

A **libertarian**, privacy-first web browser written in **Rust**.
Original code. Not a fork of Firefox, LibreWolf, Chromium, or anything
else.

**LibreWolf is the inspiration** — telemetry gone, fingerprinting
resisted, you are sovereign — implemented as native features, not
add-ons. The look is **black chrome, yellow accents**.

Linux is the product. Arch and CachyOS are the reference. Fedora, Mint,
Tails, and Qubes are first-class homes. Other Linux uses the same
binary. Windows, then macOS, then Android wait until Linux is actually
useful.

This is a long project. The chrome runs. The engine paints a growing
HTML subset. JavaScript is off. That is intentional. Read
[PHILOSOPHY.md](PHILOSOPHY.md) and [ROADMAP.md](ROADMAP.md).

## Why this exists

Firefox and Chrome are attack surfaces the size of an OS: C/C++ memory
corruption, huge IPC, GPU, and JS, plus telemetry and account gravity.
A LibreWolf-style *fork* still rebases that tree.

Frihart starts smaller:

- Rust, so use-after-free is not the weekly news
- Policy before any network or disk write
- Tor tabs that **fail closed** (no clearnet fallback)
- No login vault, no Frihart account, no phone-home
- A documented subset, not a fake "we render the whole web"

We are not "better than Firefox" today. We are building a browser that
is **harder to own**, for people who treat a leak as a failure.

## Repositories

- **Primary:** [codeberg.org/ichbinlucasv/Frihart](https://codeberg.org/ichbinlucasv/Frihart)
- **Mirror:** [github.com/ichbinlucasv/Frihart](https://github.com/ichbinlucasv/Frihart)

## Product (Linux)

- Black / yellow chrome (LibreWolf stance, our pixels)
- Identity **containers** in the tab strip (`about:containers`)
- Native **uBlock-class blocker**, on at install (`about:blocker`)
- Built-in **translator**, DeepL default, no Google (`about:translate`)
- Swisscows search, DuckDuckGo second (`about:search`)
- Tor tabs (`--tor`, Ctrl+Shift+O) via your system daemon
- ProtonVPN / Mullvad CLI hooks (`about:vpn`)
- Wipe / reset / shred this profile only (`about:shred`)
- Identity autofill; **never** a password store (`about:pass`)
- rustls fetch, first-party partitioned cookies, HTTPS-only
- HTML → CSS → layout → display list (`about:engine`)

Linux is free. Other OS: €100 lifetime, local key, no license server.
See [docs/pricing.md](docs/pricing.md).

## Linux homes

| Distro | Status |
| --- | --- |
| Arch, CachyOS | Reference. `packaging/arch/PKGBUILD` |
| Fedora | `packaging/fedora/frihart.spec` |
| Mint (Debian/Ubuntu) | `packaging/debian/` |
| Tails | Amnesic default (`--private` unless `--profile`). Use Tails Tor |
| Qubes OS | DisposableVM = private profile. Fedora & Debian template notes |
| Other Linux | Same binary. Wayland first, X11 while it lasts |

Details: [docs/distros.md](docs/distros.md). OPSEC: [docs/opsec.md](docs/opsec.md).

## Current status

**v0.1.0.** Campaigns **A, B, C** (crate phases 0–2) are **closed**.
**D, E, F, G** are open. **H** and **I** are parked.

On Linux, `cargo run` opens a real window. `https://` fetches over rustls
and paints the subset via a sandboxed `--content-worker` (`no_new_privs`
+ landlock + seccomp-bpf + rlimits; in-process fallback if the worker
dies). CSS understands `em`/`rem`, `font-weight`, and `border`. Find
(Ctrl+F) searches the display list. JS is off. `javascript:` is refused.
Tor tabs dial SOCKS only. `about:sites` claims `example.com`, RFC 1918,
suckless.org, and GNU philosophy. `about:settings` is the LibreWolf-stance
page. Tracking query keys are stripped. Private-IP redirects are refused.
Content width follows the window up to 2400 CSS px (G9-class).

Next session: [docs/HANDOFF.md](docs/HANDOFF.md).

## Build

Recent stable Rust (`rust-toolchain.toml`) and a Linux desktop (Wayland
or X11).

```bash
cargo build --release
cargo run
```

```bash
cargo run -- about:settings
cargo run -- --private
cargo run -- --tor
cargo run -- --install-addon ./some-firefox-addon.xpi
cargo run -- --profile ./profile-dev
```

```
frihart [URL] [--profile PATH] [--private] [--tor] [--version]
```

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Profile

```
$XDG_DATA_HOME/frihart/profiles/default/
    prefs.toml
    bookmarks.toml
    history.jsonl
    user.css          # optional; you create it
    downloads.json
    lock
```

Private windows use memory only. Files are `0600` / dirs `0700`.

## Documentation

| File | What it is |
| --- | --- |
| [PHILOSOPHY.md](PHILOSOPHY.md) | Constitution. Libertarian, LibreWolf stance |
| [ROADMAP.md](ROADMAP.md) | Campaigns A–I and crate phases 0–15 |
| [docs/HANDOFF.md](docs/HANDOFF.md) | Where to continue (A–C closed) |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Crate map |
| [docs/opsec.md](docs/opsec.md) | Standing OPSEC rules |
| [docs/distros.md](docs/distros.md) | Arch, Cachy, Fedora, Mint, Tails, Qubes |
| [docs/engine.md](docs/engine.md) | HTML → display list |
| [docs/css-subset.md](docs/css-subset.md) | CSS we implement vs ignore |
| [docs/defaults.md](docs/defaults.md) | Every shipped default and why |
| [docs/packaging.md](docs/packaging.md) | How to package |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to work on the tree |
| [SECURITY.md](SECURITY.md) | Threat model and reporting |

## License

MIT OR Apache-2.0. You own what you run.

## Name

Frihart is the name of the browser. It is not a reskin and it does not
stand for an acronym.
