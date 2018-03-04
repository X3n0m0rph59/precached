Name:    precached-git
Version: 1.1.0
Release: 138%{?dist}
Summary: precached - A Linux process monitor and pre-caching daemon
URL:     https://x3n0m0rph59.github.io/precached/
License: GPLv3+

# Source0: https://github.com/X3n0m0rph59/precached.git
Source0: https://github.com/X3n0m0rph59/%{name}/archive/master.tar.gz

BuildRoot: %{_tmppath}/%{name}-%{commit}-build

BuildRequires: systemd
BuildRequires: dbus-devel
BuildRequires: zeromq-devel
BuildRequires: cargo

Requires: dbus zeromq

%global gittag master
%global debug_package %{nil}

%description
Precached is written in Rust and utilizes the Linux netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. it is able to pre-fault pages into memory, to speed up
loading of programs and increase the perceived overall 'snappiness' of the
system. Additionally it supports offline prefetching of the most often used
programs while the system is idle.

%prep
%autosetup -n %{name}-%{commit}

%build
cargo build --all --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man1
%{__mkdir_p} %{buildroot}%{_mandir}/man5
%{__mkdir_p} %{buildroot}%{_mandir}/man8
%{__mkdir_p} %{buildroot}%{_datarootdir}/metainfo/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{name}/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{name}/rules.d/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/dbus-1/system.d/
%{__mkdir_p} %{buildroot}%{_unitdir}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{name}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{name}/iotrace/
%{__mkdir_p} %{buildroot}%{_docdir}/%{name}/
%{__mkdir_p} %{buildroot}%{_datarootdir}/bash-completion/completions/
%{__mkdir_p} %{buildroot}%{_datarootdir}/zsh/site-functions/
#%{__mkdir_p} %{buildroot}%{_datadir}/{name}/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.conf.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.rules.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{version}/support/man/iotracectl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precachedctl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precachedtop.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/rulesctl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/config/precached.conf %{buildroot}/%{_sysconfdir}/%{name}/
cp -a %{_builddir}/%{name}-%{version}/support/rules/README %{buildroot}/%{_sysconfdir}/%{name}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/00-log-fork-bombs.rules %{buildroot}/%{_sysconfdir}/%{name}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/10-cache-on-login.rules %{buildroot}/%{_sysconfdir}/%{name}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/99-ping-logger.rules %{buildroot}/%{_sysconfdir}/%{name}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached-prime-caches.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached-prime-caches.timer %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/dbus/org.precached.precached1.conf %{buildroot}/%{_sysconfdir}/dbus-1/system.d/
cp -a %{_builddir}/%{name}-%{version}/support/appstream/org.precache.precached.appdata.xml %{buildroot}/%{_datarootdir}/metainfo/
cp -ra %{_builddir}/%{name}-%{version}/support/config/examples %{buildroot}/%{_docdir}/%{name}/
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/iotracectl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/iotracectl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/precachedctl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precachedctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/precachedtop.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precachedtop
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/rulesctl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/rulesctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/iotracectl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_iotracectl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/precachedctl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precachedctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/precachedtop.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precachedtop
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/rulesctl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_rulesctl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precached %{buildroot}%{_sbindir}/precached
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precachedctl %{buildroot}%{_sbindir}/precachedctl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/iotracectl %{buildroot}%{_sbindir}/iotracectl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precachedtop %{buildroot}%{_sbindir}/precachedtop
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/rulesctl %{buildroot}%{_sbindir}/rulesctl

%post
case "$1" in
  2)
  # we are being upgraded
  # echo "Clearing old I/O trace logs..."
  # iotracectl clear > /dev/null 2>&1
  ;;
esac
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service
case "$1" in
  0)
  # we are being erased
  # echo "Clearing old I/O trace logs..."
  # iotracectl clear > /dev/null 2>&1
  ;;
esac

%postun
%systemd_postun_with_restart %{name}.service

