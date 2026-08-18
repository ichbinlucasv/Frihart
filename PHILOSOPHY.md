# Frihart philosophy

Frihart is a web browser built for people who consider their computer
their own. The project is original software. It is not a fork of Firefox,
LibreWolf, Chromium, or any other browser. **LibreWolf is the
inspiration** — its stance, not its code. It will grow slowly, in public,
with boring engineering discipline.

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

If LibreWolf is "Firefox with the bad parts cut out," Frihart is a
browser that never had those parts.

This file is the constitution. Feature work that contradicts it is out of
scope, even if it would make the browser more popular.

## Sovereignty

The user is the principal. Frihart is an agent. The browser does not have
interests of its own: no growth target, no paid search contract, no
"anonymous product insights," no account, no store. Swisscows and
DuckDuckGo are privacy defaults, not a revenue deal.

- The profile lives on disk the user controls.
- Defaults protect the user. Power remains available.
- Nothing leaves the machine unless the user initiated it.
- There is no Frihart account and there never will be.
- Cloud sync, if it ever exists, will be something the user hosts.

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

## Libertarian, not nihilist

User sovereignty includes the freedom to weaken protection. A power user
who wants third-party cookies on one site, or JavaScript on one origin,
must be able to do that locally, permanently, without an account.

What they cannot do is weaken *someone else's* defaults by lobbying for a
shipped-on tracker, a sponsored new-tab tile, or a silent exception for a
partner.

The browser takes sides against surveillance. It does not take sides
against the user's own judgment.

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

## Linux first

The primary platform is Linux, especially Arch and CachyOS. That is not a
slogan. It means:

- The first usable browser is a native Linux application.
- Packaging for Arch (`PKGBUILD`, later an AUR package) is a first-class
  deliverable.
- Wayland is the primary windowing target; X11 is supported while it
  remains relevant.
- Platform abstractions exist from day one so Windows, then macOS, then
  Android can be added *later* without rewriting the product.

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
7. Scripting, last, because it is the largest attack surface.

Until a rung works, we do not advertise the next one. A page that cannot
be rendered correctly should fail clearly, not half-work in a way that
trains users to distrust the engine.

## Clean defaults, quiet software

Frihart should feel finished when it is empty: no onboarding tour, no
import-your-soul wizard, no "choose your search empire" interstitial.
Open the window. Type a URL. Change a pref if you want. Close it.

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
