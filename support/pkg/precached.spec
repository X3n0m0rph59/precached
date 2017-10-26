Name:    precached
Version: 0.1.0
Release: 77%{?dist}
Summary: precached - A Linux process monitor and pre-caching daemon
URL:     https://x3n0m0rph59.github.io/precached/
License: GPLv3+

# Source0: https://github.com/X3n0m0rph59/precached.git
Source0: https://github.com/X3n0m0rph59/%{name}/archive/master.tar.gz

BuildRoot: %{_tmppath}/%{name}-%{version}-build

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
%autosetup -n %{name}-%{version}

%build
cargo build --all --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man1
%{__mkdir_p} %{buildroot}%{_mandir}/man5
%{__mkdir_p} %{buildroot}%{_mandir}/man8
%{__mkdir_p} %{buildroot}%{_datarootdir}/metainfo/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{name}/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/dbus-1/
%{__mkdir_p} %{buildroot}%{_unitdir}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{name}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{name}/iotrace/
%{__mkdir_p} %{buildroot}%{_docdir}/%{name}/
#%{__mkdir_p} %{buildroot}%{_datadir}/{name}/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.conf.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{version}/support/man/iotracectl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precachedctl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/config/precached.conf %{buildroot}/%{_sysconfdir}/%{name}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached-prime-caches.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/dbus/org.precached.precached1.conf %{buildroot}/%{_sysconfdir}/dbus-1/
cp -a %{_builddir}/%{name}-%{version}/support/appstream/org.precache.precached.appdata.xml %{buildroot}/%{_datarootdir}/metainfo/
cp -ra %{_builddir}/%{name}-%{version}/support/config/examples %{buildroot}/%{_docdir}/%{name}/
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precached %{buildroot}%{_sbindir}/precached
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precachedctl %{buildroot}%{_sbindir}/precachedctl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/iotracectl %{buildroot}%{_sbindir}/iotracectl

%post
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

%files
%license LICENSE
%doc %{_mandir}/man5/precached.conf.5.gz
%doc %{_mandir}/man8/iotracectl.8.gz
%doc %{_mandir}/man8/precachedctl.8.gz
%doc %{_mandir}/man8/precached.8.gz
%dir %{_docdir}/%{name}/examples/
# %docdir %{_docdir}/%{name}/examples/
%config(noreplace) %{_sysconfdir}/%{name}/%{name}.conf
%{_sbindir}/precached
%{_sbindir}/precachedctl
%{_sbindir}/iotracectl
%{_unitdir}/precached.service
%{_unitdir}/precached-prime-caches.service
%config(noreplace) %{_sysconfdir}/dbus-1/org.precached.precached1.conf
%{_datarootdir}/metainfo/org.precache.precached.appdata.xml
%{_sharedstatedir}/%{name}/
%{_sharedstatedir}/%{name}/iotrace/
%{_docdir}/%{name}/examples/
#%{_datadir}/%{name}/

%changelog
* Thu Oct 26 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-77
- rebuilt

* Wed Oct 25 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-76
- rebuilt

* Wed Oct 25 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-75
- rebuilt

* Tue Oct 24 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-74
- rebuilt

* Tue Oct 24 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-73
- rebuilt

* Tue Oct 24 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-72
- rebuilt

* Tue Oct 24 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-71
- rebuilt

* Mon Oct 23 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-70
- rebuilt

* Mon Oct 23 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-69
- rebuilt

* Sun Oct 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-68
- rebuilt

* Sun Oct 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-67
- rebuilt

* Sun Oct 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-66
- rebuilt

* Sun Oct 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-65
- rebuilt

* Sat Oct 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-64
- rebuilt

* Sat Oct 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-63
- rebuilt

* Sat Oct 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-62
- rebuilt

* Sat Oct 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-61
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-60
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-59
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-58
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-57
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-56
- rebuilt

* Wed Oct 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-55
- rebuilt

* Tue Oct 17 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-54
- rebuilt

* Tue Oct 17 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-53
- rebuilt

* Mon Oct 16 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-52
- rebuilt

* Mon Oct 16 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-51
- rebuilt

* Sun Oct 15 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-50
- rebuilt

* Sun Oct 15 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-49
- rebuilt

* Sun Oct 15 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-48
- rebuilt

* Sun Oct 15 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-47
- rebuilt

* Sat Oct 14 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-46
- rebuilt

* Sat Oct 14 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-45
- rebuilt

* Fri Oct 13 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-44
- rebuilt

* Fri Oct 13 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-43
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-41
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-40
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-39
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-38
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-37
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-36
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-35
- rebuilt

* Thu Oct 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-34
- rebuilt
