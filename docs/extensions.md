# Firefox extensions on Frihart — without a fork

**Short answer:** yes, compatibility is possible without forking Firefox.
It is not possible *today* to *run* uBlock Origin, Dark Reader, or
Bitwarden. We can already *install and audit* their `.xpi` files.

## What “compatible” means

A Firefox add-on is:

1. A ZIP (`.xpi`) with a `manifest.json`
2. JavaScript that calls `browser.*` / `chrome.*`
3. Optional HTML popups, options pages, and content scripts

LibreWolf runs those because it **is** Gecko. Frihart will not vendor
Gecko. Compatibility means **Frihart implements the same API** on its
own engine, the way Chrome, Firefox, and Edge all speak a related
WebExtensions dialect without being the same program.

That is original code. It is also a multi-year API surface.

## What works now (Phase 1)

```bash
frihart --install-addon ./ublock_origin.xpi
frihart about:addons
```

- Parse Manifest V2 / V3 (Firefox `browser_specific_settings.gecko.id`)
- Unpack `.xpi` or an unpacked folder into the profile
- Record permissions and mark the add-on **dormant**
- Classify each permission: already native in Frihart, planned, or refused

Nothing in the package is executed. There is no JS engine yet.

## What already exists natively (so you do not wait)

| Firefox add-on job | Frihart today |
| --- | --- |
| uBlock Origin / Disconnect / Privacy Badger | `frihart-blocker`, on by default |
| ClearURLs / Don't track me Google | strip `utm_*` / click ids |
| Consent-O-Matic | CMP hosts in the blocker seed |
| NoScript / PopUpOFF | JS off; `javascript:` refused |
| Multi-Account Containers | first-class containers |
| Dark Reader | black / yellow chrome |
| DeepL | `about:translate` |
| Swisscows | `about:search` |
| Proton Pass / Proton VPN | external CLIs, `about:pass` / `about:vpn` |
| Clear Cache / History Cleaner | wipe / reset / shred |
| Port Authority | private/loopback redirect refuse |
| TrafficLight | no cloud reputation; local blocker only |

When the WebExtensions host can run JS, those add-ons can *also* be
sideloaded. The native path stays. An add-on must not be required for
basic protection.

## Compatibility ladder

| When | What an add-on can do |
| --- | --- |
| Now | Be installed, listed, permission-audited |
| Phase 2 | `webRequest` / `proxy` map onto `frihart-net` + blocker |
| Phase 3–5 | Options / popup HTML |
| Phase 6 | Out-of-process, cannot read the profile |
| Phase 7 | Background scripts and content scripts actually run |

We will publish the subset we claim. “Works with every AMO add-on” is
not a milestone. “uBlock Origin’s network-blocking path works” is.

## What we will not do

- Fork Gecko or embed Firefox to fake compatibility
- Auto-download from addons.mozilla.org
- A remote “recommended extensions” feed
- Run privileged Firefox-only APIs (`debugger`, `geckoProfiler`, `mozillaAddons`)
- Give an add-on the chrome process

## Community

Write Frihart-native add-ons or help implement `browser.*` APIs. Codeberg
is the primary forge. Sideload from a git checkout:

```
frihart --install-addon ~/src/my-addon/
```

See [CONTRIBUTING.md](../CONTRIBUTING.md).
