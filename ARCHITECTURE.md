# Frihart architecture

This document is the map of the codebase. It describes the modular design
we will follow from Phase 0 through a mature multi-platform browser. It is
normative: new code belongs in an existing crate or in a crate this
document introduces.

Companion documents:

- [PHILOSOPHY.md](PHILOSOPHY.md) — why the product exists (libertarian, LibreWolf stance)
- [ROADMAP.md](ROADMAP.md) — campaigns A–I and crate phases 0–15
- [docs/opsec.md](docs/opsec.md) — fail closed, disk, Tor, process
- [docs/distros.md](docs/distros.md) — Arch, Cachy, Fedora, Mint, Tails, Qubes
- [docs/defaults.md](docs/defaults.md) — every shipped default and why

## Design thesis

A browser is not a rendering engine with a toolbar glued on. It is several
programs that share a profile:

| Layer | Job | Ships when |
| --- | --- | --- |
| **Chrome** | Windows, tabs, URL bar, settings, find, bookmarks | Phase 1 |
| **Profile** | Prefs, bookmarks, history, permissions, state on disk | Phase 1 |
| **Privacy policy** | The rules every other layer must ask before acting | Phase 1 |
| **Network** | DNS, TLS, HTTP, cookies, cache, downloads | Phase 2 |
| **Content** | Documents, navigation, session history | Phase 1 (internal), Phase 3 (HTML) |
| **Engine** | HTML, CSS, DOM, style, layout, paint | Phases 3–5 |
| **Script** | JS runtime and Web APIs | Phase 7 |
| **Isolation** | Process model, IPC, sandbox | Phase 6 |
| **Platform** | OS windows, paths, fonts, sandbox primitives | Phase 1 Linux, later ports |

The important inversion: **privacy policy is not a feature of the network
stack.** It is a crate every stack consults. Chrome, net, content, and
script all receive a `Policy` and must be able to explain a denial.

## Non-negotiable constraints

1. **Original product.** Frihart is not Gecko, Blink, WebKit, or Servo
   with a different chrome. Engine crates are written here.
2. **No telemetry pathway.** There is no `frihart-telemetry` crate, no
   analytics URL constant, and no "phone home later" hook.
3. **Policy before I/O.** A network request, storage write, permission
   grant, or fingerprintable API call is illegal unless `frihart-privacy`
   allowed it.
4. **Linux is the reference.** Arch/CachyOS first, then Fedora, Mint,
   Tails, Qubes. Other OS wait their turn.
5. **Capability ladder.** We do not stub a full HTML5 engine. Each crate
   grows behind tests and a documented subset.
6. **Small, named crates.** A crate that needs to know both pixel layout
   *and* TLS is in the wrong place.

## Process model (target)

```
+------------------+          IPC           +----------------------+
|  chrome process  | <-------------------> |  network process     |
|  frihart-chrome  |                       |  frihart-net         |
|  profile, prefs  |                       |  cookies, cache      |
|  windows, tabs   |                       +----------------------+
+--------+---------+
         |
         | one content process per site isolation key
         v
+------------------+
|  content process |
|  frihart-content |
|  engine crates   |
|  (later: JS)     |
+------------------+
         ^
         | OS sandbox (seccomp, landlock, namespaces)
         +-- frihart-platform
```

Phase 1 is a **single process**. The crate boundaries above still exist
in-process so Phase 6 can split them without rewriting product logic.
Treat in-process calls between chrome and content as if they were IPC:
no reaching into another crate's private state.

Site isolation key (Phase 6): registrable domain + scheme. `https://a.com`
and `https://b.com` never share a content process. `https://a.com` and
`http://a.com` do not either.

## Workspace layout

