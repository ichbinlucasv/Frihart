# Contributing to Frihart

This tree is meant to last. Small, reviewable changes beat heroic
branches. Read [PHILOSOPHY.md](PHILOSOPHY.md) and
[ARCHITECTURE.md](ARCHITECTURE.md) before writing code.

## Ground rules

1. No telemetry. A PR that adds a network call the user did not initiate
   will be rejected.
2. Put code in the crate that already owns the problem. If none does,
   propose the crate in `ARCHITECTURE.md` in the same PR.
3. Changing a default requires an update to `docs/defaults.md`.
4. Policy decisions belong in `frihart-privacy`, not in an `if` next to
   a socket.
5. Do not vendor Gecko, Blink, WebKit, Servo, or CEF.
6. Do not add a GUI framework that would own the chrome.

## Workflow

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Keep PRs to one idea. A parser fix and a chrome color change are two
PRs.

Commit messages are imperative and specific:

```
privacy: deny third-party cookie writes in the jar
chrome: focus the URL bar on Ctrl+L
html: recover from unexpected null in tokenizer
```

## Rust style

- Stable Rust only.
- `unwrap` is forbidden in library crates except for proven invariants,
  and then only with `expect` and a reason.
- Errors go through `frihart_core::FrihartError` or a crate-local error
  that converts to it at the boundary.
- No `unsafe` without a comment that states the invariant and a test
  that would fail if it were violated.
- Public items get a one-line doc comment.

## Adding a preference

1. Add a field to `frihart_config::Prefs` with a default.
2. Document it in `docs/defaults.md`.
3. If it changes a privilege, teach `frihart-privacy` about it.
4. Surface it on `about:settings` or `about:config`, not only in the file.

## Adding an `about:` page

Internal pages are structured data in `frihart-content`. Do not invent a
second HTML dialect for chrome pages.

## Platform work

Linux is the reference. Windows, macOS, and Android patches land in
`frihart-platform` (and later, a mobile chrome crate). Do not sprinkle
`#[cfg(windows)]` through the engine to "get ahead."

## What we are not looking for

- "Just embed WebView"
- Default search partnerships
- Onboarding tours
- Auto-update services that report the machine
- Drive-by dependency upgrades with no explanation
