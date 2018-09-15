---
layout: post
title: "Project Status Update #10"
date:   2018-09-15 08:15:00
tags: status update
---

# Project Status Update No. 10

Welcome to the precached project status update No. 10!

In this cycle we mainly concentrated on further stabilizing the project and on
preparing the 1.3.x and 1.4.0 releases. A critical logic error, that lead to
offline prefetching not working correctly, has been identified and subsequently
been fixed.

## What has been achieved

All external dependencies have been updated to their latest versions. We added
a new dynamic blacklisting feature to exclude I/O trace logs from prefetching.

### Dynamic blacklisting for I/O trace logs

The companion tool `iotracectl` learned to blacklist I/O trace logs. This
feature may be used in addition to the static blacklisting (via
`/etc/precached/precached.conf`). The semantics of dynamically blacklisted
I/O trace logs are the following:

* Blacklisted I/O trace logs exist alongside the "active" I/O trace logs
* They just have the `blacklisted` flag set
* Blacklisted I/O trace logs are excempt from online- and offline prefetching
* They retain their I/O trace information for immediate reuse, in case the are
  un-blacklisted
* Invalidated but blacklisted I/O trace logs will no longer be updated (re-traced)
* Expired but blacklisted I/O trace logs will be kept (not garbage collected), 
  unlike expired "active" I/O trace logs

The precached team
