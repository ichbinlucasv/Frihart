# Frihart philosophy

Frihart is a **libertarian** web browser: your machine is yours. The
project is original software. It is not a fork of Firefox, LibreWolf,
Chromium, or any other browser. **LibreWolf is the inspiration** — its
stance, not its code. Black chrome, yellow accents. Rust, so memory
unsafety is not the default disaster.

This file is the constitution. Feature work that contradicts it is out of
scope, even if it would make the browser more popular.

## Inspiration

LibreWolf is a Firefox fork that strips telemetry, tightens
fingerprinting resistance, and refuses Mozilla's commercial defaults.
That is the product ethic Frihart exists to honour.

Frihart is not LibreWolf and does not vendor Gecko. We take the ethic
further, as native features rather than add-ons:

- Identity **containers** are first-class tabs, not a Multi-Account
  Containers extension.
- A **uBlock-class blocker** is built into the browser. We do not ship
  the uBlock Origin WebExtension, because we are not Firefox. We ship
  the same job: on by default, lists the user can replace, no store.
- A **translator** lives in chrome. DeepL is the product default.
  Google is not offered. The API key stays in the profile.
- **Search** is Swisscows first, DuckDuckGo second, then other
  privacy-focused engines. No Google. No Bing. No search-deal money.
- **Dark mode** is the product look: black chrome, yellow accents.
- **Tor tabs** use the system Tor daemon. We do not bundle the network.
- **VPN** hooks official ProtonVPN and Mullvad CLIs. We do not ship a
  VPN protocol.
- **Firefox add-ons** can be *compatible* without a Gecko fork: we
  implement WebExtensions ourselves. Installed `.xpi` files are dormant
  until we have JS. We do not pretend they run.

If LibreWolf is "Firefox with the bad parts cut out," Frihart is a
browser that never had those parts.

## Why not a Firefox or Chrome branch

Forking Gecko or Blink inherits their problems: a C/C++ memory-unsafe
core, a decade of RCEs, an IPC and GPU surface the size of an operating
system, telemetry DNA, and a release train you do not control. "We
patched this week's use-after-free" is not OPSEC. It is someone else's
incident response.

Frihart's bet is smaller and slower:

- **Rust** for the product. Memory corruption is not an acceptable
  weekly event. Rust does not make logic bugs vanish. It removes a class
  of remote code execution that Firefox and Chrome still ship.
- **A smaller surface.** We refuse APIs, JS, WebRTC, and GPU toys until
  they can be isolated. A missing feature is better than a leak.
- **No upstream gravity.** Mozilla and Google can change defaults,
  contracts, and "safety" features overnight. A fork spends its life
  rebasing. We own the tree.
- **Fail closed.** Tor never falls back to clearnet. Downloads never
  execute. Passwords are never stored. A content crash must not own the
  profile (Phase 6).

We will not be "better than Firefox" on day one. We will be **harder to
own** than a mega-browser, on purpose, for people who treat a leak as a
failure.

## Libertarian contract

The user is the principal. Frihart is an agent. The browser does not have
interests of its own: no growth target, no paid search contract, no
"anonymous product insights," no account, no store.

- You own the binary (`MIT OR Apache-2.0`).
- The profile lives on disk you control, mode `0600` / `0700`.
- Defaults protect you. Power remains available.
- Nothing leaves the machine unless you initiated it.
- There is no Frihart account and there never will be.
- Logins and passwords are never stored. `store_logins` cannot be turned on.
- Cloud sync, if it ever exists, will be something you host.
- Distros update the package. We do not phone home to "check."

User sovereignty includes the freedom to weaken protection locally.
What you cannot do is weaken *someone else's* defaults by lobbying for a
shipped-on tracker, a sponsored tile, or a silent partner exception.

The browser takes sides against surveillance. It does not take sides
against your own judgment.

## Privacy is the default, not a product tier

A privacy feature that is off by default is not a privacy feature. It is
marketing.

Hard defaults:

- No telemetry of any kind. Not crash reports. Not "anonymous" usage
  pings. Not update-check beacons that report more than a version number
  the user asked for. If we cannot update without phoning home, we do not
  auto-update; the user or their distro updates the package.
- No third-party cookies.
- First-party state is partitioned.
- Referrers are stripped to the origin, and cross-site referrers are
  omitted.
