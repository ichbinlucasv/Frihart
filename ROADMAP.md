# Frihart roadmap

This is the long-term plan for a **libertarian** Linux browser with a
LibreWolf stance and original Rust code. One to two years of foundation,
then years of engine depth. Dates move. The philosophy does not.

Read with [PHILOSOPHY.md](PHILOSOPHY.md), [ARCHITECTURE.md](ARCHITECTURE.md),
[docs/opsec.md](docs/opsec.md), and [docs/distros.md](docs/distros.md).

Crate numbers (0–15) stay. Humans use **campaigns A–I**.

## Why we do not fork Firefox

A Gecko or Blink fork inherits memory-unsafe RCEs, a release train we
do not control, and telemetry DNA. Frihart's bet is a smaller Rust
surface that fails closed. We will not match Chrome's site list. We
will be harder to own.

## Honest scale

A general-purpose engine that competes with Blink or Gecko is a
**decade-scale** project. Frihart does not hide that.

| Horizon | What "done" means |
| --- | --- |
| 3 months | Linux chrome you live in: prefs, tabs, bookmarks, wipe |
| 6–9 months | Fetch, isolate cookies, first real layout |
| 12–18 months | Simple documents (blogs, docs, homepages) without JS |
| 18–24 months | Process isolation, Linux sandbox, named daily-driver sites |
| Year 2 | Packages for Arch, CachyOS, Fedora, Mint; Tails + Qubes notes that are honest |
| Year 2–3 | Constrained scripting. Windows only after Linux is actually useful |
| Year 3–4 | macOS, then Android |
| Year 5+ | Compatibility, performance, engine completeness |

If a phase slips, we slip the date. We do not skip isolation to paint
more CSS, and we do not skip policy to fetch more pages.

## Platform order (fixed)

1. **Linux** — same binary everywhere
   1. Arch / CachyOS (reference)
   2. Fedora
   3. Linux Mint (Debian/Ubuntu family)
   4. Tails (amnesic, system Tor)
   5. Qubes OS (AppVM / DisposableVM)
   6. Every other Linux
2. Windows
3. macOS
4. Android

A port starts only when the previous rung is a daily driver for the
sites Frihart claims at that time.

## Campaigns (the new plan)

| Campaign | Crate phases | Goal |
| --- | --- | --- |
| **A Foundation** | 0 | Identity, license, crates, constitution |
| **B Chrome** | 1 | Black/yellow shell. Containers, blocker, wipe. LibreWolf stance |
| **C Network OPSEC** | 2 | rustls, partitioned cookies, Tor fail-closed, safe downloads |
| **D Engine** | 3–5, 12 | HTML/CSS/layout/forms. Documents, not webapps |
| **E Isolation** | 6 | Process split, seccomp, landlock. A tab cannot own the profile |
| **F Linux homes** | packaging | Arch, Cachy, Fedora, Mint, then Tails and Qubes |
| **G Script** | 7 | Last. Default conservative. Fingerprint APIs stay denied |
| **H Other OS** | 8–10 | **Parked.** Windows, macOS, Android after Linux is real |
| **I Depth** | 11, 13–15 | **Parked.** Media, i18n, print, extension runtime |

Work now is **A–G only**. Standing OPSEC: [docs/opsec.md](docs/opsec.md).

---

## Phase 0 — Identity and foundation

**When:** weeks 1–4 (started in this repository)
**Goal:** a project that can be worked on for years without being rewritten.

### Technical tasks

- Name, license (`MIT OR Apache-2.0`), philosophy, architecture, roadmap
- Cargo workspace with crate boundaries that match the process model
- Typed prefs and privacy-first defaults
- Profile directory, lockfile, bookmarks and history stores
- Policy engine as a pure crate
- `about:` document model (no HTML engine)
- Linux window + original chrome (tabs, URL bar, internal pages)
- `cargo test` and a Linux CI workflow
- Git history that starts clean

### Architectural decisions (locked)

- Original code. Not a fork.
- Rust, edition 2024, stable toolchain
- Privacy policy is a crate, not a checklist in the HTTP client
- Single process now; crate seams as if IPC already existed
- Software-rendered chrome; GPU compositor later
- Permissive license so users can actually own the binary they run

