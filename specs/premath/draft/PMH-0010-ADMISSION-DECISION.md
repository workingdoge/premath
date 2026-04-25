---
slug: draft
shortname: PMH-0010-ADMISSION-DECISION
title: workingdoge.com/premath/PMH-0010-ADMISSION-DECISION
name: Premath Meta-Harness AdmissionDecision
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - admission
  - decision
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`AdmissionDecision` is the kernel decision object for a proposed harness
mutation. No mutation executes merely because a learning actor wrote it.

## 2. Shape

```text
AdmissionDecision :=
  Sigma {
    id        : CID
    intent_ref: CID
    decision  : AdmissionVerdict
    checks    : AdmissionChecks
    created_at: Timestamp
  }
```

```text
AdmissionVerdict :=
  accepted
| rejected_type_error
| rejected_policy_violation
| rejected_leakage_risk
| rejected_non_reproducible
| rejected_authority_escalation
| needs_witness
| needs_smaller_eval
```

```text
AdmissionChecks :=
  Sigma {
    type_check           : CID
    policy_check         : CID
    leakage_check        : CID
    authority_check      : CID
    reproducibility_check: CID
  }
```

## 3. Kernel ownership

Only the kernel admission path MAY emit an `AdmissionDecision`. A learning actor
MAY cite evidence and request authority, but the request has no effect without
this decision object.

## 4. V0 status

This object is defined in v0 but SHOULD remain dormant until receipt-first
evaluation and archive mediation exist.
