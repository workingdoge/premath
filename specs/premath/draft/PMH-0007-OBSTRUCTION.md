---
slug: draft
shortname: PMH-0007-OBSTRUCTION
title: workingdoge.com/premath/PMH-0007-OBSTRUCTION
name: Premath Meta-Harness Obstruction
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - obstruction
  - failure
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`Obstruction` turns failed or invalid runs into reusable typed knowledge.

## 2. Shape

```text
Obstruction :=
  Sigma {
    id                    : CID
    run_id                : CID
    candidate_id          : CID
    task_id               : Optional String
    kind                  : ObstructionKind
    evidence_refs         : List CID
    local_trace_window_ref: Optional CID
    severity              : info | warning | blocking
    proposed_repair_ref   : Optional CID
  }
```

Minimum obstruction kinds:

```text
forbidden_read
| missing_visibility_receipt
| missing_trace_event
| answer_schema_violation
| citation_failure
| grader_failure
| runtime_failure
| trace_incoherence
| timeout
| confounded_edit
| authority_escalation
```

## 3. Invalidity

Every failed or invalid run MUST either emit an `Obstruction` or be marked
incomplete. Silent failure is non-conforming.

## 4. Confounded edits

When two or more patch components cannot be separated in the evidence, the
system SHOULD emit a confounded-edit obstruction rather than treating the
observed regression as a scalar score alone.