```
Frihart/
├── crates/
│   ├── frihart/              # binary: CLI, logging, process entry
│   ├── frihart-core/         # IDs, errors, version, URL helpers
│   ├── frihart-config/       # typed prefs, privacy-first defaults
│   ├── frihart-profile/      # on-disk profile, lock, bookmarks, history
│   ├── frihart-privacy/      # policy engine (the constitution, as code)
│   ├── frihart-blocker/      # native uBlock-class host blocker
│   ├── frihart-search/       # Swisscows / DDG / privacy search catalog
│   ├── frihart-extensions/   # WebExtensions host (parse now, run later)
│   ├── frihart-html/         # HTML subset tokenizer + tree
│   ├── frihart-dom/          # arena Document + NodeId
│   ├── frihart-css/          # declarations + rules
│   ├── frihart-style/        # UA + author cascade
│   ├── frihart-layout/       # block flow
│   ├── frihart-gfx/          # display list
│   ├── frihart-pipeline/     # html → display list
│   ├── frihart-forms/        # GET/POST encode
│   ├── frihart-ipc/          # typed process messages
│   ├── frihart-js/           # values; exec off
│   ├── frihart-media/        # sniff only
│   ├── frihart-i18n/         # chrome strings
│   ├── frihart-print/        # display list → PS
│   ├── frihart-autofill/     # identity fill, never passwords
│   ├── frihart-net/          # HTTP(S), DNS, TLS, downloads
│   ├── frihart-platform/     # OS paths, windowing hooks, sandbox stubs
│   ├── frihart-content/      # documents, about: pages, navigation
│   └── frihart-chrome/       # Linux UI: tabs, toolbar, viewport
├── docs/                     # design notes that are not the constitution
├── ARCHITECTURE.md
├── ROADMAP.md
├── PHILOSOPHY.md
└── Cargo.toml                # workspace
```

### Engine crates (spine)

Each crate has types and tests. Depth is still the long job.

| Crate | Phase | Status |
| --- | --- | --- |
| `frihart-html` | 3 | tokenizer, fragments (lists/pre/img) |
| `frihart-dom` | 3 | arena Document + NodeId |
| `frihart-css` | 4 | class/id/descendant selectors |
| `frihart-style` | 4 | UA + user + author, max-width, line-height |
| `frihart-layout` | 4 | block flow + cosmic-text wrap |
| `frihart-gfx` | 4 | Fill + Text + link hit-test |
| `frihart-pipeline` | 12 | html → display list (chrome paints it) |
| `frihart-forms` | 5 | GET URL + POST body |
| `frihart-ipc` | 6 | envelopes + in-process bus |
| `frihart-js` | 7 | values; exec and fingerprint APIs off |
| `frihart-media` | 11 | sniff; autoplay off |
| `frihart-i18n` | 13 | chrome catalog, en default |
| `frihart-print` | 14 | display list → local PostScript |

## Crate contracts

### `frihart-core`

The only crate everyone may depend on freely.

Contains:

- `APP_NAME`, `APP_ID`, version
- `FrihartError` and `Result`
- `TabId`, `WindowId`, `DocumentId`
- URL classification (`about`, `https`, `http`, `file`, unknown)
- Navigation intent types

Contains **not**: prefs, pixels, sockets, HTML.

### `frihart-config`

Typed `Prefs` with serde/TOML. Unknown keys are preserved when possible
and never cause a silent reset of the whole file.

Defaults live in one function. Changing a default is a documented
decision in `docs/defaults.md`.

No crate writes prefs except through this API.

### `frihart-profile`

Owns the XDG profile directory:

```
$XDG_DATA_HOME/frihart/profiles/<name>/
    prefs.toml
    bookmarks.toml
    history.jsonl
    permissions.toml      # later
    cookies/              # later, net
    cache/                # later, net
    lock
```

Private windows use an in-memory profile that never creates those files.

A pid lock prevents two chrome processes from writing the same profile.
Readers of a locked profile (debugging tools) are allowed; writers are
not.

### `frihart-privacy`

Pure policy. No sockets. No filesystem. No window handles.

