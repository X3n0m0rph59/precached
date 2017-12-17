---
title: "Project Status Update #5"
date:   2017-12-17 10:47:00
categories: status update
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
  * Get in-flight tracer data of currently traced processes
  * Get the current status of the prefetcher threads
  * Request and drain ringbuffer of `precached` daemon's internal events
  * Get global statistics gathered by the daemon

The new `preachedtop` application periodically issues one or more of the above
listed requests to the `precached` daemon, and then displays the returned data.

## User Interface of `precachedtop`

Since `precachedtop` is still under heavy development, the UI is subject to
change.

![precachedtop](/precached/images/precachedtop_01.png)

![precachedtop](/precached/images/precachedtop_02.png)

![precachedtop](/precached/images/precachedtop_03.png)

![precachedtop](/precached/images/precachedtop_04.png)

The precached team
