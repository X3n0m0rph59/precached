---
title: "Project Status Update #3"
date:   2017-11-15 13:23:00
categories: status update
---

# Project Status Update No. 3

Welcome to the precached project status update No. 3!

It's been a while since the last project status update has been published.
Much has happened in the meantime! In the first half of the last four
weeks we mainly added new features and did some internal refactoring
to improve code quality and style. In the second half we mainly did
stabilization work, tuning of parameters and testing.

## What has been achieved

* Internal code refactoring to improve code quality and style
* Added a "Hot Applications" plugin, that supersedes the tentative
  "Dynamic Whitelisting" plugin
* Improved the ftrace based tracing subsystem
* Improved compatibility with other ftrace based tracers
  running on the system at the same time
* Tuned internal parameters and process priorities
* Added tunable memory limits to `/etc/precached/precached.conf`
* Added a memory management subsystem, `mlock()ed` memory can now
  be freed again, on memory pressure
* Reduced overall power consumption somewhat
* Updated systemd unit files: added `precached-prime-caches.timer`
  mechanism to perform a deferred offline-prefetch
* Added "Metadata Whitelisting" mechanism to the "static whitelist"
  plugin to cache `statx()` metadata
* Implement "program blacklisting" by executable path, to allow to
  exclude e.g.: `/usr/bin/cp` or `/usr/bin/mv` from I/O trace generation
* Added file size tracking and tracking of "total prefetch size" to the
  trace log generation subsystem, in addition to tracking just the number
  of performed I/O operations
* Added packages for ubuntu and debian based Linux distros
* General stability improvements

## List of new and noteworthy plugins:

* "Hot Applications" - Tracks the most often used applications and performs
  offline-prefetching, when the system is idle
* "ftrace Messages" - Allows plugins to insert custom data into the kernel's
  ftrace event stream
* The "DBUS-Interface" plugin has been promoted from being a plugin to a core
  infrastructure component of the daemon proper

The "Hot Applications" plugin replaces the old "Dynamic Whitelisting" mechanism
by utilizing a histogram based offline-prefetching approach. This should lead
to a greater hit-rate of the cached data.

## What didn't quite work out

* The binary format of I/O trace logs has changed, therefor a complete retrace
  is needed after upgrade to the current version

Note: We clear all I/O trace logs now on package updates, to account for
incompatible changes in the underlying binary format of *.trace files.

The precached team

## Article Updates

This posting has been edited at: Wed Nov 15 14:54:00

* Fixed typos
* Rewordings
