---
slug: draft
shortname: PMH-0011-UNREPLAYABLE-RECEIPT
title: workingdoge.com/premath/PMH-0011-UNREPLAYABLE-RECEIPT
name: Premath Meta-Harness UnreplayableReceipt
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - replay
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

`UnreplayableReceipt` records explainable replay failure. A retired harness
must be replayable or explainably unreplayable.

## 2. Shape

```text
UnreplayableReceipt :=
  Sigma {
    id                  : CID
    subject_ref         : CID
    subject_kind        : harness | run | eval_receipt | runtime_closure
    attempted_replay_at : Timestamp
    replay_requested_by : CID
    reason              : UnreplayableReason
    missing_refs        : List CID
    evidence_refs       : List CID
    status              : UnreplayableStatus
    substitution_policy_ref: Optional CID
  }
```

```text
UnreplayableReason :=
  model_provider_unavailable
| model_weights_unavailable
| model_version_retired
| runtime_image_missing
| dependency_unresolvable
| verifier_unavailable
| benchmark_artifact_missing
| secret_unavailable
| policy_forbids_replay
| external_service_unavailable
| unknown
```

```text
UnreplayableStatus :=
  temporarily_unreplayable
| permanently_unreplayable
| replay_requires_substitution
```

## 3. Replay law

Replay failure MUST NOT erase the original receipt. It appends an
`UnreplayableReceipt` that explains why the original subject cannot currently
be replayed.

## 4. Substitution

If replay requires a substituted model, runtime, verifier, or benchmark
artifact, the substitution policy MUST be explicit before any substituted run
is compared with the original.
