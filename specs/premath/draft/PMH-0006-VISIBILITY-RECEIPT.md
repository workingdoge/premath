---
slug: draft
shortname: PMH-0006-VISIBILITY-RECEIPT
title: workingdoge.com/premath/PMH-0006-VISIBILITY-RECEIPT
name: Premath Meta-Harness VisibilityReceipt
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - visibility
  - receipt
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`VisibilityReceipt` records archive access decisions for learning and evaluated
actors. It is the anti-leakage evidence object.

## 2. Shape

```text
VisibilityReceipt :=
  Sigma {
    id                  : CID
    run_id              : CID
    task_id             : String
    actor               : VisibilityActor
    artifact_ref        : CID
    artifact_path_label : String
    access_kind         : list | read | write | execute
    decision            : allowed | denied
    allowed_by_ref      : Optional CID
    denied_by_ref       : Optional CID
    trace_event_ref     : CID
    logical_time        : Integer
  }
```

```text
VisibilityActor :=
  harness
| model
| proposer
| grader
| runner
```

## 3. Denied access

Denied access attempts MUST produce both a `TraceEvent` and a
`VisibilityReceipt`. Denied access is evidence, not absence.

## 4. Derivable no-forbidden-read claim

A system SHOULD be able to derive:

```text
NoForbiddenRead(actor, sealed_scope)
```

from visibility receipts and the active visibility law.

## 5. Kernel validation reads

Internal kernel reads for validation MAY be separately audited. They are not
the leakage boundary unless they expose archive material to a learning or
evaluated actor.
