# Frihart on Tails

Tails is amnesic. Tor is already the network.

- Install the Debian package into **Persistent** only if the user asked.
- Default profile must not survive reboot. Use `--private` or a ramdisk
  path. Do not write under `/home/amnesia` unless persistence is on.
- SOCKS is `127.0.0.1:9050` (Tails' Tor). Never start a second daemon.
- If SOCKS is down, fail. Never clearnet.

This is campaign F. The `.deb` is `packaging/debian/`.
