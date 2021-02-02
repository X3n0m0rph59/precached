# vim: sw=4:ts=4:et


%define relabel_files() \
restorecon -R /usr/sbin/precached; \
restorecon -R /usr/sbin/precachedctl; \
restorecon -R /usr/sbin/precached-debug; \
restorecon -R /usr/sbin/iotracectl; \
restorecon -R /usr/sbin/rulesctl; \
restorecon -R /usr/bin/precachedtop; \
restorecon -R /usr/bin/precached-trigger; \
restorecon -R /usr/bin/precached-gui; \
restorecon -R /var/lib/precached; \
restorecon -R /var/run/precached; \
restorecon -R /var/log/precached; \

%define selinux_policyver 1.3.1

Name:   precached_selinux
Version:	1.3
Release:	1%{?dist}
Summary:	SELinux policy module for precached

Group:	System Environment/Base
License:	GPLv3+
URL:		https://x3n0m0rph59.gitlab.io/precached
Source0:	precached.pp
Source1:	precached.if
Source2:	precached_selinux.8


Requires: policycoreutils, libselinux-utils
Requires(post): selinux-policy-base >= %{selinux_policyver}, policycoreutils
Requires(postun): policycoreutils
BuildArch: noarch

%description
This package installs and sets up the  SELinux policy security module for precached.

%install
install -d %{buildroot}%{_datadir}/selinux/packages
install -m 644 %{SOURCE0} %{buildroot}%{_datadir}/selinux/packages
install -d %{buildroot}%{_datadir}/selinux/devel/include/contrib
install -m 644 %{SOURCE1} %{buildroot}%{_datadir}/selinux/devel/include/contrib/
install -d %{buildroot}%{_mandir}/man8/
install -m 644 %{SOURCE2} %{buildroot}%{_mandir}/man8/precached_selinux.8
install -d %{buildroot}/etc/selinux/targeted/contexts/users/


%post
semodule -n -i %{_datadir}/selinux/packages/precached.pp
semanage port -a -t precached_webapp_port_t -p tcp 8023
if /usr/sbin/selinuxenabled ; then
    /usr/sbin/load_policy
    %relabel_files

fi;
exit 0

%postun
if [ $1 -eq 0 ]; then
    semodule -n -r precached
    semanage port -d -p tcp 8023;
    if /usr/sbin/selinuxenabled ; then
       /usr/sbin/load_policy
       %relabel_files

    fi;
fi;
exit 0

%files
%attr(0600,root,root) %{_datadir}/selinux/packages/precached.pp
%{_datadir}/selinux/devel/include/contrib/precached.if
%{_mandir}/man8/precached_selinux.8.*


%changelog
* Thu Jul 21 2019 Ivo Damjanovic <ivo@damjanovic.it> 1.3-1
- added iotracectl rulesctl to binaries
* Thu Jul 19 2019 Ivo Damjanovic <ivo@damjanovic.it> 1.2-1
- added var_run_t var_lib_t to allow rules
* Thu Jul 19 2019 Ivo Damjanovic <ivo@damjanovic.it> 1.1-1
- added more allow rules
* Thu Jul 18 2019 Ivo Damjanovic <ivo@damjanovic.it> 1.0-1
- Initial version

