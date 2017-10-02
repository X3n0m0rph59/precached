Name:    precached
Version: 0.1.0
Release: 1%{?dist}
Summary: precached - A Linux process monitor and pre-caching daemon
URL:     https://x3n0m0rph59.github.io/precached/
License: GPLv3+

# Source0: https://github.com/X3n0m0rph59/precached.git
Source0: https://github.com/X3n0m0rph59/%{name}/archive/master.tar.gz

BuildRequires: systemd
BuildRequires: dbus-devel
BuildRequires: cargo

Requires: dbus

%global gittag master
%global debug_package %{nil}

%description
Precached is written in Rust and utilises the Linux netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. in the future it will be able to pre-fault pages into
memory to speed up loading of programs and increase the perceived overall
'snappiness' of the system.

%prep
%autosetup -n %{name}-%{gittag}

%build
cargo build --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man5
%{__mkdir_p} %{buildroot}%{_mandir}/man8
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{name}/
%{__mkdir_p} %{buildroot}%{_unitdir}/
%{__mkdir_p} %{buildroot}%{_datadir}/{name}/
cp -a %{_builddir}/%{name}-%{gittag}/support/man/precached.conf.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{gittag}/support/man/precached.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{gittag}/support/config/precached.conf %{buildroot}/%{_sysconfdir}/%{name}/
cp -a %{_builddir}/%{name}-%{gittag}/support/systemd/precached.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{gittag}/support/dbus/org.precached.precached1.conf %{buildroot}/%{_sysconfdir}/dbus-1/
install -Dp -m 0755 %{_builddir}/%{name}-%{gittag}/target/release/precached %{buildroot}%{_sbindir}/%{name}

%post
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

%files
%license LICENSE
%doc %{_mandir}/man5/%{name}.conf.5.gz
%doc %{_mandir}/man8/%{name}.8.gz
%config(noreplace) %{_sysconfdir}/%{name}/%{name}.conf
%{_sbindir}/%{name}
%{_unitdir}/%{name}.service
%config(noreplace) %{_sysconfdir}/dbus-1/org.precached.precached1.conf
%{_sharedstatedir}/%{name}/
#%{_datadir}/%{name}/

%changelog
