# Packaging Frihart on Linux

Arch / CachyOS is the reference. Fedora and Debian/Mint follow so the
same binary lands on every major family. Tails and Qubes are later
homes with extra OPSEC rules — see [distros.md](distros.md).

**Linux-first:** these packages are the free **desktop Linux** product
path. The quality bar is stable and fast on desktop Linux before any
mobile or other-OS packaging — see [../packaging/linux/README.md](../packaging/linux/README.md)
and the planned map in [mobile-targets.md](mobile-targets.md). No
mobile builds ship from this tree yet.

| Distro family | Path | Tool |
| --- | --- | --- |
| Arch, CachyOS, EndeavourOS, Manjaro | `packaging/arch/PKGBUILD` | `makepkg`, later AUR |
| Fedora, RHEL, CentOS Stream | `packaging/fedora/frihart.spec` | `rpmbuild`, later COPR |
| Debian, Ubuntu, Mint | `packaging/debian/` | `dpkg-buildpackage` |
| Tails | Debian `.deb` + amnesic notes | persistence optional |
| Qubes OS | Fedora + Debian templates | AppVM / DisposableVM |

All produce `/usr/bin/frihart` and a desktop file (including a
**New Private Window** action: `frihart --private`). The desktop entry
uses `Icon=frihart` and packages install the Templar cross from
`branding/hicolor/{64,128,256,512}/apps/frihart.png` into
`/usr/share/icons/hicolor/…`. Canonical lockup for docs/README is
`branding/frihart-lockup.png` (not installed as a menu icon). They do
not phone home. Updates come from the distro or from git.

Tails/Qubes `.deb`/RPM **publication** is later. The files and the
amnesic default already exist.

## Build from source (any distro)

```bash
# Arch / CachyOS — package from this tree
cd packaging/arch
FRIHART_LOCAL=1 makepkg -f --nocheck
# then: sudo pacman -U frihart-0.1.0-1-*.pkg.tar.zst

# Arch / CachyOS
sudo pacman -S rust fontconfig freetype2
# Fedora
sudo dnf install cargo rustc fontconfig-devel freetype-devel gcc
# Debian / Ubuntu / Mint
sudo apt install cargo rustc libfontconfig1-dev libfreetype-dev gcc pkg-config

cargo build --release
sudo install -Dm755 target/release/frihart /usr/local/bin/frihart
sudo install -Dm644 packaging/linux/org.frihart.Frihart.desktop \
  /usr/local/share/applications/org.frihart.Frihart.desktop
for sz in 64x64 128x128 256x256 512x512; do
  sudo install -Dm644 "branding/hicolor/$sz/apps/frihart.png" \
    "/usr/local/share/icons/hicolor/$sz/apps/frihart.png"
done
# refresh icon cache if you have it
# sudo gtk-update-icon-cache -f /usr/local/share/icons/hicolor 2>/dev/null || true
```

Prefer the distro packages above when you can; they install under `/usr`
and keep the same `Icon=frihart` name.

## Tor (optional, for `--tor` tabs)

Install the **system** Tor daemon. Frihart talks to `127.0.0.1:9050`
unless you change prefs. On Tails, that daemon already exists. Do not
start a second one.

```bash
# Arch / CachyOS
sudo pacman -S tor && sudo systemctl enable --now tor
# Fedora
sudo dnf install tor && sudo systemctl enable --now tor
# Debian / Ubuntu / Mint
sudo apt install tor && sudo systemctl enable --now tor
```

## VPN CLIs (optional)

```bash
# Mullvad — official repo, see https://mullvad.net/download/desktop
# ProtonVPN — official repo, see https://protonvpn.com/support/linux-vpn-setup
```

Frihart detects `mullvad` and `protonvpn-cli` on `PATH`. It does not
bundle either client.

## Extensions

There is no extension store. See `docs/extensions.md`.