### Success criteria

- `cargo test --workspace` is green
- `cargo run` opens Frihart on Linux
- Tabs, URL bar, and `about:` pages work
- Prefs persist in the profile
- No network traffic on startup
- A stranger can read ARCHITECTURE.md and know where new code goes

---

## Phase 1 — Linux browser shell

**When:** months 1–3
**Goal:** the chrome is a real home. You change settings here. You keep
bookmarks here. You are not waiting on an engine to have a browser.

### Technical tasks

- Polish tab strip, keyboard, focus, scrolling, HiDPI
- Black / yellow dark chrome as the product look
- First-class **containers** (tab assignment, isolation keys)
- Native **uBlock-class blocker** (host engine + built-in seed)
- Built-in **translator** chrome (`about:translate`, no default cloud)
- Multiple windows
- Bookmarks manager as `about:bookmarks` (create, delete, persist)
- History viewer as `about:history` with wipe / disable
- Find-in-page for internal documents
- Session restore (off by default, implemented correctly)
- Private window: in-memory profile, distinct chrome cue
- `about:config` that is typed, not a string soup
- Arch/CachyOS packaging notes and a `PKGBUILD`
- Accessibility start: keyboard-only use must work
- Screenshot and visual checklist for the chrome

### Architectural decisions

- Chrome owns pixels of the frame; content owns the viewport document
- Internal pages stay structured data until the HTML engine can replace
  them *without* changing prefs or policy
- No GUI toolkit takeover. We keep the widget set small and ours.
- Containers are a profile object, not an extension.
- The blocker is a crate (`frihart-blocker`), not a WebExtension host.
- LibreWolf is inspiration. Gecko is not a dependency.

### Success criteria

- Daily usable as a "home base" even though the web is not rendered yet
- Keyboard-only navigation of chrome and `about:` pages
- Profile survives crash (lock released, files consistent)
- `PKGBUILD` builds a clean package on Arch

---

## Phase 2 — Sovereign network stack

**When:** months 3–6
**Goal:** Frihart can talk to the network the way a privacy browser must,
before it can paint a page.

### Technical tasks

- TLS via rustls, system or mozilla-backed roots (documented)
- HTTP/1.1 client, then HTTP/2
- Redirect handling, size limits, timeouts
- Cookie jar: first-party, partitioned, `HttpOnly` / `Secure` / `SameSite`
- Cache with explicit keys that include the isolation partition
- HTTPS-only upgrade and a visible exception UI
- Downloads to a user-chosen directory, no execute-on-download
- Referrer policy implementation matching prefs
- User-Agent frozen; no Client Hints
- Optional DoH, user-supplied URL only
- Certificate error pages that are honest
- Local integration tests against a test listener (no live web in CI)

### Architectural decisions

- `frihart-net` never imports chrome
- Every request is a `privacy::RequestContext`
- No HTTP/3 until fingerprinting and middlebox behavior are reviewed
- No default DoH vendor
- Cookies and cache are not world-readable files

### Success criteria

- `frihart https://example.com` fetches and can display **source**
- Third-party cookies cannot be set in tests
- Startup still makes zero unsolicited connections
- HTTPS-only blocks cleartext without a user exception
- A download cannot run itself

---

## Phase 3 — HTML and DOM

**When:** months 6–12
**Goal:** a correct, tested parser and a real DOM. Not a layout engine.

### Technical tasks

- Create `frihart-html`: WHATWG tokenizer + tree builder
- Create `frihart-dom`: nodes, attributes, ids, parent/child
- Parse fixtures from the html5lib test suite where we claim support
- Character encoding: UTF-8 first, then a documented decoder list
- `Document::Html` wired through navigation
- View-source as a first-class document
- Error recovery that matches the spec for the subset we implement
- Streaming parse (do not require the whole body in memory forever)

### Architectural decisions

- Write the parser. Do not vendor another engine's parser as the product.
- DOM is engine-owned. Chrome does not mutate it.
- No JS bindings in this phase. The tree is static after parse.
- Unknown tags stay in the tree; they do not crash the parser.

