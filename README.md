# Precached - A Linux process monitor and pre-caching daemon

[![Build Status](https://travis-ci.org/X3n0m0rph59/precached.svg?branch=master)](https://travis-ci.org/X3n0m0rph59/precached)

Precached is written in Rust and utilizes the Linux Netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. it is able to pre-fault pages into memory, to speed up
loading of programs and increase the perceived overall 'snappiness' of the
system. Additionally it supports offline prefetching of the most often used
programs while the system is idle.

## Quick Installation Guide

### Install on ArcoLinux/Manjaro or other Arch Linux based Distros

```shell
    $ yay -Sy precached
```

### Install on Fedora

```shell
    $ sudo dnf copr enable x3n0m0rph59/precached
    $ sudo dnf install precached
```

### Install on Ubuntu

```shell
    $ sudo add-apt-repository ppa:x3n0m0rph59/precached
    $ sudo apt update && sudo apt install precached
```

### Install from Source

```shell
    $ git clone https://gitlab.com/X3n0m0rph59/precached.git
    $ cd precached/
    $ cargo build --release

    # ... copy files ...
```

### Enable service autostart

```shell
    $ sudo systemctl enable --now precached.service
    $ sudo systemctl enable --now precached-prime-caches.timer
    $ systemctl --user enable --now precached-trigger.service
```

## Why You may want to use precached

Precached tries to tackle some of the long standing performance issues
of the Linux desktop:

* The system has unused (free) memory directly after boot-up. Therefore programs
  take a longer time to start up, cache cold start is way slower than cache hot
  start
* A Cron-Job evicts many important pages from the page cache. The system feels
  sluggish afterwards and won't recover for a long time
* The system has unused (free) memory after a "memory hog" process quit.
  The system reacts sluggish until the caches are primed again

## What is my RAM doing?

You can read about Linux's memory management here: https://www.linuxatemyram.com/index.html

## Use precached if

You have a reasonably fast CPU and a slow disk drive (and an ample
amount of RAM) installed in your system, then you may see a performance
improvement by using precached. The larger the speed difference between the
CPU (fast) and the I/O subsystem (slow), the more you gain by running precached.

### Only marginal improvements by using precached if

* You have a fast NVMe SSD drive
* You have less than ~2GBs of RAM installed
  (and want to use a modern Linux desktop)

If any of the above is true for your system, then you aren't likely to get a
noticeable improvement out of using precached.

## Benchmark Results

The preliminary benchmarks that we took are looking very promising already.
We used the current development version of precached (as of 2017-10-30) and
ran some benchmarks on it:

The benchmarks confirmed that precached is able to speed up load times of
mid-sized and huge applications on Linux. We nearly achieve cache hot load
times on first run (after system boot-up) of applications like e.g. LibreOffice
and Firefox. After a memory hog process exited we need a short amount of idle
time (roughly ~30 secs) to re-prime the caches, after that we achieve
cache hot load times again. We did not discover any corner cases at which
the system performed significantly worse than without precached running.
The system feels much more ‘snappier’ even directly after login.
Boot-up is slowed down somewhat though, since we read approximately 2GB of
additional data into the RAM cache (in our test setup).
This happens with a raised nice level and mostly during the GDM greeter’s
password prompt and thereafter. We are investigating the possibility to
move the offline prefetch phase somewhat further into the startup process
to achieve even faster boot times (delayed prefetching).

## Current State

### What remains to be done

* Possibly implement fork-bomb mitigation
* Markov-chain based prefetching
* ...

### Getting Involved

We are actively looking for contributions! Besides from code contributions,
we could especially well use:

* Text translations
* Documentation authors
* Issue triaging

***So please feel free to participate!***

If you are new to Open Source software, you may want to read
[How to Contribute](https://opensource.guide/how-to-contribute/)

### Website

[Project Website](https://x3n0m0rph59.gitlab.io/precached/)

### Authors

precached - Copyright (C) 2017-2020 the precached developers