%files
%license LICENSE
%doc %{_mandir}/man5/precached.conf.5.gz
%doc %{_mandir}/man5/precached.rules.5.gz
%doc %{_mandir}/man8/iotracectl.8.gz
%doc %{_mandir}/man8/precachedctl.8.gz
%doc %{_mandir}/man8/precachedtop.8.gz
%doc %{_mandir}/man8/rulesctl.8.gz
%doc %{_mandir}/man8/precached.8.gz
%dir %{_docdir}/%{name}/examples/
%dir %{_datarootdir}/bash-completion/completions/
%dir %{_datarootdir}/zsh/site-functions/
%dir %{_sysconfdir}/%{name}/rules.d/
# %docdir %{_docdir}/%{name}/examples/
%config(noreplace) %{_sysconfdir}/%{name}/%{name}.conf
%config(noreplace) %{_sysconfdir}/%{name}/rules.d/
%{_sbindir}/precached
%{_sbindir}/precachedctl
%{_sbindir}/iotracectl
%{_sbindir}/rulesctl
%{_sbindir}/precachedtop
%{_unitdir}/precached.service
%{_unitdir}/precached-prime-caches.service
%{_unitdir}/precached-prime-caches.timer
%config(noreplace) %{_sysconfdir}/dbus-1/system.d/org.precached.precached1.conf
%{_datarootdir}/metainfo/org.precache.precached.appdata.xml
%{_sharedstatedir}/%{name}/
%{_sharedstatedir}/%{name}/iotrace/
%{_datarootdir}/bash-completion/completions/iotracectl
%{_datarootdir}/bash-completion/completions/precachedctl
%{_datarootdir}/bash-completion/completions/precachedtop
%{_datarootdir}/bash-completion/completions/rulesctl
%{_datarootdir}/zsh/site-functions/_iotracectl
%{_datarootdir}/zsh/site-functions/_precachedctl
%{_datarootdir}/zsh/site-functions/_precachedtop
%{_datarootdir}/zsh/site-functions/_rulesctl
%{_docdir}/%{name}/examples/
#%{_datadir}/%{name}/

%changelog
* Sun Jan 14 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-136
- rebuilt

* Sun Jan 14 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-135
- rebuilt

* Tue Jan 09 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-134
- rebuilt

* Tue Jan 09 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-133
- rebuilt

* Tue Jan 09 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-132
- rebuilt

* Tue Jan 09 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-131
- rebuilt

* Sun Jan 07 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-130
- rebuilt

* Sun Jan 07 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-129
- rebuilt

* Sun Jan 07 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-128
- rebuilt

* Sat Jan 06 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-127
- rebuilt

* Sat Jan 06 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-126
- rebuilt

* Thu Jan 04 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-125
- rebuilt

* Thu Jan 04 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-124
- rebuilt

* Tue Jan 02 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-123
- rebuilt

* Sun Dec 31 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-122
- rebuilt

* Sun Dec 31 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-121
- rebuilt

* Tue Dec 26 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-120
- rebuilt

* Mon Dec 25 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-119
- rebuilt

* Mon Dec 25 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-118
- rebuilt

* Mon Dec 25 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-117
- rebuilt

* Fri Dec 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-116
- rebuilt

* Thu Dec 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-115
- rebuilt

* Thu Dec 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-114
- rebuilt

* Sun Dec 03 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-113
- rebuilt

* Sun Nov 26 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-112
- rebuilt

* Sun Nov 26 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-111
- rebuilt

* Thu Nov 23 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-110
- rebuilt

* Thu Nov 23 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-109
- rebuilt

* Wed Nov 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-108
- rebuilt

* Wed Nov 22 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-107
- rebuilt

* Tue Nov 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-106
- rebuilt

* Tue Nov 21 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-105
- rebuilt

* Mon Nov 20 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-104
- rebuilt

* Sat Nov 18 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-103
- rebuilt

* Fri Nov 17 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-102
- rebuilt

* Thu Nov 16 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-101
- rebuilt

* Tue Nov 14 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-100
- rebuilt

* Sun Nov 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-99
- rebuilt

* Sun Nov 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-98
- rebuilt

* Sun Nov 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-97
- rebuilt

* Sun Nov 12 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-96
- rebuilt

* Fri Nov 10 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-95
- rebuilt

* Fri Nov 10 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-94
- rebuilt

* Thu Nov 09 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-93
- rebuilt

* Thu Nov 09 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-92
- rebuilt

* Tue Nov 07 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-91
- rebuilt

* Tue Nov 07 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-90
- rebuilt

* Tue Nov 07 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-89
- rebuilt

* Sun Nov 05 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-88
- rebuilt

* Sat Nov 04 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-87
- rebuilt

* Mon Oct 30 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-86
- rebuilt

* Mon Oct 30 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-85
- rebuilt

* Mon Oct 30 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-84
- rebuilt

* Sun Oct 29 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-83
- rebuilt

* Sun Oct 29 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-82
- rebuilt

* Sun Oct 29 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-81
- rebuilt

* Sat Oct 28 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-80
- rebuilt

* Fri Oct 27 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-79
- rebuilt

* Thu Oct 26 2017 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 0.1.0-78
- rebuilt

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
