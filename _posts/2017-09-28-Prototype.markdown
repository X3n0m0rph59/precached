---
title: "First Prototype released"
date:   2017-09-28 09:00:00
categories: update release
---

# First Prototype released

We just released the first working prototype!
It is currently able to monitor fork()/execve() events and scan process file mappings.
Valid mappings are subsequently mlocked() to the memory of the precached process.

## TODO List (non-exhaustive):
	* Implement ceiling on mlocked() memory
	* Persistence layer
	* Systemd init scripts
	* Daemonization
	* Configuration support (/etc/precached)
	* And much more...
