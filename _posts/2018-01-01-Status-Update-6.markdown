---
title: "Project Status Update #6"
date:   2018-01-01 17:30:00
categories: status update
---

# Project Status Update No. 6

Welcome to the precached project status update No. 6!
We wish you a happy new year 2018!

In this cycle we mainly implemented a rule based engine which can be used to
execute pre-defined actions when certain events are triggered.

## What has been achieved

We created a rule based matching engine that executes certain actions as soon
as the associated event is triggered. The rules subsystem can be managed by
the new companion tool `rulesctl`. Said rules are written in a very simple
declarative language. As a starting point, please find example .rules files
in the directory `/etc/precached/rules.d`.

Below some example .rules file excerpts:

This rule stanza will periodically log the text 'Ping' with severity 'warn'
to the syslog:
```
# Event-Name		  Filter		  Action		  Arguments
Ping		          Noop		    Log		      Severity:Warn 
```

The full .rules file including metadata looks like this:
```
# =============================================================================
!Version: 1.0
!Enabled: false
!Name: Ping Logger
!Description: A Simple Demo Rule
# =============================================================================
# =============================================================================
# Event-Name		    Filter		  Action		  Arguments
  Ping		          Noop		    Log		      Severity:Warn
# =============================================================================
```

The following line will send a desktop notification to the first logged in user
when a fork bomb has been detected:
```
# Event-Name		    Filter		  Action		  Arguments
ForkBombDetected    Noop        Notify      Noop
```

The full .rules file including metadata looks like this:
```
# =============================================================================
!Version: 1.0
!Enabled: true
!Name: Log Fork Bombs
!Description: Log Fork Bombs
# =============================================================================
# =============================================================================
# Event-Name		    Filter		  Action		  Arguments
  ForkBombDetected  Noop        Log         Severity:Warn
  ForkBombDetected  Noop        Notify      Noop
# =============================================================================
```

In the next cycle we plan to add a slew of new actions.

## User Interface of `rulesctl`

![rulesctl](/precached/images/rulesctl_01.png)

![rulesctl](/precached/images/rulesctl_02.png)

The precached team
