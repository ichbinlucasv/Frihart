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
- Done 2026-10-13: `crates/frihart-engine` (`Engine` trait, `OwnEngine`,
  `select_engine`) and the `frihart-engine-servo` crate with the `servo`
  feature.

## M1 result (2026-10-14): Servo builds and renders

`servo = "=0.7.0"` is now a real optional dependency of
`frihart-engine-servo` (feature `servo`, off by default; crate
`rust-version` 1.88, rest of the workspace 1.85). Features used:
`bundled` + `js_jit` only; clipboard, multiprocess, WebGL, WebGPU, WebXR,
Bluetooth, gamepad and GStreamer media are off.

Measured on the 8-core / 16 GB build box (Debian 13, rustc 1.99):

| Build | Time | Output |
| --- | --- | --- |
| Cold dev build of all Servo deps (`-j6`) | 3 min 01 s | `target/` 2.9 GB for Servo alone |
| Workspace dev build with `--features servo` | 3 min 33 s | — |
| Release example (`-j4`, no LTO, 16 CGUs, shared box) | 10 min 12 s | 150 MB binary, 113 MB stripped |
| Debug example binary | — | 323 MB |

The workspace `target/` with both profiles reached about 15 GB. SpiderMonkey
was not compiled from source: `mozjs_sys` 153.3 downloads its prebuilt
archive, which is why the cold build is minutes, not the 30–60 min
estimated above. Build dependencies installed on the box: clang/LLVM
(19), libclang-dev, cmake, pkg-config, fontconfig/freetype/harfbuzz dev,
EGL/GLES/Mesa dev, X11/xcb/xkbcommon/wayland dev, libudev, libunwind.
The binary links only system libc/libstdc++, fontconfig, freetype,
libpng, zlib, bzip2, brotli and expat.

What works (tested, `cargo test -p frihart-engine-servo --features servo`):
`ServoEngine::render_html` renders off screen through Servo's
`SoftwareRenderingContext` (CPU, no window, no GPU) and returns the frame
as one `DisplayOp::Image`; the chrome can blit it (`Framebuffer::blit_rgba`).
The test page uses CSS grid and a script that changes the title and a
style; both show up in the pixels. One `Servo` instance is reused across
renders (one per process). `examples/servo_headless.rs` writes a PPM.

Privacy settings (`prefs::HardenedPrefs`, checked by `is_locked_down`
before Servo starts): Frihart's frozen user agent; devtools server,
WebRTC, geolocation, Bluetooth, gamepad, WebGPU, notifications and service
workers off; plain HTTP goes to a dead loopback proxy (port 9) and fails;
HTTPS goes to Frihart's Compat proxy (below). Tested: a subresource on a
local host is never contacted directly.

Trade-offs to know:

- **Lockfile.** Cargo resolves optional dependencies too, so `Cargo.lock`
  gained ~540 packages, and Servo's pins moved shared crates: ICU went
  from 2.3 to 2.1 (`icu_*`, `idna_adapter` 1.2.1) for every build,
  including the default one (it is `url`'s IDNA backend). Default builds
  still compile none of Servo.
- **`cargo audit`.** No vulnerabilities. Servo adds 7 "unmaintained"
  warnings (`bincode` 1, `paste`, five `unic-*` crates); the 4 existing
  ones (`rustybuzz`, `ttf-parser`) are unchanged.
- **TLS provider.** Servo enables rustls with aws-lc-rs, Frihart's net
  layer with ring; with both on, rustls cannot pick a default and Servo's
  resource thread panics. The adapter installs ring as the process
  provider before Servo starts.
- **Not 100% Rust**: SpiderMonkey (C++) arrives prebuilt from mozjs.

## M3 step (2026-10-14): Servo HTTPS goes through Frihart

`frihart_net::compat_proxy` is a loopback `CONNECT` proxy started per
Servo instance. Servo's HTTPS proxy preference points at it, so Frihart
does the DNS lookup and the policy check, and Servo only ever talks to
127.0.0.1: only port 443; `.onion`, `.i2p`, `.local`, `.internal`,
`.invalid`, `localhost` refused; IP literals and names resolving to
loopback, link-local or private ranges refused (DNS rebinding);
hosts on the native blocker lists refused; request head capped at 8 KiB
and 10 s. Tested end to end: Servo's requests for a special-use name and a
private IP reach the proxy and are refused without any lookup.

*2026-10-15:* Chrome strips tracking parameters on every navigate (and
`open_url`); the strip list grew (Matomo, TikTok `ttclid`, LinkedIn,
etc.). Compat clearnet fetches Frihart performs itself use
`SiteCookieJars` (one jar per first-party host), never the profile jar.
Still open for M3: in-tunnel Cookie / Client-Hints / header audit inside
Servo (the CONNECT tunnel is TLS end to end), and a leak test against a
live page.

## Supply chain (2026-10-17)

`cargo audit` is clean of vulnerabilities again: rustls 0.23.45 → 0.23.46
(RUSTSEC-2026-0346/0347/0348/0349: secrets in `Debug`, ECH data, Brotli
certificate allocation, ServerHello key-exchange downgrade), and zip 2.4.2
(yanked) → 9.0.4 (RUSTSEC-2026-0367/0368/0369, all about
`ZipArchive::extract`). Frihart never called `extract`; `materialize` now
also skips symlink entries, never writes through an existing symlink,
never applies archive permission bits, and caps entries (10 000) and
extracted size (256 MiB). zip 9 needs Rust 1.88, so the workspace
`rust-version` is now 1.88 (Debian 13's packaged rustc 1.85 can no longer
build Frihart; use rustup or a backport). The remaining audit output is
"unmaintained" warnings only (`rustybuzz`/`ttf-parser`, and Servo's
`bincode` 1, `paste`, `unic-*`).

## M2 step (2026-10-15): Compat mode in chrome

`Tab` carries `TabMode::{Reader, Compat}`. `prepare_frame` calls
`select_engine`; clearnet Compat uses `ServoEngine` when the `servo`
feature is on, otherwise falls back to the own engine. Tor and I2P tabs
are forced to Reader. Shift+E cycles the mode on the active tab.

## M4 step (2026-10-16): Per-site JS

`frihart_profile::SiteJsStore` persists host → bool under `site_js.toml`
(0600). Shields shows **JavaScript (this site)** for the active host;
clicks call `Browser::toggle_site_js`. Effective allow = override or
global `privacy.javascript` (default false). Tor/I2P always refuse.

Compat frames pass the flag into `ServoEngine::render_html_with_js`.
`HardenedPrefs.javascript` defaults false (`is_locked_down` requires it
off). Servo 0.7 has no master "disable scripts" preference, so when JS
is off Frihart strips `<script>` elements and sets `js_wasm_enabled:
false`. Reader mode may store an "on" override but does not run script
until Compat (Servo) or Campaign B-JS (Boa).

## Milestones (Campaign S — Servo Compat engine)

- **M1 — Feasibility spike.** `servo` builds on Linux behind the cargo
  feature `servo`, off by default; record real build time and size.
- **M2 — Render a clearnet tab via Servo** inside Frihart's chrome.
- **M3 — Force all Servo networking through Frihart's proxy / net layer.**
- **M4 — Per-site JS toggle** in the protections panel. **Done** (2026-10-16):
  profile `SiteJsStore`, shields row, `HardenedPrefs.javascript` (default
  false) + HTML `<script>` strip when off; Tor/I2P never enable.
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
