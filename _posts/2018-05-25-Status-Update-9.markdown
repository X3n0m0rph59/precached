---
layout: post
title: "Project Status Update #9"
date:   2018-05-25 10:59:00
tags: status update
---

# Project Status Update No. 9

Welcome to the precached project status update No. 9!

In this cycle we mainly concentrated on stabilizing the project and on
preparing the 1.2 release.

## What has been achieved

All external dependencies have been updated to their latest versions.
Additionally the big i18n support patch has landed. We added localizations
for the english and german languages. Most user visible strings have been
translated. Translation of error and other log messages still remains to be
done.

### precached-debug

The project gained a new debugging tool called `precached-debug`. It may be
used to exercise the I/O tracing subsystem of `precached`. For now it supports
the creation and subsequent accessing of files.

### Memory consumption analysis

The companion tool `iotracectl` learned to calculate memory consumption
informations for I/O trace logs. It can be used to analyze which I/O trace logs,
and subsequently which files contribute the most to the overall memory usage of
the `precached` daemon. For instance the command
`iotracectl sizes --executable=firefox` will show the amount of memory that
will be consumed if the files recorded in the I/O trace logs of the `firefox`
executable get loaded into the cache.

![Screenshot](/precached/images/iotracectl-sizes.png)

### Introspection

We added an introspection feature to the `precached` daemon. The companion tool
`precachedctl` gained the ability to show this introspection data.
The command `precachedctl plugins analyze internal-state` will print a tabular
view of key and value pairs, showing the most important internal state
parameters of the running `precached` process. Additionally a warning level
indicator is displayed, which indicates if the value is in nominal range.

![Screenshot](/precached/images/precachedctl-internal-state.png)

### Runtime Profiles

The daemon now supports switching between different profiles. For now, we added
the two profiles: `BootUp` and `UpAndRunning`. After the daemon has been started
it enters the `BootUp` profile, where only online-prefetching will be performed.
Offline-prefetching will be activated after
`precached-trigger transition-profile` got executed, and the daemon transitioned
to the `UpAndRunning` profile. To put it simple, offline-prefetching will now
be started as soon as the user logs into the desktop. We supply a .desktop
file that gets installed in the xdg-autostart directory. It runs the
aforementioned `precached-trigger transition-profile`. This leads to somewhat
faster boot times, since the prefetching is postponed to a later stage of the
system's start-up process.

The precached team