### Success criteria

- html5lib tokenizer tests we claim pass, pass in CI
- A set of hand-written documents round-trip through parse → serialize
- View-source works for any fetched document
- Malformed HTML cannot panic the content path

---

## Phase 4 — CSS, layout, and paint

**When:** months 10–18 (overlaps late Phase 3)
**Goal:** paint a useful subset of the web: documents, not web apps.

### Technical tasks

- `frihart-css`: tokenizer, parser, stylesheet OM
- `frihart-style`: cascade, inheritance, specified → computed
- `frihart-layout`: box generation, normal flow, inline, block
- Flexbox after flow is trustworthy
- Grid later, not in the first layout year
- Text with `cosmic-text`: wrapping, bidi later
- Replaced content: PNG, JPEG, then WebP; SVG is its own project
- `frihart-gfx`: display list, software paint first, GPU when measured
- Stylesheet origin: user agent, user, author
- User stylesheet file in the profile (`user.css`)
- Incremental layout later; correctness first

### Architectural decisions

- Subset is documented (`docs/css-subset.md` when the crate is born)
- Unimplemented properties are ignored, never guessed into a broken layout
- Software paint remains correct even after a GPU path exists
- Color, writing-mode, and fonts are designed for fingerprint resistance
  (engine fonts, not system enumeration)

### First CSS subset (must)

- Selectors: type, class, id, descendant, child, `:root`, `:link`, `:visited` (partitioned)
- Properties: `display`, `margin`, `padding`, `border`, `width`, `height`,
  `color`, `background-color`, `font-size`, `font-weight`, `font-family`
  (mapped to engine fonts), `text-align`, `line-height`, `list-style`,
  `max-width`, `white-space`
- Units: `px`, `em`, `rem`, `%`, `vh`/`vw` (careful with fingerprinting)

### Success criteria

- A fixture suite of static articles paints recognizably
- Images from the same origin display
- User `user.css` applies
- No font or canvas API leaks the system inventory
- Layout of the subset is deterministic in tests

---

## Phase 5 — First daily-driver loop

**When:** months 16–22
**Goal:** Frihart is the browser you use for a *named* list of simple sites.

### Technical tasks

- Forms: input, textarea, checkbox, radio, select, submit (no JS)
- GET/POST forms, `application/x-www-form-urlencoded`
- Cookies on form POST, same-site rules
- Basic auth only if we can do it without a password manager phone-home
  (we will not)
- Bookmarks from the toolbar
- Find in page for HTML
- Readable error pages, certificate pages, HTTPS-only interstitial
- Reader-like UA stylesheet that makes unstyled documents pleasant
- Performance: first paint under a second for a 100 KB article on modest hardware
- A public "sites we claim" list, updated honestly

### Architectural decisions

- Compatibility is claimed per site, not "the web"
- A broken layout is a bug; a missing JS app is expected
- No site-specific hacks in the engine. If example.com needs a hack,
  the subset is wrong or the site needs JS we do not have.

### Success criteria

- 20+ named static or mostly-static sites work without JS
- Forms can search a user-configured search URL
- You can read documentation, blogs, and news text daily
- The "sites we claim" list is tested in CI where licenses allow

---

## Phase 6 — Process isolation and sandbox

**When:** months 18–24
**Goal:** a hostile page cannot read the profile or own the OS user.

### Technical tasks

- Create `frihart-ipc` with a small typed protocol
- Split network into its own process
- Split content: one process per site isolation key
- Chrome process is the only process that touches the full profile
- Linux sandbox: `seccomp-bpf`, landlock, no_new_privs, optional user
  namespace + chroot/pivot
- Crash of a content process reloads that tab, not the browser
- Resource limits (fds, memory) per content process
- Audit: content process cannot open a raw socket
- Fuzz IPC and the HTML parser

### Architectural decisions

- Site isolation is scheme + registrable domain
- No shared memory of DOM across sites
- GPU later: if a GPU process exists, it is not trusted with cookies
- Sandbox is default-on for content. A pref may weaken it; it may not
  disappear from the product.

