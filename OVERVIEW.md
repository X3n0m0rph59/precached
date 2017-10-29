# Precached - A Linux process monitor and pre-caching daemon

## Project Overview

* What problem does it try to solve

### Basic Memory Management Concepts

* Virtual Memory
* Address Space
* Memory mappings
* Kernel Page Cache

### Current State

* Precached is still in an early stage of development

### Software Architecture

* Describe how it is implemented
* Plugin based architecture
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
