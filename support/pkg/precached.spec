%global OrigName precached

Name:    precached
Version: 1.3.0
Release: 0%{?dist}
Summary: precached - A Linux process monitor and pre-caching daemon
URL:     https://x3n0m0rph59.gitlab.io/precached/
License: GPLv3+

# Source0: https://gitlab.com/X3n0m0rph59/precached.git
Source0: https://gitlab.com/X3n0m0rph59/%{OrigName}/-/archive/master/%{OrigName}-master.tar.gz

BuildRoot: %{_tmppath}/%{name}-build

BuildRequires: systemd
BuildRequires: dbus-devel
BuildRequires: zeromq-devel
BuildRequires: cargo

Requires: dbus zeromq

Conflicts: precached

%global gittag master
%global debug_package %{nil}

%description
Precached is written in Rust and utilizes the Linux Netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. it is able to pre-fault pages into memory, to speed up
loading of programs and increase the perceived overall 'snappiness' of the
system. Additionally it supports offline prefetching of the most often used
programs while the system is idle.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --all --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man1
%{__mkdir_p} %{buildroot}%{_mandir}/man5
%{__mkdir_p} %{buildroot}%{_mandir}/man8
%{__mkdir_p} %{buildroot}%{_datarootdir}/metainfo/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{OrigName}/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/%{OrigName}/rules.d/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/dbus-1/system.d/
%{__mkdir_p} %{buildroot}%{_sysconfdir}/xdg/autostart/
%{__mkdir_p} %{buildroot}%{_unitdir}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{OrigName}/
%{__mkdir_p} %{buildroot}%{_sharedstatedir}/%{OrigName}/iotrace/
%{__mkdir_p} %{buildroot}%{_docdir}/%{OrigName}/
%{__mkdir_p} %{buildroot}%{_datarootdir}/icons/hicolor/scalable/apps/
%{__mkdir_p} %{buildroot}%{_datarootdir}/bash-completion/completions/
%{__mkdir_p} %{buildroot}%{_datarootdir}/zsh/site-functions/
%{__mkdir_p} %{buildroot}%{_datarootdir}/%{OrigName}/i18n/
#%{__mkdir_p} %{buildroot}%{_datadir}/{OrigName}/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.conf.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.rules.5 %{buildroot}/%{_mandir}/man5/
cp -a %{_builddir}/%{name}-%{version}/support/man/iotracectl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precachedctl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precachedtop.1 %{buildroot}/%{_mandir}/man1/
cp -a %{_builddir}/%{name}-%{version}/support/man/rulesctl.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached-trigger.1 %{buildroot}/%{_mandir}/man1/
cp -a %{_builddir}/%{name}-%{version}/support/man/precached-debug.8 %{buildroot}/%{_mandir}/man8/
cp -a %{_builddir}/%{name}-%{version}/support/config/precached.conf %{buildroot}/%{_sysconfdir}/%{OrigName}/
cp -a %{_builddir}/%{name}-%{version}/support/config/log4rs.yaml %{buildroot}/%{_sysconfdir}/%{OrigName}/
cp -a %{_builddir}/%{name}-%{version}/support/rules/README %{buildroot}/%{_sysconfdir}/%{OrigName}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/00-log-fork-bombs.rules %{buildroot}/%{_sysconfdir}/%{OrigName}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/10-cache-on-login.rules %{buildroot}/%{_sysconfdir}/%{OrigName}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/rules/99-ping-logger.rules %{buildroot}/%{_sysconfdir}/%{OrigName}/rules.d/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached-prime-caches.service %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/systemd/precached-prime-caches.timer %{buildroot}/%{_unitdir}/
cp -a %{_builddir}/%{name}-%{version}/support/dbus/org.precached.precached1.conf %{buildroot}/%{_sysconfdir}/dbus-1/system.d/
cp -a %{_builddir}/%{name}-%{version}/support/appstream/org.precache.precached.appdata.xml %{buildroot}/%{_datarootdir}/metainfo/
cp -ra %{_builddir}/%{name}-%{version}/support/config/examples %{buildroot}/%{_docdir}/%{OrigName}/
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/iotracectl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/iotracectl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precachedctl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precachedctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precachedtop.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precachedtop
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/rulesctl.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/rulesctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precached-trigger.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precached-trigger
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precached-debug.bash-completion %{buildroot}/%{_datarootdir}/bash-completion/completions/precached-debug
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/iotracectl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_iotracectl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precachedctl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precachedctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precachedtop.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precachedtop
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/rulesctl.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_rulesctl
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precached-trigger.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precached-trigger
cp -a %{_builddir}/%{name}-%{version}/support/shell/completions/en_US/precached-debug.zsh-completion %{buildroot}/%{_datarootdir}/zsh/site-functions/_precached-debug
cp -a %{_builddir}/%{name}-%{version}/support/appstream/org.precache.precached-trigger.appdata.xml %{buildroot}/%{_datarootdir}/metainfo/
cp -a %{_builddir}/%{name}-%{version}/support/desktop/precached-trigger.desktop %{buildroot}/%{_sysconfdir}/xdg/autostart/precached-trigger.desktop
cp -a %{_builddir}/%{name}-%{version}/support/assets/precached.svg %{buildroot}/%{_datarootdir}/icons/hicolor/scalable/apps/precached-trigger.svg
cp -ra %{_builddir}/%{name}-%{version}/support/i18n %{buildroot}/%{_datarootdir}/%{OrigName}/
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precached %{buildroot}%{_sbindir}/precached
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precachedctl %{buildroot}%{_sbindir}/precachedctl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/iotracectl %{buildroot}%{_sbindir}/iotracectl
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precachedtop %{buildroot}%{_bindir}/precachedtop
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/rulesctl %{buildroot}%{_sbindir}/rulesctl
install -Dp -m 4755 %{_builddir}/%{name}-%{version}/target/release/precached-trigger %{buildroot}%{_bindir}/precached-trigger
install -Dp -m 0755 %{_builddir}/%{name}-%{version}/target/release/precached-debug %{buildroot}%{_sbindir}/precached-debug

