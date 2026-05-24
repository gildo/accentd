# Generate sources with the following:
# export TAG=0.2.1
# git clone --depth=1 --branch=v${TAG} https://github.com/gildo/accentd accentd-${TAG}/
# cd accentd-${TAG}/ && cargo vendor && cd -
# tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner --pax-option='delete=atime,delete=ctime' -cvf accentd-${TAG}.tar.gz accentd-${TAG}/

# Setting up build dependencies can be done with the following:
# dnf install --setopt install_weak_deps=0 rpm-build
# dnf build-dep --setopt install_weak_deps=0 ./dist/accentd.spec

Name:           accentd
Version:        0.2.1
Release:        1%{?dist}
Summary:        Press-and-hold accent character popup for Linux (macOS-style)

License:        MIT
URL:            https://github.com/gildo/accentd

Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

Requires:       gtk4
Requires:       gtk4-layer-shell

BuildRequires:  cargo-rpm-macros
BuildRequires:  crate(cairo-rs)
BuildRequires:  crate(gdk-pixbuf)
BuildRequires:  crate(graphene-sys)
BuildRequires:  crate(gtk4-sys)
BuildRequires:  crate(pango)
BuildRequires:  gtk4-layer-shell-devel
BuildRequires:  systemd-rpm-macros


%description
Press-and-hold accent character popup for Linux, like macOS.

Hold a letter key (e.g. 'e') for 300ms and a popup appears with numbered accented variants.

Press a number to replace the letter. Press ESC to cancel.

Works on any desktop environment and display server: GNOME, KDE, Sway, Hyprland, X11, Wayland. Operates at the evdev/uinput level, same as keyd and kanata.


%prep
%autosetup -n %{name}-%{version}
%cargo_prep -v vendor


%build
%cargo_build
%cargo_license_summary
%cargo_vendor_manifest


%install
install -Dm0755 target/release/accentd       "%{buildroot}%{_bindir}/accentd"
install -Dm0755 target/release/accentd-popup "%{buildroot}%{_bindir}/accentd-popup"
install -Dm0755 target/release/accentctl     "%{buildroot}%{_bindir}/accentctl"

install -Dm644 dist/accentd.service       "%{buildroot}%{_unitdir}/accentd.service"
install -Dm644 dist/accentd-popup.service "%{buildroot}%{_unitdir}/accentd-popup.service"
install -Dm644 dist/70-accentd.rules      "%{buildroot}/usr/lib/udev/rules.d/70-accentd.rules"

install -dm755 "%{buildroot}/%{_datadir}/%{NAME}/locales"
install -Dm644 data/locales/*.toml      "%{buildroot}/%{_datadir}/%{NAME}/locales/"
install -Dm644 data/config.default.toml "%{buildroot}/%{_datadir}/%{NAME}/config.default.toml"


%check
%cargo_test


%post
%systemd_post accentd.service accentd-popup.service

udevadm control --reload-rules ||:


%preun
%systemd_preun accentd.service accentd-popup.service


%postun
%systemd_postun_with_restart accentd.service accentd-popup.service

udevadm control --reload-rules ||:


%files
%license LICENSE
%license cargo-vendor.txt
%doc README.md
%{_bindir}/accentctl
%{_bindir}/accentd
%{_bindir}/accentd-popup
%{_unitdir}/accentd.service
%{_unitdir}/accentd-popup.service
%config %{_datadir}/%{NAME}/locales/*.toml
%config %{_datadir}/%{NAME}/config.default.toml
/usr/lib/udev/rules.d/70-accentd.rules

%changelog
* Thu May 21 2026 Joshua Stone <joshua.gage.stone@gmail.com> - 0.2.1-1
- Initial package
