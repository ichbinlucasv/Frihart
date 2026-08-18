# Frihart on Tails

Tails is amnesic. Tor is already the network.

- Install the Debian package into **Persistent** only if the user asked.
- Default profile is **memory-only** (same as `--private`) unless you
  pass `--profile` at a Persistent path.
- SOCKS is `127.0.0.1:9050` (Tails' Tor). Never start a second daemon.
- If SOCKS is down, fail. Never clearnet.

This is campaign F. The `.deb` is `packaging/debian/`.
