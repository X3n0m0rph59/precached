---
title: "Project Status Update #7"
date:   2018-01-15 10:30:00
categories: status update
---

# Project Status Update No. 7

Welcome to the precached project status update No. 7!

In this cycle we mainly concentrated on extending the new rule engine
and on fixing bugs throughout the whole project.

## What has been achieved

The companion tool `iotracectl` learned the new paramater `--flags` that
may be used on the subcommands `filter`, `show` and `remove`.
This new parameter allows to filter for a specific flag state.

Flags may be one of:

* Valid, meaning that the I/O trace is valid and will be used by the precached prefetcher
* Invalid, meaning that the I/O trace is invalid and will not be used by the precached prefetcher
* Fresh, meaning that the I/O trace is younger than the expiry time
* Expired, meaning that the I/O trace is old and expired, though it maybe still useful (precached will re-trace the associated executable on next execution)
* Current, meaning that the I/O trace is younger than the modification time of the corresponding binary
* Outdated, the binary that has been traced got updated (mtime newer), making the trace irrelevant (precached will re-trace the associated executable on next execution)

Besides that, we extended the rule engine, which is now able to catch the following events:

* Noop: Does nothing at all, just a placeholder
* Ping: Internal `Ping` event; may be used as a timer or liveness check
* UserLogin: This event gets fired when precached notices that a user has logged in on the system

New and noteworthy actions are:

* Noop: Does nothing at all, just a placeholder
* Log: Log a message to syslog, the text can be given in the `Message:` argument
* Notify: Send a desktop notification to the first logged in user
* CacheDirRecursive: Recursively statx() files in the directory given in `Directory:` argument

In addition the rule engine learned to expand variables and macros in
all strings given to it.

E.g. the following rule stanza

```
# Event-Name		  Filter		  Action		  Arguments
UserLogin		  Noop              Log                 Severity:Warn,Message:"$date User $user logged in! ($home_dir)"
UserLogin               Noop              CacheDirRecursive   Directory:"$home_dir/.gnome"
```

will be processed and finally be expanded to this equivalent:

```
# Event-Name		  Filter		  Action		  Arguments
UserLogin		  Noop              Log                 Severity:Warn,Message:"2018-01-15 08:59 User admin logged in! (/home/admin)"
UserLogin               Noop              CacheDirRecursive   Directory:"/home/admin/.gnome"
```

## List of new and noteworthy plugins

* Janitor: A janitor for precached that handles all house keeping tasks
* Rule Engine: Learned a few new features like variable and macro expansion (see above)
* Rule Event Bridge: Serves as a gateway between precached internals and the rule engine
* User Session: The caching related code, mainly the caching-of-directories-on-user-login feature has been superseeded by a .rule file


The precached team
