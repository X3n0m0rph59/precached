---
title: "Benchmarks, Round 1"
date:   2017-10-30 5:30:00
categories: status update benchmarks
---

# Benchmarks

The first round of benchmarks has arrived, finally! We used the current
development version of precached (as of 2017-10-30) and ran some benchmarks on
it:

The benchmarks confirmed that precached is able to speed up load times of
mid-sized and huge applications on Linux. We nearly achieve *cache hot* load
times *on first run after bootup* of applications like e.g. LibreOffice and Firefox.
After a memory hog process exited we need a short amount of idle time
(~30 secs on *System No. 2*) to re-prime the caches, after that we achieve
cache hot load times again. We did not discover any corner cases at which the
system performed significantly worse than without precached running.
The system feels much more 'snappier' even directly after login.
Bootup is slowed down somewhat though, since we read approximately 2GB of
additional data into the RAM cache. This happens with a raised nice level and
mostly during the GDM greeter's password prompt and thereafter. We are
investigating the possibility to move the offline prefetch phase somewhat further
into the startup process to achieve even faster boot times (delayed prefetching).

## Specifications of *System No. 1*:
```
  ~6 year old laptop running Fedora Core 27
  Dual Core Intel CPU
  4GB DDR3 RAM
  500GB HDD, BFQ I/O Scheduler
  Linux 4.14
```
|Application      |stock|precached|cache hot|
|:----------------|----:|--------:|--------:|
|System Bootup    |1:30 |1:39     |n.a.     |
|LibreOffice Calc |19   |4        |4        |
|Firefox 57       |15   |4        |3.5      |

_Application load times on first run after bootup_


## Specifications of *System No. 2*:
```
  ~3 year old laptop running Fedora Core 27
  Quad Core Intel CPU
  4GB DDR3 RAM
  256GB SATA-3 fast SSD, CFQ I/O Scheduler
  Linux 4.14
```
|Application      |stock|precached|cache hot|
|:----------------|----:|--------:|--------:|
|System Bootup    |28.5 |32       |n.a.     |
|LibreOffice Calc |6.5  |3        |3        |
|Firefox 57       |7    |3.5      |3        |

_Application load times on first run after bootup_


Stay tuned for more benchmarks to come!

## Article Updates

This posting has been edited at 2017-10-30:

* Fixed typos
* Rewordings
