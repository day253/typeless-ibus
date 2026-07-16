%global debug_package %{nil}

Name:           typeless-ibus
Version:        0.5.2
Release:        1%{?dist}
Summary:        Native IBus voice input engine for Linux

License:        MIT
URL:            https://github.com/day253/typeless-ibus
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  cmake
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  make
BuildRequires:  pkgconfig
BuildRequires:  pkgconfig(alsa)
BuildRequires:  pkgconfig(opus)
BuildRequires:  rust >= 1.85
Requires:       ibus >= 1.5.22

%description
typeless-ibus records speech while a configurable key is held and commits the
recognized text directly to the focused application through IBus.

%prep
%setup -q

%build
cargo build --release --offline --locked

%check
cargo test --release --offline --locked

%install
install -Dm0755 target/release/typeless-ibus-engine \
  %{buildroot}%{_libexecdir}/typeless-ibus-engine
install -d %{buildroot}%{_datadir}/ibus/component
sed 's|/usr/libexec/typeless-ibus-engine|%{_libexecdir}/typeless-ibus-engine|' \
  data/typeless.xml > %{buildroot}%{_datadir}/ibus/component/typeless.xml

%post
%{_bindir}/ibus write-cache --system >/dev/null 2>&1 || :

%postun
%{_bindir}/ibus write-cache --system >/dev/null 2>&1 || :

%files
%license LICENSE
%doc README.md README_zh.md CHANGELOG.md THIRD_PARTY.md data/config.example.json
%{_libexecdir}/typeless-ibus-engine
%{_datadir}/ibus/component/typeless.xml

%changelog
* Fri Jul 17 2026 day253 <9634619+day253@users.noreply.github.com> - 0.5.2-1
- Add compact localized listening text for English and Chinese desktops.

* Thu Jul 16 2026 day253 <9634619+day253@users.noreply.github.com> - 0.5.1-1
- Preserve long dictation across ASR segments and recording timeouts.
- Add native ARM64 packages and distribution-specific release assets.

* Wed Jul 15 2026 day253 <9634619+day253@users.noreply.github.com> - 0.5.0-1
- Add native RPM and SRPM packaging for Fedora and openSUSE.

* Wed Jul 15 2026 day253 <9634619+day253@users.noreply.github.com> - 0.4.0-1
- Initial native RPM package.
