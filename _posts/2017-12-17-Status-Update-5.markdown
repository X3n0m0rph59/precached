---
layout: post
title: "Project Status Update #5"
date:   2017-12-17 10:47:00
tags: status update
---

# Project Status Update No. 5

Welcome to the precached project status update No. 5!
In this cycle we mainly concentrated our work on creating a new companion tool
named `precachedtop`. This tool will hopefully be useful to inspect the
internal workings of `precached` and may be used to uncover hidden bugs.
It follows the design principles of other top-like tools like e.g. `htop`.

## What has been achieved

We created an UNIX domain sockets based IPC mechanism in the `precached`
daemon that implements a simple request/response protocol.

The following requests are currently implemented:
* Get a list of currently traced processes
* Get in-flight tracer data of currently traced processes
* Get the current status of the prefetcher threads
* Request and drain ringbuffer of `precached` daemon's internal events
* Get global statistics gathered by the daemon

The new `precachedtop` application periodically issues one or more of the above
mentioned requests to the `precached` daemon, and then displays the returned
data.

## User Interface of `precachedtop`

Since `precachedtop` is still under heavy development, the UI is subject to
change.

![precachedtop](/precached/images/precachedtop_01.png)

The precached team

## Article Updates

This posting has been edited at: Thu Dec 21 09:01:00

* Rewording
* Fixed typos
* Publish
