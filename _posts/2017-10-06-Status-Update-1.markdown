---
title: "Project Status Update #1"
date:   2017-10-06 12:59:00
categories: status update
---

# Project Status Update

From this day on, we would like to give you a bi-monthly project status update!

The precached project is now nearly two weeks in existence.
We achieved much in this short timespan, but there is still a long way ahead of us, of course.

## What has been achieved

* Prototype implementation of a Linux daemon, written in the Rust programming language
* Capable of whitelisting files that it should mmap(), and subsequently mlock()
* Comprised of plugins and hooks layered on top of a small core daemon
* Many new plugins and hooks that really look promising but are still in their infancy right now
* A slew of unstable new features
* Packaging for Fedora Linux using the Copr infrastructure

## List of new and noteworthy plugins:

* Static whitelist: lock files into memory, based on a static list of file names
* Dynamic whitelisting: Scan `/proc/$pid/maps` and add the most often mapped files to a `dynamic whitelist`
* VFS statx() cache: Prime the kernel's dentry caches by walking and stat()ing files and directories
* System metrics plugin: Signals memory pressure changes and paging status (swapping)
* Beginnings of a DBUS interface to control the daemon

Note: There are a bunch of other new plugins that have been omitted here, since they are in a conceptual 
phase right now and it is not said that all will work out (see below). You may want to take a look at the 
project sources, to see the current state and new plugin concept ideas!

## What didn't quite work out

The idea to write a ptrace() based I/O tracer plugin didn't work out so well in the end!
It just is too slow because it incurs at least two additional thread context switches per trapped system call.
This puts so much load on the system that the expected speed gain of prefetching will be diminished 
by a huge amount. We will go down the ftrace route now...

The precached team
