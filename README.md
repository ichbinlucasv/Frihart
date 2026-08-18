# Frihart

Frihart is an original, privacy-first web browser written in Rust.
It is not a fork of Firefox, LibreWolf, Chromium, or anything else.
**LibreWolf is the inspiration** — telemetry gone, fingerprinting
resisted, the user is sovereign — implemented as original code.

The primary platform is Linux (Arch and CachyOS first). Windows, then
macOS, then Android come later — in that order, and not before Linux is
actually useful.

This repository is at the beginning of a long project. The chrome runs.
The web engine does not, yet. That is intentional. Read
[ROADMAP.md](ROADMAP.md).

## Repositories

- **Primary:** [codeberg.org/ichbinlucasv/Frihart](https://codeberg.org/ichbinlucasv/Frihart)
- **Mirror:** [github.com/ichbinlucasv/Frihart](https://github.com/ichbinlucasv/Frihart)

## Phase 1 product

- Identity **containers** built into the tab strip (`about:containers`)
- Native **uBlock-class blocker**, on at install (`about:blocker`)
- Built-in **translator** chrome, no Google (`about:translate`)
- **Dark mode** as the product look: black chrome, yellow accents

## Principles

- No telemetry. Not now, not hidden, not "anonymous."
- Anti-tracking and fingerprinting resistance are defaults, not add-ons.
- The user is sovereign. Power is available. Defaults still protect.
- Original architecture. Shared primitives (TLS, fonts, Unicode) are
  allowed. Shipping someone else's browser is not.

The full constitution is [PHILOSOPHY.md](PHILOSOPHY.md).

## Current status

**Phase 1.** On Linux, `cargo run` opens a real window:

- Black chrome, yellow accents, dark pages
- Tab strip with container stripes, URL bar, navigation keys
- Native blocker, containers, translator (`about:blocker`,
  `about:containers`, `about:translate`)
- Find in page (Ctrl+F), bookmark (Ctrl+D), cycle container (Ctrl+Shift+C)
- Privacy-first prefs persisted in an XDG profile
- No network on startup, and no web rendering yet

Typing an `https://` URL shows an honest "not yet" page. Phase 2 is the
network stack. Phases 3–5 are the document engine.

## Build

Requires a recent stable Rust (see `rust-toolchain.toml`) and a Linux
desktop with Wayland or X11.

```bash
cargo build --release
cargo run
```

Useful invocations:

```bash
cargo run -- about:settings
cargo run -- --private
cargo run -- --profile ./profile-dev
```

```
frihart [URL] [--profile PATH] [--private] [--version]
```

Tests (no display required):

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Profile

Default location on Linux:

```
$XDG_DATA_HOME/frihart/profiles/default/
    prefs.toml
    bookmarks.toml
    history.jsonl
    lock
```

Private windows use memory only.

## Documentation

| File | What it is |
| --- | --- |
| [PHILOSOPHY.md](PHILOSOPHY.md) | Why the product exists, and what it will refuse |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Modular design and crate contracts |
| [ROADMAP.md](ROADMAP.md) | Phases 0–11, milestones, time ranges |
| [docs/defaults.md](docs/defaults.md) | Every shipped default and why |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to work on the tree |
| [SECURITY.md](SECURITY.md) | Threat model and reporting |

## License

MIT OR Apache-2.0. You own what you run.

## Name

Frihart is the name of the browser. It is not a reskin of another
product and it does not stand for an acronym.
