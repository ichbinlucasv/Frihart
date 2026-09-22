# Arch / CachyOS package

Reference Linux desktop package. Arch and CachyOS share this
`PKGBUILD`; Manjaro and EndeavourOS detect as Arch.

This is **desktop Linux only** (campaign F). The quality bar is a
stable, fast daily-driver binary on Arch/Cachy before any mobile port
work — see [../linux/README.md](../linux/README.md) and
[docs/mobile-targets.md](../../docs/mobile-targets.md).

```bash
cd packaging/arch
FRIHART_LOCAL=1 makepkg -f --nocheck
sudo pacman -U frihart-0.1.0-1-*.pkg.tar.zst
```

Installs `/usr/bin/frihart`, the shared desktop entry, and hicolor
icons. Tor / I2P are optional system daemons — see
[docs/packaging.md](../../docs/packaging.md).
