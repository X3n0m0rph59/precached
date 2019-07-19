### Precached Selinux Guide

_change working directory to precached/support/selinux_

#### Change on version update
```
precached.te > policy_module(precached, 1.1.1)
precached_selinux.spec > %define selinux_policyver 1.1.1
precached_selinux.spec > Version:        1.1
```
#### WARNING deletes and generate new policy, only use for new barebone policy
`((sudo sepolicy generate --init /usr/sbin/precached))`

#### check syntax, test and build policy
`sudo ./precached.sh`

#### update precached.pp policy
`sudo make -f /usr/share/selinux/devel/Makefile precached.pp`

#### install precached.pp policy (optional)
`sudo /usr/sbin/semodule -i precached.pp`

#### check syntax, test and build policy
`sudo ./precached.sh`
#### install policy
`sudo rpm -ivh ./noarch/*.rpm`

#### check for re confinement
`ls -Zd /var/lib/precached/`

#### restart precached service
`sudo systemctl restart precached`

#### update policy with new allow rules 
_(cherry pick needed allow rules from the next 2 commands into precached.te per hand)_
```
sudo ausearch -c 'precached' --raw | audit2allow -M my-precached
sudo ausearch -c 'precached/fanot' --raw | audit2allow -M my-precachedfanot
```
#### check syntax, test and build policy
`(sudo ./precached.sh –update)`

#
#### switch to enforcing after the policy reached a stable level
`remove "permissive precached_t;" from precached.te`

#### to avoid issues the domain can be set to permissive at first use
`(sudo semanage permissive -a precached_t)`

#
