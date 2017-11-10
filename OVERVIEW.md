# Precached - A Linux process monitor and pre-caching daemon

This document will give you a brief introduction of the precached project.

## Project Overview

Precached is a Linux system daemon that is written in the Rust programming 
language. It tries to improve system performance by loading data that will 
most likely be referenced soon from slow mass storage (e.g. hard disks) 
into RAM. Generally speaking: precached tries to keep the most often used 
programs in the in-memory cache (page cache). Additionally precached is able 
to readahead data that is *not currently cached*, when a process is executed 
(online prefetching).

The precached daemon records which files are accessed on application startup 
by using the Linux ftrace subsystem, and later uses the generated I/O trace 
log to dynamically load the accessed files into the systems page cache.
Thus when using precached, the most often used applications are "cache hot" 
most of the time, and will load much faster compared to their "cache cold" 
startup times.

### Current State

Precached is in an early stage of development. You may encounter some 
serious bugs.

The basic functionality should be working though.

### Software Architecture

As said before, precached is written in the excellent programming language 
called "Rust". We believe that the choice of programming language 
contributes to the overall stability and security of a software system.

The precached daemon is comprised of a small core which only implements
the most basic functionality. This core is then extended by "plugins" and 
"hooks", which deliver the actual features like e.g. generation of I/O 
trace logs or prefetching of data into RAM.

The precached daemon makes use of a multithreaded architecture design and 
tries to utilise the available cpu cores as efficiently as possible.
It spins up multiple threads:

  * precached main thread - Coordinates the other threads
  * event loop - Listens for procmon events and delivers them to the main thread
  * ftrace - This thread processes the ftrace event stream
  * worker (4) - Thread pool that executes background tasks of lower priority
  * prefetch (NCPUs) - Thread pool that is used to asynchronously read data from slow mass storage devices into ram


#### Design Considerations

We chose a plugin based software architecture for precached to be able to 
easily extend its functionality in the future.

#### Available Plugins and Hooks

The following plugins are available for precached (as of 2017-11-10)

  * I/O Trace Log Manager (stable) - Manage I/O trace logs
  * Markov Log Manager (not implemented/in development)
  * Hot Applications (stable) - Offline prefetching of the most often used applications
  * Metrics (stable) - Generate system metrics and deliver events based on them
  * Statistics (stable) - Generate statistics using data from Metrics plugin
  * Notifications (stable) - Desktop notifications using D-BUS
  * Custom Rules (not implemented/in development)
  * System Agent (not implemented/in development)
  * VFS Stat(x) Cache (stable) - Prime the kernel’s dentry caches by walking directories and stat()ing files
  * Static Blacklist (stable) - Blacklist files that shall not be accessed by the precached daemon
  * Static Whitelist (stable) - Force caching of files or applications into memory
  * ftrace Messages (stable) - Insert custom messages into the Linux ftrace subsystems event stream
  * Fork Bomb Mitigation (not implemented/in development)

The following hooks are available for precached (as of 2017-11-10)

  * ftrace logger (experimental) - Generate I/O trace logs by utilising the Linux ftrace subsystem
  * ptrace logger (deprecated) - Generate I/O trace logs by ptrace()ing processes and trapping system calls
  * Fork Bomb detector (not implemented/in development)
  * I/O Trace Prefetcher (in development) - Prefetch files using a previously recorded I/O trace log
  * Markov Prefetcher (not implemented/in development)
  * Process Tracker (stable) - Track fork() and exec() of processes running on the system
  * Rule Hook (not implemented/in development)

### Benchmark Results

The preliminary benchmarks that we took are looking very promising already.
We used the current development version of precached (as of 2017-10-30) and 
ran some benchmarks on it:

The benchmarks confirmed that precached is able to speed up load times of 
mid-sized and huge applications on Linux. We nearly achieve cache hot load 
times on first run after system bootup of applications like e.g. LibreOffice 
and Firefox. After a memory hog process exited we need a short amount of idle 
time (roughly ~30 secs) to re-prime the caches, after that we achieve
cache hot load times again. We did not discover any corner cases at which 
the system performed significantly worse than without precached running.
The system feels much more ‘snappier’ even directly after login.
Bootup is slowed down somewhat though, since we read approximately 2GB of 
additional data into the RAM cache. This happens with a raised nice level 
and mostly during the GDM greeter’s password prompt and thereafter. 
We are investigating the possibility to move the offline prefetch phase 
somewhat further into the startup process to achieve even faster boot times 
(delayed prefetching).

## Specifications of *System No. 1*:
```
  ~7 year old laptop running Fedora Core 27
  Dual Core Intel CPU
  4GB DDR3 RAM
  500GB HDD, BFQ I/O Scheduler
  Linux 4.14
```

|Application      |Stock|precached|Cache Hot|
|-----------------|----:|--------:|--------:|
|System Bootup    |1:30 |1:39     |n.a.     |
|LibreOffice Calc |19   |4        |4        |
|Firefox 57       |15   |4        |3.5      |

_Application load times on first run after bootup_


## Specifications of *System No. 2*:
```
  ~5 year old laptop running Fedora Core 27
  Quad Core Intel CPU
  4GB DDR3 RAM
  256GB SATA-3 fast SSD, CFQ I/O Scheduler
  Linux 4.14
```

|Application      |Stock|precached|Cache Hot|
|-----------------|----:|--------:|--------:|
|System Bootup    |28.5 |32       |n.a.     |
|LibreOffice Calc |6.5  |3        |3        |
|Firefox 57       |7    |3.5      |3        |

_Application load times on first run after bootup_

For further information please see the 
[precached project website](https://x3n0m0rph59.github.io/precached/).

### Related Software Projects

There are other projects tackling this problem domain

* [preload by Behdad Esfahbod](http://behdad.org/download/preload.pdf)
* [memlockd by Russel Coker](https://doc.coker.com.au/projects/memlockd/)
* [vmtouch by Hoytech](https://hoytech.com/vmtouch/)

### Further reading

* LWN has some great articles
