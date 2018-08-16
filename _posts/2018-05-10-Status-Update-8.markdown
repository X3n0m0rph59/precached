---
layout: post
title: "Project Status Update #8"
date:   2018-05-10 11:20:00
tags: status update
---

# Project Status Update No. 8

Welcome to the precached project status update No. 8!

In this (long) cycle we mainly concentrated on optimizing performance on
very old machines and on squashing bugs throughout the whole project.

## What has been achieved

We added two new companion tools called `precached-trigger` as well as
`precached-debug`. The tool `precached-debug` currently supports the creation
and subsequent access of multiple files which may be used to verify that the
`precached` daemon correctly traces file access patterns.
The new tool `precached-trigger` can be used by unprivileged users to trigger
certain predefined actions in the `precached` daemon. This is needed to support
auto-starting of the offline prefetch as soon as the GDM (Gnome Display Manager)
shows the password prompt, or when the user has successfully logged in to
his or her desktop.

Shell completions for all companion tools are now auto-generated during the
build process.

## What didn't quite work out

We needed to revert the change to set the scheduling parameters of the prefetcher
threads to a realtime scheduling class (SCHED_RR). It caused livelocks on
single core systems and therefor didn't help performance.

## List of new and noteworthy plugins

* Triggers: Support triggers (perform certain predefined actions in precached) by unprivileged users

The release of version 1.2.0 is imminent, so stay tuned...


The precached team

## Article Updates

This posting has been edited at: Sat May 12 13:20:00

* Rewording
* Clarifications