### Success criteria

- Killing a content process does not lose other tabs' documents
- A test content process cannot read `prefs.toml`
- A test content process cannot connect to an address the network
  process did not proxy
- `about:processes` shows the tree

---

## Phase 7 — Scripting, carefully

**When:** year 2–3
**Goal:** enough script to make *some* real sites work, without becoming
a generic exploit runtime.

### Technical tasks

- Decision document: subset interpreter vs. embedding a JS engine
  (the product remains Frihart; an embedded engine is a component, not a
  fork of a browser)
- WebExtensions host: execute sideloaded Firefox add-ons against the
  subset of `browser.*` we have implemented (see docs/extensions.md)
- Start with no JIT if we embed; JIT is an opt-in later
- Bindings: DOM read, simple mutation, events (`click`, `submit`,
  `DOMContentLoaded`)
- `fetch` / `XMLHttpRequest` go through `frihart-net` + policy
- Timers, `console` (local only)
- Per-origin JS permission: default deny or default allow *per user pref*,
  shipped default remains conservative
- No `eval` until we have a reason and a sandbox story
- No WebAssembly in the first scripting year
- Fingerprint APIs (`canvas.toDataURL`, `WebGL`, `AudioContext`) stay
  denied or farbling

### Architectural decisions

- Script is last because it is the largest attack surface
- Chrome is not scriptable by web content
- Extensions, if they ever exist, are not web-origin JS with full chrome
  access
- We would rather refuse a site than add a rotten API

### Success criteria

- A documented list of progressive-enhancement sites work with JS on
- Hostile tests cannot read cookies of another origin
- Hostile tests cannot reach `file://` or the profile
- JS can be flipped off globally and per site, immediately

---

## Campaign F — Linux homes (before any other OS)

**When:** overlaps late Phase 5–6. **Before Phase 8.**
**Goal:** the binary is excellent on the machines we actually love.

### Technical tasks

- Arch / CachyOS: AUR package, reproducible `PKGBUILD`
- Fedora: COPR or a clean `.spec` people can rebuild
- Linux Mint / Debian: `.deb` that installs `/usr/bin/frihart` only
- Tails: ephemeral default; use Tails Tor at `127.0.0.1:9050`; write
  only to Persistent if the user said so; honest "not Tor Browser"
- Qubes: Fedora + Debian template packages; DisposableVM = private
  profile; Whonix SOCKS fail-closed; no qrexec holes
- Desktop file, man page, no extra network on first launch
- Document each home in [docs/distros.md](docs/distros.md)

### Success criteria

- `makepkg` / `rpmbuild` / `dpkg-buildpackage` produce a silent binary
- Tails notes are testable on a stick, not theoretical
- A Qubes DisposableVM leaves nothing on the template
- Mint and Fedora users get the same chrome as Arch

---

## Phase 8 — Windows

**When:** after Linux (including Tails and Qubes notes) is a daily driver
**Estimate:** 2–4 months for chrome + platform, longer for sandbox parity

### Technical tasks

- `frihart-platform` Win32/Win64 implementation
- Windowing through winit's Windows backend
- Profile under `%LOCALAPPDATA%\Frihart`
- Code signing story the user can verify (not a store lock-in)
- Sandbox: Job objects, restricted tokens — honest about the gap vs. Linux
- Installer: `msiexec` or a simple signed archive. No bundleware.

### Success criteria

- Same fixtures render
- Same prefs file format
- No extra network on startup
- Update path does not install other software

---

## Phase 9 — macOS

**When:** after Windows is usable
**Estimate:** 2–3 months for chrome + platform

### Technical tasks

- App bundle, notarization *only if* we can do it without Apple-only
  distribution becoming a choke point
- Profile under `~/Library/Application Support/Frihart`
- Sandbox: Seatbelt profiles
- Retina chrome already required by Linux HiDPI work

### Success criteria

- Feature parity with the Windows build of that month
- Gatekeeper is documented; sideloading remains possible

---

## Phase 10 — Android