```
Policy::decide(RequestContext) -> Decision { Allow, Deny { reason }, Modify { .. } }
```

Request contexts include:

- outbound HTTP
- cookie read/write
- referrer
- storage
- permission (geolocation, notifications, …)
- fingerprint surface (canvas, fonts, WebGL, audio, …)

Chrome displays `Decision` reasons. Content and net obey them.

### `frihart-blocker`

Native filter engine. Inspired by uBlock Origin, not a port of it and
not a WebExtension. Phase 1 is host matching plus a built-in tracker
seed, on by default. Phase 2 ingests EasyList / EasyPrivacy / uBlock
filter files from disk. No list updates from Frihart servers.

The network stack asks this crate *and* `frihart-privacy` before a
request leaves the machine.

### `frihart-net`

Talks to the network and to `frihart-privacy`. Never to chrome widgets.

Phase 1: types and a client trait.
Phase 2: rustls + HTTP/1.1, then HTTP/2; DNS via the system resolver
first, then optional DNS-over-HTTPS **chosen by the user**.
No default DoH provider that we have a relationship with.

Cookie jar, cache, and HSTS store live here and ask privacy before every
read or write.

### `frihart-platform`

OS seams:

- config/data/cache directories
- window creation parameters
- later: seccomp-bpf, landlock, user namespaces, pledge-like helpers

Linux implementation is real. Other OSes return `Unsupported` until their
phase.

### `frihart-content`

A `Document` is one of:

- `Blank`
- `Internal(InternalPage)` — structured `about:` pages, no HTML required
- later: `Html(...)`
- later: `Failure { url, kind, message }`

Navigation:

```
NavigationController
    -> classify URL
    -> load document
    -> push session history
```

`about:` URLs never touch the network. That is enforced here, not by
convention.

Internal pages are data (title, blocks, toggles, links). Chrome renders
them. This keeps product copy and UI hit-testing out of a future HTML
engine.

### `frihart-chrome`

Owns the event loop, the pixels of the chrome, and hit-testing.

It does not parse HTML. It does not speak TLS. It asks content for the
active document and paints a viewport.

Phase 1 drawing is software (`winit` + `softbuffer` + `cosmic-text`).
That is deliberate. A custom GPU compositor is a Phase 4/5 problem. The
chrome widget set is ours so we do not inherit another toolkit's
accessibility and theming model before we have time to own it.

### `frihart` (binary)

- Parses CLI (`URL`, `--profile`, `--private`, `--version`)
- Installs a stderr logger
- Opens the profile
- Hands control to `frihart-chrome`

In Phase 6 this binary becomes the process supervisor: it will spawn
chrome, network, and content processes from the same executable with a
hidden `--mode=` flag. Design CLI and logging so that split is possible.

## Data flow for a navigation (Phase 1)

```
user types in URL bar
    -> chrome commits the string
    -> frihart-core parses / classifies
    -> if http(s): content produces an honest "not yet" internal page
    -> if about:: content builds InternalPage from prefs + policy
    -> chrome paints chrome + viewport
    -> if a toggle changes: config writes prefs, content reloads the page
```

Phase 2 inserts `frihart-net` after classification and before document
construction. Phase 3 replaces the failure page with an HTML document
when the response is `text/html`.

## UI architecture

Chrome layout, in pixels from the top of the window:

```
+------------------------------------------------------+
| tab strip                                            |
+------------------------------------------------------+
| back  forward  reload  | url bar | privacy indicator |
+------------------------------------------------------+
|                                                      |
|                  content viewport                    |
|                                                      |
+------------------------------------------------------+
| status                                               |
+------------------------------------------------------+
```

All chrome metrics live in `frihart-chrome::theme`. Do not scatter magic
numbers.

Input is handled in this order:

1. URL bar, if focused (text editing)
2. Chrome widgets (tabs, buttons)
3. Content hit-test (internal links and toggles; later, the engine)

