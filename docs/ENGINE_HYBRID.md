# ADR: Hybrid engine — Servo Compat + Frihart's own engine

- **Status:** Accepted (Lucas, 2026-10-13)
- **Supersedes:** the "we will not embed another engine" stance in
  PHILOSOPHY.md, docs/stance.md, README and ROADMAP (Campaign G lock).

## Decision

Frihart becomes a **hybrid** browser:

- **Frihart's own engine** (Rust, this tree) stays. It renders **Tor
  tabs, I2P tabs and reader mode**, and remains the privacy-hardened
  default wherever it works.
- **Servo** (Rust, Linux Foundation, embedded through its `servo`
  embedding crate, historically called `libservo`) is added as the
  **Compat engine** for modern clearnet sites that need JS and modern CSS.

## Rationale

People will use Frihart for everything: Reddit, social media, news, tech
and cybersecurity news, Hacker News, Android sites, government and
biotech portals. Those sites need JavaScript, flexbox/grid, web fonts and
web APIs. With an engine written from scratch, that is more than a
decade away. Servo is a modern engine, mostly Rust, governed in the open,
with an embedding API. Using it for clearnet compatibility gives a usable
daily browser in roughly 1–2 years, while Frihart's engine keeps its job
as the small, auditable, fail-closed surface for anonymity networks.

Alternatives rejected: own engine only (usable for docs only, for years);
Gecko (C++, LibreWolf/Tor Browser already exist, no longer ours).

## Architecture

Engine selection is **per tab**, decided in one place
(`frihart_engine::select_engine`):

| Tab network | Mode | Engine |
| --- | --- | --- |
| Tor | any | Own engine (always) |
| I2P | any | Own engine (always) |
| Clearnet | Reader | Own engine |
| Clearnet | Compat | Servo (if built with `servo`), else own engine |

Both engines implement the `Engine` trait (`crates/frihart-engine`).
The chrome (tabs, address bar, protections panel, status bar) is the same
for both; the tab badge shows which engine is active (`OWN` / `COMPAT`).
Servo code lives only in `crates/frihart-engine-servo`, behind the cargo
feature `servo`, off by default.

## Privacy hardening for Servo

1. **Networking through Frihart.** Servo's network loads are routed
   through Frihart's net layer / local proxy (`frihart-net`): blocker,
   tracking-parameter stripping, private-IP redirect refusal, cookie and
   referrer policy, DNS policy. No direct sockets from Servo (M3).
2. **Frozen user agent** and fingerprinting surface matching the
   LibreWolf-style defaults (one UA, fixed fonts/timezone exposure where
   Servo allows it).
3. **No telemetry, no crash upload, no devtools server** in release
   builds; `devtools`, `bluetooth`, `webxr`, `gamepad`, `webgpu` features
   off.
4. **JS per-site permission.** JS is off unless the user enables it for a
   site (M4). The default may become "on for Compat tabs" only with M5.
5. **Fail-closed on Tor/I2P.** Servo **must never** handle a Tor or I2P
   tab until a leak audit (DNS, WebRTC, proxy bypass, timing, fonts)
   proves it safe. Enforced in code: `ServoEngine::allowed_on` returns
   false for Tor/I2P and `select_engine` never picks Servo for them.
6. **Sandbox.** Run Servo in its multiprocess mode inside Frihart's
   landlock/seccomp content worker when feasible.

## Risks

- **Build size and time.** Servo pulls ~80 direct dependencies
  (SpiderMonkey via `mozjs`, WebRender, Stylo, surfman, tokio, GStreamer
  for media). Expect a cold release build of **30–60 min** on 8 cores,
  ~**10–20 GB** of `target/`, and a binary of roughly **60–120 MB**
  (vs. a few MB today). Mitigation: feature-gated, separate CI job.
- **Not 100% Rust.** Servo's JavaScript engine is **SpiderMonkey (C++)**
  through `mozjs`. Frihart's own code stays Rust-only; the Compat engine
  inherits that C++ surface. This is the price of real-site JS.
- **Servo maturity.** Servo is improving fast but does not open every
  site; some sites will still break. Embedding API is pre-1.0 and changes
  between releases — pin the version and upgrade deliberately.
- **License.** Servo is MPL-2.0 (file-level copyleft). Compatible with
  linking from MIT/Apache Frihart; Servo files stay MPL.
- **Attack surface.** A JS engine is an exploit runtime. Mitigated by
  per-site JS, sandboxing and keeping Tor/I2P on the own engine.

## M1 findings (2026-10-13)

- `libservo` is **not** a crates.io name. The embedding crate is
  published as **`servo` on crates.io, version 0.7.0** (released
  2026-10-05), MPL-2.0, **MSRV 1.88** (our toolchain: 1.98; workspace
  `rust-version` 1.85 would need raising only for the Servo crate).
  Also available by git from `https://github.com/servo/servo`.
- Features include `default`, `js_jit`, `media-gstreamer`, `webgl`,
  `webgpu`, `webxr`, `bluetooth`, `multiprocess`, `vello`, `baked-in-resources`.
- Expected Linux system dependencies (Servo build docs): clang/LLVM,
  python3, cmake, pkg-config, GStreamer dev packages (if media is on),
  fontconfig/freetype, libX11/xcb/wayland, EGL/Mesa (surfman).
- **Not built yet.** A full build was not attempted in this session to
  avoid a heavy build in the default workspace. Size and time above are
  estimates. M1 continues with an out-of-default spike build.
- Done now: `crates/frihart-engine` (`Engine` trait, `OwnEngine`,
  `select_engine`) and an empty `crates/frihart-engine-servo` with the
  `servo` feature (no dependency yet), both tested.

## Milestones (Campaign S — Servo Compat engine)

- **M1 — Feasibility spike.** `servo` builds on Linux behind the cargo
  feature `servo`, off by default; record real build time and size.
- **M2 — Render a clearnet tab via Servo** inside Frihart's chrome.
- **M3 — Force all Servo networking through Frihart's proxy / net layer.**
- **M4 — Per-site JS toggle** in the protections panel.
- **M5 — Compat becomes the default for clearnet tabs** (own engine stays
  for Tor/I2P/reader).

## Parallel JS tracks (approved by Lucas, 2026-10-13)

Two campaigns run beside Servo Compat; details live in
[ROADMAP.md](../ROADMAP.md):

- **Campaign B-JS — Boa light JS.** Embed Boa (Rust) into Frihart's *own*
  engine behind a feature flag, off by default, enabled per site in the
  shields panel. Bridge for progressive-enhancement sites until the own
  engine can run JS. Tor/I2P stay refuse until a leak audit.
- **Campaign J — Frihart's own JS engine (10-year, 100% Rust).** Grow a
  real JS engine in `crates/frihart-js` (parser → interpreter →
  conformance → JIT → Web APIs → performance). Yearly metrics: test262 %,
  claimed sites, bench speed. Gradually replaces Boa per tab, then Servo
  on clearnet when metrics win. End state: no SpiderMonkey, no Boa.

Per-tab selection stays the table above until B-JS/J change it. Servo
never handles Tor/I2P; Boa and Frihart JS inherit that rule until audited.
