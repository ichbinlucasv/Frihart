# Testing Frihart

Two layers matter for Campaign D work: **unit / crate tests** and
**pipeline claim fixtures**. Chrome UI is not covered by CI.

## Full workspace (what CI runs)

GitHub Actions (mirror) and local checks are the same three commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

See `.github/workflows/ci.yml`. Format and Clippy fail the job; tests
must stay green before a claim lands.

## Pipeline (HTML → display list)

`frihart-pipeline` is the engine spine. `layout_html` /
`layout_html_ex` turn HTML (+ optional extra CSS) into a `Frame`
(title, boxes, display list).

Run only that crate:

```bash
cargo test -p frihart-pipeline
```

Claimed static documents live as fixtures under
`crates/frihart-pipeline/testdata/` (one `.html` file per host/path).
Pipeline tests `include_str!` those fixtures and assert readable
titles, hits, and wide-viewport paint (often `5120×1440`). Adding a
site claim means: capture live HTML into `testdata/`, add a test, and
update `docs/sites.md` / `about:sites` in the same change set — that
queue is owned by the claim agent; do not race the same files.

## Content (`frihart-content`)

Internal pages (`about:sites`, `about:settings`, …) are structured
data in `frihart-content`, not a second HTML dialect.

```bash
cargo test -p frihart-content
```

## CSS property tests

Property parsing and cascade live in `frihart-css` and `frihart-style`.
Layout behaviour is asserted through `frihart-pipeline` (and
`frihart-layout`) rather than by claiming a live host.

```bash
cargo test -p frihart-css
cargo test -p frihart-style
cargo test -p frihart-layout
```

Document honest coverage in [css-subset.md](css-subset.md). Site
claims stay in [sites.md](sites.md).

## Manual smoke (desktop)

```bash
cargo run -- about:sites
cargo run -- about:engine
cargo run -- https://example.com/
cargo run -- --tor
cargo run -- --i2p
```

Tor / I2P need your system daemons and **fail closed** (no clearnet
fallback). Private windows: `cargo run -- --private`.

## Remotes while testing

Push and PR against **Codeberg** first. GitHub is a mirror. See
[CONTRIBUTING.md](../CONTRIBUTING.md).