**When:** after desktop is real
**Estimate:** 4–8 months for a first cut

### Technical tasks

- Reuse engine crates; do not reuse desktop chrome
- Thin Android chrome (Rust + a small JNI layer, or a Rust activity)
- Profiles in app-private storage
- No Play-services dependency
- F-Droid packaging is the goal, not a Play-only listing
- Mobile navigation: one tab strip model that does not ape Chrome

### Success criteria

- Same engine fixtures
- No Google libraries in the dependency tree
- Background network is user-visible and stoppable

---

## Phase 11 — Maturity

**When:** ongoing after Phase 5
**Goal:** depth, not novelty.

### Technical tasks (ordered by need, not excitement)

- Flexbox completeness, then grid
- Tables that match real documents
- SVG (its own mini-engine)
- Audio/video with system decoders, autoplay off
- Print to PDF
- User styles and later a *local* extension model (no remote gallery)
- Translations of chrome
- Performance: parallel style/layout, better cache
- Full bidi, CJK, complex scripts (Linux font stack we already have)
- Automated compatibility dashboard against *our* fixture corpus

### Success criteria

- The "sites we claim" list grows every quarter
- Security advisories have a process (see SECURITY.md)
- Distro packages exist for Arch and at least one other Linux

---

## Phase 12 — Engine pipeline

**Goal:** one function from HTML bytes to a display list.

Crate: `frihart-pipeline`. Tests exist. Chrome still paints its own way until layout is trusted.

---

## Phase 13 — Chrome language

**Goal:** every chrome string goes through `frihart-i18n`. Default English.

---

## Phase 14 — Print

**Goal:** display list → PDF/PS. Crate: `frihart-print`. Local only.

---

## Phase 15 — Extension runtime

**Goal:** execute sideloaded WebExtensions against the `browser.*` subset we have. Host crate already installs and audits. Runtime waits on Phase 6–7.

---

## What we will not schedule

- "Electron mode" or embedding Chromium to look finished
- A rewrite in another language
- A token, a foundation coin, a DAO
- Telemetry "just for the launch"
- Default-search revenue as a milestone
- Feature parity with Chrome as a Phase N exit criterion

Chrome parity is not a destination on this map. A sovereign, understandable
browser that renders an expanding, honest subset of the web is.

## Suggested working cadence

A sustainable rhythm for a long project:

- **One crate-visible improvement per week** (a pref, a parser rule, a
  chrome fix), not a monthly hero branch
- **Policy tests never red**
- **Friday is delete-code day** when the design got ahead of the tests
- **Engine work in the morning, chrome polish when tired** — chrome bugs
  are user-visible and tempting; the engine is the long pole
- **Tag 0.1.0** when Phase 1 success criteria are met, not before

## Near-term checklist (the only items that matter this month)

- [x] Repository, license, philosophy, architecture, roadmap
- [x] Workspace and Phase 0/1 crates
- [x] Linux window and `about:` chrome
- [x] Black / yellow dark chrome
- [x] Native containers + about:containers
- [x] Native blocker seed + about:blocker
- [x] Translator prefs + about:translate
- [x] Find in page, bookmark shortcut, PKGBUILD
- [x] HiDPI scale (winit scale_factor → Metrics)
- [ ] Multi-window (WindowId exists; one window still)
- [x] Bookmarks / history as about: pages
- [x] rustls fetch + view-source (Phase 2)
- [x] First-party partitioned cookies, 0600 profile files
- [x] Tor tabs refuse clearnet (no silent fallback)
- [x] HTML subset + arena DOM + CSS/layout/gfx pipeline
- [x] Identity autofill (no password store)
- [x] Lists, pre/code, quote, br, img boxes
- [x] class/id/descendant CSS, max-width, line-height, user.css
- [x] cosmic-text wrap + display-list link hits
- [x] Tor SOCKS5 (fail closed) + Downloads 0600 never execute
- [ ] Chrome paints only the pipeline (fields still overlay)
- [ ] Distro notes for Tails (amnesic) and Qubes (DisposableVM)
- [ ] Fedora + Mint packages besides the Arch PKGBUILD
