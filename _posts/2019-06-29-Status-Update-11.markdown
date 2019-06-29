---
layout: post
title: "Project Status Update #11"
date:   2019-06-29 09:00:00
tags: status update
---

# Project Status Update No. 11

Welcome to the precached project status update No. 11!

It has been quiet for a while around precached, but now we are back with some 
exciting news!
In this cycle we could improve the efficiency of precached by a large amount.

Precached now works on locked-down Linux systems, that means we now support 
Fedora 30 with Secure Boot enabled!

## What has been achieved

The `ftrace` based tracer has been replaced by the new `fanotify` based tracer.
This leads to huge improvements in some areas:
* Precached now works on systems with lockdown-mode enabled (e.g.: Fedora 30 with Secure Boot)
* Reduced CPU-load, compared to the `ftrace` based tracer
* We are guaranteed to not miss any filesystem events, even on high system-load
    * This will reduce re-generation of altered trace logs

Additionally, all external dependencies have been updated to their latest versions.

### New `fanotify` based tracing

`Fanotify` is a recent Linux API that is intended to be used to watch for 
filesystem events. It is much more efficient than using `ftrace`, since 
it does not require extensive parsing of log messages.

The precached team
