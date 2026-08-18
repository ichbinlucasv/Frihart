# Frihart on Tails

Tails is amnesic. Tor is already the network.

- Install the Debian package into **Persistent** only if the user asked.
- Default profile is **memory-only** (same as `--private`) unless you
  pass `--profile` at a Persistent path. Detection: `ID=tails` in
  `/etc/os-release`.
- SOCKS is `127.0.0.1:9050` (Tails' Tor). Never start a second daemon.
- If SOCKS is down, fail. Never clearnet.
- Desktop file: “New Private Window” runs `frihart --private`. On Tails
  the default launch is already private.

This is campaign F. The `.deb` is `packaging/debian/`. A Tails-specific
package is not published yet.
