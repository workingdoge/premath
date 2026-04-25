---
slug: draft
shortname: PMH-0004-TRACE-EVENT
title: workingdoge.com/premath/PMH-0004-TRACE-EVENT
name: Premath Meta-Harness TraceEvent
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - trace
  - evidence
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`TraceEvent` is a kernel-owned observation of run behavior. It is evidence
about what happened; it is not self-authored by the evaluated candidate.

## 2. Shape

```text
TraceEvent :=
  Sigma {
    id              : CID
    run_id          : CID
    task_id         : String
    candidate_id    : CID
    parent_event_id : Optional CID
    logical_time    : Integer
    actor           : TraceActor
    kind            : TraceEventKind
    input_ref       : Optional CID
    output_ref      : Optional CID
    policy_refs     : List CID
  }
```

```text
TraceActor :=
  harness
| model
| archive_api
| grader
| runner
| kernel
```

Minimum v0 event kinds:

```text
task_start
| model_call
| model_output
| archive_list
| file_read_attempt
| file_read
| evidence_select
| answer_submit
| grader_call
| grader_result
| error
| timeout
| task_end
```

## 3. Author boundary

No `TraceEvent` MAY be authored by the evaluated candidate. The candidate MAY
perform a mediated action that causes the kernel observer to append a
`TraceEvent`.

## 4. Coherence

A `TraceBundle` becomes admissible evidence only after the declared trace
coherence checker accepts it or emits a typed obstruction.
