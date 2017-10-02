Name:    precached
Version: 0.1.0
Release: 1%{?dist}
Summary: precached - A Linux process monitor and pre-caching daemon
URL:     https://github.com/X3n0m0rph59/precached

License: GPLv3+
Source0: https://github.com/X3n0m0rph59/precached.git

%description
Precached is written in Rust and utilises the Linux netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. in the future it will be able to pre-fault pages into
memory to speed up loading of programs and increase the perceived overall
'snappiness' of the system.

%build
cargo build --release --verbose

%install
%{__mkdir_p} %{buildroot}%{_mandir}/man5
%{__mkdir_p} %{buildroot}%{_mandir}/man8
cp -a support/man/precached.conf.5 %{buildroot}/%{_mandir}/man5/
cp -a support/man/precached.8 %{buildroot}/%{_mandir}/man8/
install -Dp -m 0755 target/release/precached %{buildroot}%{_sbindir}/precached

%files
%doc %{_mandir}/man5/precached.conf.5
%doc %{_mandir}/man8/precached.8
%license LICENSE COPYING
%{_sbindir}/precached
%{_datadir}/%{name}/
%config(noreplace) %{_sysconfdir}/%{name}/precached.conf

%changelog
