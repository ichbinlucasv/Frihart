# Linux desktop packaging

This tree is the **free desktop Linux** product path (campaign F).

Quality bar (locked in [ROADMAP.md](../../ROADMAP.md) and
[docs/pricing.md](../../docs/pricing.md)): Frihart must be **stable and
fast on desktop Linux** before any mobile or other-OS packaging work.
Do not treat phone ports as part of these packages.

| Piece | Role |
| --- | --- |
| `org.frihart.Frihart.desktop` | Shared desktop entry (`Icon=frihart`, New Private Window) |
| `../arch/PKGBUILD` | Arch / CachyOS reference package |
| `../fedora/frihart.spec` | Fedora family (incl. Qubes Fedora templates) |
| `../debian/` | Debian / Ubuntu / Mint (incl. Tails / Qubes Debian notes) |
| `../tails/`, `../qubes/` | Amnesic / compartment OPSEC notes |

All of the above install the **same** `/usr/bin/frihart` Linux binary.
How-to: [docs/packaging.md](../../docs/packaging.md). Distro homes:
[docs/distros.md](../../docs/distros.md).

**Planned later (not shipping here):** packaged mobile / non-Linux ports
— GrapheneOS, Jolla (Sailfish), Volla, and similar — are documented in
[docs/mobile-targets.md](../../docs/mobile-targets.md). No mobile builds
or packages live under `packaging/` yet.
