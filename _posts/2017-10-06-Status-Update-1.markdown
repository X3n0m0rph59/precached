---
title: "Project Status Update #1"
date:   2017-10-06 12:59:00
categories: status update
---

# Project Status Update

From this day on, we would like to give you a bi-monthly project status update!

The precached project is now nearly two weeks in existence.
We achieved much in this short time span, but there is still a long way ahead of
us, of course.

## What has been achieved

* Prototype implementation of a Linux daemon, written in the Rust programming language
* Capable of caching selected files from a whitelist by calling `mmap()`, and subsequently `mlock()` on them
* Comprised of plugins and hooks layered on top of a core daemon proper
* Many new plugins and hooks that are really looking very promising, but are still in their infancy right now
* A slew of other new features (unstable)
* Packaging for Fedora Linux using the Copr infrastructure

## List of new and noteworthy plugins:

* Static whitelist: Lock files into memory, based on a static list of file names
* Dynamic whitelisting: Scan `/proc/$pid/maps` and add the most often mapped files to a dynamic whitelist
* VFS statx() cache: Prime the kernel's dentry caches by walking and stat()ing files and directories
* System metrics plugin: Signals changes in memory pressure, and paging status (swapping)
* Beginnings of a DBUS interface to control the daemon

Note: There exists a bunch of other new plugins that have been omitted here,
since they are still in a conceptual phase right now and it is not guaranteed
that they will work out at all (see below). You may want to take a look at the
project source code repository at [github.com](https://github.com/X3n0m0rph59/precached/tree/master/src),
to see the current state of plugins and new plugin concept ideas!

## What didn't quite work out

The idea to write a ptrace() based process I/O tracer plugin didn't work out so
well in the end! It just is too slow to ptrace every newly created process,
because it incurs at least two additional thread context switches per trapped
system call. This puts so much load and latency on the system that the expected
speed gain of file prefetching will be diminished by a huge amount.
We will go down the ftrace route now...

The precached team

## Article Updates

This posting has been edited at: Sat Oct 7 04:25:00

* Added link to github.com
* Fixed typos
* Rewordings
