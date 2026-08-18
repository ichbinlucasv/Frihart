# Frihart on Qubes OS

Qubes is compartments. Frihart is an app in an AppVM.

- Package for **Fedora** and **Debian** templates (`packaging/fedora`,
  `packaging/debian`).
- Detection: `/usr/share/qubes` or `/etc/qubes-rpc`.
- A DisposableVM (`/run/qubes/this-is-dvm` or `QUBES_DVM`) runs as a
  private profile (memory only), same as Tails.
- There is no physical NIC. Traffic goes through `sys-net` /
  `sys-firewall` / `sys-whonix` as the user set.
- Whonix: use that workstation's SOCKS. Fail closed if it is down.
- Do not add qrexec services in v1. Do not punch holes.

This is campaign F. Template packages are not published yet.
