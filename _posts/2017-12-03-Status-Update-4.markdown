---
layout: post
title: "Project Status Update #4"
date:   2017-12-03 08:47:00
categories: status update
---

# Project Status Update No. 4

Welcome to the precached project status update No. 4!
In this cycle, we drastically improved the companion tools.
They learned many new features (see below).

## What has been achieved

* Update and improve the companion tools `precachedctl` and `iotracectl`
* Add filtering and sorting feature to the companion tools
* Add shell completion support for bash and zsh for the companion tools
* Use local timezone instead of utc when displaying date and time values
* Implement caching of logged user's home directories
* Change thread priorities
* Fix and improve online prefetching
* Internal code refactoring to improve code quality and style

The companion tools learned to display date and time values in the local
timezone. We now use ASCII line drawing instead of unicode characters to
render the tables (for maximum compatibility). Unicode can be enabled via
a command line switch.
We refactored the internal logic of I/O trace file enumeration in the
companion tools, to fully support filtering and sorting at once.
The default sorting of `iotracectl` is now "sort by date ascending".
The oldest trace logs are displayed first.

![iotracectl](/precached/images/iotracectl_04.png)
List I/O trace logs of firefox, ordered by the size of prefetched data

![iotracectl](/precached/images/iotracectl_05.png)
List *new and not-optimized* trace logs

The tool `precachedctl` gained the ability to display the internal histogram
state written by the plugin `hot applications`.

![iotracectl](/precached/images/precachedctl_01.png)
List top 20 "hottest applications" on the system

## List of new and noteworthy plugins

* "User Session" - Metadata caching of logged users's home directories
* "I/O Trace Log Cache" - Cache .trace files to improve online prefetching

A new caching mechanism for the I/O trace files prevents, that I/O trace
logs get evicted from memory.
We now cache certain whitelisted directories in the user's home directory.
When we detect the login of a user we immediately start caching the
metadata. The metadata of whitelisted files of the user with id 1000
is cached ahead of time (before login).

The precached team

## Article Updates

This posting has been edited at: Sun Dec 03 08:59:00

* Added `precachedctl` section
* Rewordings
* Fixed typos
