# Precached - A Linux process monitor and pre-caching daemon
[![Build Status](https://travis-ci.org/X3n0m0rph59/precached.svg?branch=master)](https://travis-ci.org/X3n0m0rph59/precached)

Precached is written in Rust and utilises the Linux netlink connector interface
to monitor the system for process events. It can act upon such events via
multiple means. E.g. in the future it will be able to pre-fault pages into
memory to speed up loading of programs and increase the
perceived snappiness of the system.

### Quick install guide
    $ git clone https://github.com/X3n0m0rph59/precached.git  
    $ cd precached/
    $ cargo build

### Notes
This project is in a very early stage of development and you may
possibly encounter serious bugs.

### Current State

#### What is working right now

* mlock() of mapped files

#### What remains to be done 

* Implement ceiling on mlocked() memory
* Implement VFS statx() caching (pre-read file metadata)
* Possibly implement fork-bomb mitigation
* Implement a persistence layer
* Prime caches on daemon startup
* Daemonization support
* Implement a DBUS interface
* Write a nice CLI tool to control the daemon
* And write a precached GUI in GTK+
* ...

### Authors
precached - Copyright (C) 2017 the precached developers
