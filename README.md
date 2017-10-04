# Precached - A Linux process monitor and pre-caching daemon
[![Build Status](https://travis-ci.org/X3n0m0rph59/precached.svg?branch=master)](https://travis-ci.org/X3n0m0rph59/precached) [![Package Status](https://copr.fedorainfracloud.org/coprs/x3n0m0rph59/precached/package/precached/status_image/last_build.png)](https://copr.fedorainfracloud.org/coprs/x3n0m0rph59/precached/package/precached/)

Precached is written in Rust and utilises the Linux netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. in the future it will be able to pre-fault pages into
memory to speed up loading of programs and increase the perceived overall
'snappiness' of the system.

### Quick Install Guide

#### Install on Fedora

```
    $ sudo dnf copr enable x3n0m0rph59/precached
    $ sudo dnf install precached
    $ sudo systemctl enable --now precached
```

#### Install From Source

```
    $ git clone https://github.com/X3n0m0rph59/precached.git  
    $ cd precached/
    $ cargo build --release
```

### Notes

This project is still in a very early stage of development and you may
possibly encounter some serious bugs.

### Why You may want to use precached

Precached tries to tackle some of the long standing performance issues
of the Linux desktop:

* The system has unused (free) memory directly after bootup. Therefor programs
  take a longer time to start up, cache cold start is way slower than cache hot
  start
* A Cron-Job evicts many important pages from the page cache. The system feels
  sluggish afterwards and won't recover for a long time
* The system has unused (free) memory after a "memory hog" process quit.
  The system reacts sluggish until the caches are primed again

### Use precached if

You have a reasonably fast CPU and a slow disk drive (and an ample
amount of RAM) installed in your system, then you may see a performance
improvement by using precached. The larger the speed difference between the
CPU (fast) and the I/O subsystem (slow), the more you gain by running precached.

### Don't use precached if

* You have a fast NVME SSD drive
* You have less than ~2GBs of RAM installed
  (and want to use a modern Linux desktop)

If any of the above is true for your system, then you aren't likely to get a
noticeable improvement out of using precached.

### Benchmarks

* Still need to be done

### Current State

#### What is working right now

* mlock() of mapped files
* Static Whitelisting
* VFS statx() caching (pre-read file metadata)

#### What remains to be done

* Implement ceiling on mlocked() memory
* Possibly implement fork-bomb mitigation
* Implement a persistence layer
* Daemonization support
* Prime caches on daemon startup
* Markov-chain based prefetching
* Implement a DBUS interface
* Write a nice CLI tool to control the daemon
* And write a precached GUI in GTK+
* Create and publish benchmarks
* ...

### Getting Involved

We are actively looking for contributions! Besides from code contributions,
we could especially well use:
* Project logo designs
* Text translations
* Documentation authors
* Issue triaging

***So please feel free to participate!***

If you are new to Open Source software, you may want to read
[How to Contribute](https://opensource.guide/how-to-contribute/)

### Website

[Project Website](https://x3n0m0rph59.github.io/precached/)

### Authors

precached - Copyright (C) 2017 the precached developers
