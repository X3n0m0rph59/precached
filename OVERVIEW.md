# Precached - A Linux process monitor and pre-caching daemon

### Project Overview

* What problem does it try to solve

### Current State

#### What is working right now

* mlock()

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

### Software Architecture

* Describe how it is implemented
* Hooks vs. Plugins

### Design Considerations

* Choice of programming language

### Technical Background

* Virtual memory
* Shared mappings
* mmap() and mlock() syscalls
* madvise() syscall
* Linux page cache and fs buffer cache
* dentry cache
* ```# echo 3 > /proc/sys/vm_drop_caches" ```

### Benchmark Results

* Speedup percentage
  * System boot
  * Startup of LibreOffice
  * ...

### Related Software Projects

There are other projects tackling this problem domain

* [preload by Behdad Esfahbod](http://behdad.org/download/preload.pdf)
* [memlockd by Russel Coker](https://doc.coker.com.au/projects/memlockd/)
* [vmtouch by Hoytech](https://hoytech.com/vmtouch/)

### Further reading

* LWN has some great articles
