Name:           frihart
Version:        0.1.0
Release:        1%{?dist}
Summary:        Sovereign privacy-first web browser
License:        MIT OR Apache-2.0
URL:            https://codeberg.org/ichbinlucasv/Frihart
Source0:        %{url}/archive/v%{version}.tar.gz
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  fontconfig-devel
BuildRequires:  freetype-devel
Requires:       fontconfig
Requires:       freetype

%description
Frihart is an original privacy-first web browser written in Rust.
LibreWolf-inspired, not a fork. Arch / CachyOS is the reference;
this spec is the Fedora family package.

%prep
%autosetup -n Frihart

%build
cargo build --release --locked --bin frihart

%install
install -D -m 0755 target/release/frihart %{buildroot}%{_bindir}/frihart
install -D -m 0644 packaging/linux/org.frihart.Frihart.desktop \
    %{buildroot}%{_datadir}/applications/org.frihart.Frihart.desktop
install -D -m 0644 LICENSE-MIT %{buildroot}%{_datadir}/licenses/%{name}/LICENSE-MIT
install -D -m 0644 LICENSE-APACHE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE-APACHE

%files
%{_bindir}/frihart
%{_datadir}/applications/org.frihart.Frihart.desktop
%license LICENSE-MIT LICENSE-APACHE

%changelog
* Tue Aug 18 2026 Lucas <codeberg.ecx3s@passmail.com> - 0.1.0-1
- Initial Fedora package
