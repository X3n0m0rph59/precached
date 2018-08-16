---
layout: post
title: "First Prototype Implementation released"
date:   2017-09-28 15:45:00
categories: update release
---

# First Prototype Implementation released

We just released the first working prototype!
It is currently able to monitor events sent by the Linux kernel whenever a
fork()/execve() syscall is executed, and subsequently scan the newly created
process' file mappings. Valid mappings are then mlocked() to the virtual
memory of our precached process.

![Code](/precached/images/code.png)
*Rust language*

![Screenshot](/precached/images/screenshot.png)
*Prototype of precached running on Linux*

## TODO List (non-exhaustive):

* Implement ceiling on mlocked() memory
* Implement VFS statx() caching (pre-read file metadata)
* Possibly implement fork-bomb mitigation
* Implement a persistence layer
* Prime caches on daemon startup
* Systemd init scripts
* External configuration support (/etc/precached/)
* Daemonization support
* Implement the DBUS interface
* Write a nice CLI tool to control the daemon
* And write a precached GUI in GTK+
* And much more...
