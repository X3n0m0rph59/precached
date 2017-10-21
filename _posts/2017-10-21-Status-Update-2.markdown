---
title: "Project Status Update #2"
date:   2017-10-21 03:50:00
categories: status update
---

# Project Status Update No. 2

Welcome to the precached project status update No. 2!
Roughly two weeks have passed since the last status update has been published,
and there happened a lot of new feature development in the meantime.

## What has been achieved

The last two weeks largely revolved around the implementation of the I/O tracing
and prefetching subsystems. Both are now in a working albeit experimental state.
The precached daemon gained the ability to perform syscall tracing of other
processes' I/O activity, utilising the Linux ftrace subsystem. It logs the
performed syscalls to an "I/O trace log" file, and later replays this pre-recorded
actions to prime the system's caches. In theory, this may improve the responsiveness
of your computer by no longer making the workload I/O bound but making it CPU bound
instead, thereby eliminating much of the perceived latency (slowness). The trace
log replay may either occur on exec()-time of a process (online prefetching),
or it may occur ahead of time while the system is idle. Online prefetching is done
by the "iotrace prefetcher" plugin, which spawns threads with a soft realtime
scheduling policy and a high priority, one thread per cpu core in your system.
If a process calls fork() and subsequently exec(), we immediately commence online
prefetching of the files that will eventually be accessed by that newly created
process. For this to achieve, the previously logged entries in the trace files
are loaded, and then spread equally between the multiple prefetcher threads,
which then perform the "heavy lifting" of faulting the referenced data in.
The goal here is to outpace the newly started process and reading all
the data in _before_ it is actually needed.
We also do support a preliminary form of offline prefetching by listing the path
of the program binaries of which dependent files shall be kept in memory in the
`/etc/precached/precached.conf` file. Precached will then try to keep all files
in memory, that are referenced by the I/O trace log of that program.
```
program_whitelist = [
 "/usr/lib64/libreoffice/program/soffice.bin",
]
```
This snippet will keep LibreOffice (the binary and all dependent files)
cached in ram

There are two newly written companion executables:
  * iotracectl - manage I/O trace log files
  * precachedctl - manage the daemon process

I/O trace log files may be managed by the `iotracectl` tool. It currently
supports these subcommands:
`$ iotracectl --help`
`status          Show the current status of the precached I/O tracing subsystem

 top             Top/htop like display of in-flight I/O traces

 list            List all available I/O traces

 info            Print metadata information of specific I/O traces

 dump            Dump I/O trace log entries (file access operations)

 analyze         Analyze I/O trace logs (check for missing files)

 optimize        Optimize I/O trace logs (optimize access patterns)

 remove          Remove I/O trace

 clear           Completely clear all I/O traces and reset the precached I/O tracing subsystem

 help            Display this short help text

 test-tracing    Test the I/O tracing subsystem of precached
 `

![iotracectl list](/precached/images/iotracectl_01.png)

![iotracectl list](/precached/images/iotracectl_02.png)

![iotracectl analyze](/precached/images/iotracectl_03.png)

## List of new and noteworthy plugins:
  * ftrace logger: Log I/O syscalls of processes to an I/O trace log file
  * iotrace prefetcher: Online prefetching of files during startup of a program
  * static whitelist (extended): Offline prefetching of files when the system is idle
  * system metrics

In a first round of testing, it has been shown that online prefetching has a
moderate effect on application startup times. Offline prefetching yielded a much
higher improvement.

Stay tuned for some first benchmarks coming soon!

In the next weeks we will work on stabilizing the new features, and possibly
implement offline prefetching of application files using a prefetcher based on
markov chains that will supplement the "manual whitelisting" approach.

The precached team


## Article Updates

This posting has been edited at: Sat Oct 21 04:00:00

* Fixed typos
* Rewordings
