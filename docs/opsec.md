# OPSEC

Standing rules. They do not wait for a phase number.

LibreWolf's stance is the ethic. Frihart's job is to make that ethic
**hard to accidentally break**.

## Fail closed

| Situation | What happens |
| --- | --- |
| Tor tab, no SOCKS / dead daemon | Error. No clearnet. |
| HTTP while HTTPS-only | Interstitial. No fetch. |
| Download | Written `0600`. Never `+x`. Never executed. |
| Password field | Not stored. External manager only. |
| Unknown scheme | Refused. |
| JS / WebRTC / canvas / WebGL | Off until reviewed. Default stays conservative. |

## Disk

- Profile files `0600`, directories `0700`.
- No world-readable cookies or history.
- Wipe = this session like new; bookmarks stay.
- Reset = prefs default; bookmarks stay.
- Shred = this profile only; other named profiles stay.
- Private window = memory only.

## Network

- Zero connections on startup.
- rustls only. No system CA surprises as the product default.
- Frozen User-Agent. No Client Hints.
- No `Referer` path. No DNT bit. GPC may be sent.
- First-party cookies, partitioned by isolation key (scheme + host +
  container).
- Blocker on at install. Lists are local. No Frihart list server.

## Memory and process

Rust removes a class of RCEs that still ship in Gecko and Blink. It
does not remove logic bugs.

Target (Phase 6):

- Chrome is the only process that touches the full profile.
- One content process per isolation key.
- Network process has no DOM.
- Linux sandbox: seccomp-bpf, landlock, `no_new_privs`.
- A tab crash reloads that tab, not the browser.

Until then, crate seams are written as if IPC already existed.

## What we will not do "for security"

- Phone home to refresh a blocklist.
- Force a DoH vendor.
- Ship DRM or any closed module.
- Hide prefs that weaken protection. Document them instead.
- Impersonate Chrome's fingerprint to "blend in."

## Tails and Qubes

See [distros.md](distros.md). Short version: on Tails we must not fight
amnesia; on Qubes we must not assume we can see the wire.
