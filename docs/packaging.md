# Packaging Frihart on Linux

Arch / CachyOS is the reference. Fedora and Debian are first-class
follow-ups so the same binary can land on every major distro family.

| Distro family | Path | Tool |
| --- | --- | --- |
| Arch, CachyOS, EndeavourOS, Manjaro | `packaging/arch/PKGBUILD` | `makepkg`, later AUR |
| Fedora, RHEL, CentOS Stream | `packaging/fedora/frihart.spec` | `rpmbuild`, later COPR |
| Debian, Ubuntu, Mint | `packaging/debian/` | `dpkg-buildpackage` |

All three produce `/usr/bin/frihart` and a desktop file. They do not
phone home. Updates come from the distro or from git.

## Build from source (any distro)

```bash
sudo packages for the usual Rust desktop stack:
# Arch / CachyOS
sudo pacman -S rust fontconfig freetype2
# Fedora
sudo dnf install cargo rustc fontconfig-devel freetype-devel gcc
# Debian / Ubuntu
sudo apt install cargo rustc libfontconfig1-dev libfreetype-dev gcc pkg-config

cargo build --release
sudo install -Dm755 target/release/frihart /usr/local/bin/frihart
```

## Tor (optional, for `--tor` tabs)

Install the **system** Tor daemon. Frihart talks to `127.0.0.1:9050`.

```bash
# Arch / CachyOS
sudo pacman -S tor && sudo systemctl enable --now tor
# Fedora
sudo dnf install tor && sudo systemctl enable --now tor
# Debian / Ubuntu
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

There is no extension store yet. See `docs/extensions.md`.
