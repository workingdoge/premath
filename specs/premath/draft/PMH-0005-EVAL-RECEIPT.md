---
slug: draft
shortname: PMH-0005-EVAL-RECEIPT
title: workingdoge.com/premath/PMH-0005-EVAL-RECEIPT
name: Premath Meta-Harness EvalReceipt
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - eval
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

`EvalReceipt` is the authoritative fact of an evaluation run. Scores are
subordinate to it.

## 2. Shape

```text
EvalReceipt :=
  Sigma {
    id                   : CID
    run_id               : CID
    candidate_id         : CID
    harness_spec_ref     : CID
    runtime_closure_ref  : CID
    eval_policy_ref      : CID
    task_set_ref         : CID
    trace_bundle_ref     : CID
    visibility_bundle_ref: CID
    score_bundle_ref     : CID
    outcome_bundle_ref   : CID
    result_nf            : EvalResultNF
    obstruction_refs     : List CID
    verifier_digest      : SHA256
    created_at           : Timestamp
  }
```

```text
EvalResultNF :=
  Sigma {
    status           : valid | invalid | partial | quarantined
    primary_score    : Number
    secondary_scores : Map String Number
    cost             : CostProfile
  }
```

## 3. Score subordination

A score bundle MUST NOT be interpreted as a result unless it is referenced by a
valid `EvalReceipt`.

## 4. Invalid and partial receipts

An invalid or partial run MAY still have an `EvalReceipt` when the receipt
faithfully binds the obstruction and invalidity facts. Such receipts are
informative, but they are not admissible improvement evidence unless the
`EvalPolicy` explicitly permits that obstruction class for the scoped claim.
