---
title: "First Prototype Implementation released"
date:   2017-09-28 15:00:00
categories: update release
---

# First Prototype Implementation released

We just released the first working prototype!
It is currently able to monitor fork()/execve() events sent by the Linux kernel
and subsequently scan process file mappings. Valid mappings are then mlocked()
to the memory of the precached process.

![Code](../images/code.png)
*Rust language*

![Screenshot](../images/screenshot.png)
*precached running on Linux*

## TODO List (non-exhaustive):
	* Implement ceiling on mlocked() memory
	* Implement VFS statx() caching (pre-read file metadata)
	* Implement a persistence layer
	* Prime caches on daemon startup
	* Systemd init scripts
	* External configuration support (/etc/precached/)
	* Daemonization support
	* Implement the DBUS interface
	* Write a CLI tool to control the daemon
	* And write a precached GUI in GTK
	* And much more...