- HTTPS-only mode is on.
- Fingerprinting resistance is on.
- Third-party storage, cache, and identifiers are denied.
- WebRTC, until it exists and is reviewed, is off.
- JavaScript, until a real engine exists, is off. When it exists, it will
  still be a user-controlled permission, not an implicit yes to the whole
  web.
- Do Not Track is not sent. DNT is a tracking bit. Global Privacy Control
  may be sent because it has a legal meaning some jurisdictions honor.

## Anti-tracking is policy, not a list subscription

Blocklists are useful and we will ship local ones the user can replace.
They are not the architecture. The architecture is:

1. The network stack does not volunteer identifying information.
2. The cookie and storage jars do not leak across sites.
3. The content engine does not expose cheap fingerprint surfaces.
4. The chrome does not add its own identifiers.

A tracker that is not on a list should still fail to identify the user.

## Fingerprinting resistance

The goal is not "look like Chrome." That strategy makes the user a member
of a crowd they do not control, and it collapses as soon as Chrome
changes. The goal is:

- Reduce the number of high-entropy APIs.
- Clamp or ignore the rest to a small set of buckets.
- Prefer refusing an API over lying in a way that sites can detect and
  escalate.

We will document every surface we expose. If we cannot explain a surface,
it does not ship.

## OPSEC is standing work

See [docs/opsec.md](docs/opsec.md). Short version:

- Policy before I/O.
- No login vault. External managers only (Proton Pass, KeePassXC, …).
- Wipe / reset / shred are first-class and this-profile-only.
- Tor tabs fail closed.
- Downloads are `0600` and never execute.
- Isolation keys (scheme + host + container) from day one.
- Process split and Linux sandbox before we grow JS.

## Original code, borrowed primitives

"100% original" means Frihart is not a reskin of another browser. It has
its own process model, chrome, preferences, privacy policy engine, and
(over time) its own document engine.

It does **not** mean reimplementing TLS, Unicode, or font rasterization.
Those are solved problems. Using `rustls`, system fonts, and similar
libraries is correct engineering. Using Gecko, Blink, WebKit, or Servo as
the product is not.

Every third-party crate must be justified. A crate that phones home, pulls
in a telemetry SDK, or makes the supply chain absurd is rejected.

## Linux first — the homes we love

The primary platform is Linux. That is not a slogan. The same binary
should be excellent on the systems this project is built for:

| Home | Why it matters |
| --- | --- |
| **Arch** / **CachyOS** | Reference. Rolling, user-controlled, PKGBUILD first. |
| **Fedora** | RPM family. Qubes templates often start here. |
| **Linux Mint** | Debian/Ubuntu family. The machine people actually give relatives. |
| **Tails** | Amnesic. Tor is the network. Persistence is opt-in. |
| **Qubes OS** | Compartments. Disposable VMs. Network is never "the NIC." |
| **Every other Linux** | Same binary. Wayland first, X11 while it lasts. |

Windows, then macOS, then Android come **after** Linux is a daily driver
on the homes above — including Tails and Qubes — not before.

We do not delay Linux quality to keep hypothetical Windows code paths
warm.

## Honesty about scope

A competitive, general-purpose web engine is a decade-scale effort even
for a well-funded team. Frihart will not pretend otherwise.

The path is a **capability ladder**:

1. A real browser chrome the user can live in.
2. A network stack that already behaves like a privacy browser.
3. Documents we own (`about:` pages, then HTML).
4. Incremental CSS, layout, and paint.
5. A useful reader for simple sites.
6. Isolation and sandboxing.
7. Linux packaging for the homes we love.
8. Scripting, last, because it is the largest attack surface.

Until a rung works, we do not advertise the next one. A page that cannot
be rendered correctly should fail clearly, not half-work in a way that
trains users to distrust the engine.

## Clean defaults, quiet software

Frihart should feel finished when it is empty: no onboarding tour, no
import-your-soul wizard, no "choose your search empire" interstitial.
Open the window. Type a URL. Change a pref if you want. Close it.

The look is **black and yellow**. That is the product, not a theme store.

Logs go to stderr or a local file the user chose. They never go to us.

## What we will not build

- Telemetry, crash-report uploaders, "improvement programs"
- User accounts, sync services we host, or identity providers
- Sponsored tiles, default-search deals, or partner exceptions
- A built-in crypto wallet, NFT feature, or token gate
- Remote "recommended extensions"
- A content filter sold as morality while phone-home stays on
- DRM modules
- Any feature whose correct operation requires Frihart servers

If a future contributor needs one of these, they can fork. The trunk
stays sovereign.
