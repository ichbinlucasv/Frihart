# Linux homes

Frihart is one Linux binary. Packaging differs. OPSEC differs on Tails
and Qubes. The engine does not.

## Reference: Arch and CachyOS

- `packaging/arch/PKGBUILD`
- Install system `tor` if you want Tor tabs: `pacman -S tor`
- Wayland first (Sway, Hyprland, KDE, GNOME). X11 still works.

CachyOS is treated as Arch. Same package, same paths.

## Fedora

- `packaging/fedora/frihart.spec`
- `dnf install tor` for Tor tabs
- Also the usual base for **Qubes** Fedora templates

## Linux Mint (and Debian / Ubuntu)

- `packaging/debian/`
- Mint is the Debian-family machine we care about most
- `apt install tor` for Tor tabs

## Tails (planned)

Tails is amnesic Debian. Persistence is opt-in. Tor is already the
network.

Frihart on Tails must:

- Use the **existing** Tor SOCKS (`127.0.0.1:9050`). Never start a
  second daemon. Never fall back to clearnet.
- Default to an **ephemeral profile** unless the user pointed
  `--profile` at Persistent storage.
- Write nothing to the root filesystem that survives reboot.
- Ship as a `.deb` the user can install to persistence if they want.

We do not replace Tor Browser on Tails in v1. We sit beside it for
people who want Frihart's chrome and policy on a Tails stick.

## Qubes OS (planned)

Qubes is compartments, not a desktop theme.

Frihart on Qubes must:

- Run as a normal app in an AppVM or a DisposableVM
- Treat a DisposableVM like `--private` (memory-only profile)
- Never assume a physical NIC. Traffic goes through `sys-net` /
  `sys-firewall` / `sys-whonix` as the user set
- For Whonix: SOCKS as that workstation documents (often the
  Whonix gateway). Fail closed if SOCKS is down
- Package for **Fedora** and **Debian** templates — the two people
  actually use

We will not invent qrexec features in v1. Isolation is Qubes' job;
Frihart must not punch holes in it.

## Everyone else

Alpine, Gentoo, NixOS, openSUSE, Slackware, and the rest: build from
source with the same `cargo build --release`. If you maintain a
package, keep it offline-first: no extra repos that phone home.
