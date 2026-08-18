# Packaging Frihart on Linux

Arch / CachyOS is the reference. Fedora and Debian/Mint follow so the
same binary lands on every major family. Tails and Qubes are later
homes with extra OPSEC rules — see [distros.md](distros.md).

| Distro family | Path | Tool |
| --- | --- | --- |
| Arch, CachyOS, EndeavourOS, Manjaro | `packaging/arch/PKGBUILD` | `makepkg`, later AUR |
| Fedora, RHEL, CentOS Stream | `packaging/fedora/frihart.spec` | `rpmbuild`, later COPR |
| Debian, Ubuntu, Mint | `packaging/debian/` | `dpkg-buildpackage` |
| Tails | Debian `.deb` + amnesic notes | persistence optional |
| Qubes OS | Fedora + Debian templates | AppVM / DisposableVM |

All produce `/usr/bin/frihart` and a desktop file. They do not phone
home. Updates come from the distro or from git.

## Build from source (any distro)

```bash
# Arch / CachyOS
sudo pacman -S rust fontconfig freetype2
# Fedora
sudo dnf install cargo rustc fontconfig-devel freetype-devel gcc
# Debian / Ubuntu / Mint
sudo apt install cargo rustc libfontconfig1-dev libfreetype-dev gcc pkg-config

cargo build --release
sudo install -Dm755 target/release/frihart /usr/local/bin/frihart
```

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