%post
case "$1" in
  2)
  # we are being upgraded
  # echo "Clearing old I/O trace logs..."
  # iotracectl clear > /dev/null 2>&1
  ;;
esac
%systemd_post %{OrigName}.service

%preun
%systemd_preun %{OrigName}.service
case "$1" in
  0)
  # we are being erased
  # echo "Clearing old I/O trace logs..."
  # iotracectl clear > /dev/null 2>&1
  ;;
esac

%postun
%systemd_postun_with_restart %{OrigName}.service

%files
%license LICENSE
%doc %{_mandir}/man5/precached.conf.5.gz
%doc %{_mandir}/man5/precached.rules.5.gz
%doc %{_mandir}/man8/iotracectl.8.gz
%doc %{_mandir}/man8/precachedctl.8.gz
%doc %{_mandir}/man8/rulesctl.8.gz
%doc %{_mandir}/man8/precached-debug.8.gz
%doc %{_mandir}/man8/precached.8.gz
%doc %{_mandir}/man1/precachedtop.1.gz
%doc %{_mandir}/man1/precached-trigger.1.gz
%dir %{_docdir}/%{OrigName}/examples/
%dir %{_sysconfdir}/xdg/autostart/
%dir %{_datarootdir}/icons/hicolor/scalable/apps/
%dir %{_datarootdir}/bash-completion/completions/
%dir %{_datarootdir}/zsh/site-functions/
%dir %{_sysconfdir}/%{OrigName}/rules.d/
%dir %{_datarootdir}/%{OrigName}/i18n/
# %docdir %{_docdir}/%{OrigName}/examples/
%config(noreplace) %{_sysconfdir}/%{OrigName}/%{OrigName}.conf
%config(noreplace) %{_sysconfdir}/%{OrigName}/log4rs.yaml
%config(noreplace) %{_sysconfdir}/%{OrigName}/rules.d/
%{_sbindir}/precached
%{_sbindir}/precached-debug
%{_sbindir}/precachedctl
%{_sbindir}/iotracectl
%{_sbindir}/rulesctl
%{_bindir}/precachedtop
%attr(4755, root, root) %{_bindir}/precached-trigger
%{_unitdir}/precached.service
%{_unitdir}/precached-prime-caches.service
%{_unitdir}/precached-prime-caches.timer
%config(noreplace) %{_sysconfdir}/dbus-1/system.d/org.precached.precached1.conf
%{_datarootdir}/metainfo/org.precache.precached.appdata.xml
%{_datarootdir}/metainfo/org.precache.precached-trigger.appdata.xml
%{_sharedstatedir}/%{OrigName}/
%{_sharedstatedir}/%{OrigName}/iotrace/
%{_sysconfdir}/xdg/autostart/precached-trigger.desktop
%{_datarootdir}/icons/hicolor/scalable/apps/precached-trigger.svg
%{_datarootdir}/bash-completion/completions/iotracectl
%{_datarootdir}/bash-completion/completions/precachedctl
%{_datarootdir}/bash-completion/completions/precachedtop
%{_datarootdir}/bash-completion/completions/rulesctl
%{_datarootdir}/bash-completion/completions/precached-trigger
%{_datarootdir}/bash-completion/completions/precached-debug
%{_datarootdir}/zsh/site-functions/_iotracectl
%{_datarootdir}/zsh/site-functions/_precachedctl
%{_datarootdir}/zsh/site-functions/_precachedtop
%{_datarootdir}/zsh/site-functions/_rulesctl
%{_datarootdir}/zsh/site-functions/_precached-trigger
%{_datarootdir}/zsh/site-functions/_precached-debug
%{_docdir}/%{OrigName}/examples/
#%{_datadir}/%{OrigName}/
%{_datarootdir}/%{OrigName}/i18n/C
%{_datarootdir}/%{OrigName}/i18n/de_AT
%{_datarootdir}/%{OrigName}/i18n/de_AT.UTF-8
%{_datarootdir}/%{OrigName}/i18n/de_AT.utf8
%{_datarootdir}/%{OrigName}/i18n/de_DE.UTF-8
%{_datarootdir}/%{OrigName}/i18n/de_DE.utf8
%{_datarootdir}/%{OrigName}/i18n/de_DE/messages.fluent
%{_datarootdir}/%{OrigName}/i18n/en_US/messages.fluent
%{_datarootdir}/%{OrigName}/i18n/en_UK

%changelog
* Mon Aug 06 2018 X3n0m0rph59 <x3n0m0rph59@gmail.com> - 1.3.0-0
- rebuilt