Keyboard shortcuts are listed on `about:keyboard` and implemented in one
match. Do not bind keys in three places.

## Privacy architecture

Every default is a `Prefs` field with a documented rationale. The policy
engine reads prefs and produces decisions. Content and net never read
"should I track?" from a second source.

Fingerprint resistance is a set of named surfaces, each with a strategy:

| Surface | Default strategy |
| --- | --- |
| User-Agent | Frozen, generic Frihart token. No OS micro-version. |
| Language | User pref; single language by default |
| Timezone | UTC unless the user opts into system zone |
| Screen | Chrome window size only; no extra displays |
| Canvas | Disabled until we have a paint API; then farbling or deny |
| Fonts | Engine font list, not the system inventory |
| WebGL / GPU | Denied until reviewed |
| Audio | Denied until reviewed |
| WebRTC | Denied |
| Client hints | Never sent |
| DNT header | Never sent |
| GPC header | Sent |

When a strategy cannot be implemented yet, the surface is **denied**, not
left raw.

## Rendering architecture (future)

Do not start a "real browser engine" as one crate. The pipeline is:

```
bytes
  -> frihart-html     tokenizer / tree builder
  -> frihart-dom      document
  -> frihart-css      stylesheets
  -> frihart-style    computed style per element
  -> frihart-layout   fragments
  -> frihart-gfx      display list -> pixels
```

Each arrow is a testable function. A page that uses an unimplemented CSS
feature must still produce a layout, with the unimplemented rule dropped
and (in a debug pref) logged.

JavaScript is not in this pipeline until Phase 7. A site that requires JS
shows a clear permission / capability page rather than a blank viewport.

## Platform roadmap in the architecture

`frihart-platform` exposes traits. Implementations:

1. **Linux** (now) — Wayland + X11 via winit, XDG dirs, later landlock/seccomp
2. **Windows** — after Linux is a daily driver for simple sites
3. **macOS** — after Windows
4. **Android** — embedding the same engine; chrome will be a different crate
   or a radically thinner one

Do not `#ifdef` Windows into Linux modules. New files, new impl blocks.

## Testing doctrine

| Kind | Where | Purpose |
| --- | --- | --- |
| Unit | each crate | prefs, policy, history, URL classify |
| Snapshot | content / later engine | `about:` pages, then HTML/CSS fixtures |
| Integration | `frihart` / chrome (headless later) | navigation, prefs persist |
| Manual | Linux desktop | windowing, input, visual chrome |

Policy tests are load-bearing. A default that regresses is a release
blocker.

There is no test that "phones home to verify connectivity." Network tests
use fixtures and local listeners.

## Dependency doctrine

Allowed without debate: Rust std ecosystem staples (`serde`, `thiserror`,
`url`), `rustls`, `winit`, `softbuffer`, `cosmic-text`, OS bindings.

Requires a written justification in the PR:

- Anything that pulls a TLS stack other than rustls
- Anything that executes JS
- Anything with its own HTTP client defaults
- Huge UI frameworks that would own our chrome

Forbidden:

- Crash reporters that upload
- Analytics SDKs
- "Update" libraries that report hardware or locale
- Prebuilt engine blobs (no shipping Chromium Embedded Framework)

## Versioning

`0.x` until the first Linux release that can fetch an HTML document and
paint a useful subset. Crate versions stay aligned in the workspace.
Breaking crate APIs is allowed in 0.x; breaking on-disk profile formats
requires a migrator.

## How to add a feature

1. Name the crate that owns it. If none does, propose the crate here
   first.
2. Add the pref, if any, with a default and a `docs/defaults.md` entry.
3. Put the privilege check in `frihart-privacy` if the feature can leak
   data or identify the user.
4. Implement the smallest slice that can be tested.
5. Expose it in chrome only after the tests exist.

If a feature needs all five steps, it is probably the right size. If it
needs a sixth crate "just this once," it is probably in the wrong shape.
