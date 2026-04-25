---
slug: draft
shortname: PMH-0008-IMPROVEMENT-CLAIM
title: workingdoge.com/premath/PMH-0008-IMPROVEMENT-CLAIM
name: Premath Meta-Harness ImprovementClaim
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - improvement
  - claim
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`ImprovementClaim` is the scoped comparison object for harness specimens.
Improvement is not a global adjective.

## 2. Shape

```text
ImprovementClaim :=
  Sigma {
    id                   : CID
    candidate_ref        : CID
    baseline_ref         : CID
    scope                : ImprovementScope
    metric_order_ref     : CID
    evidence_receipt_refs: List CID
    conclusion           : improves | regresses | mixed | inconclusive
    confidence           : descriptive_only | weak | moderate | strong
  }
```

```text
ImprovementScope :=
  Sigma {
    benchmark_ref       : CID
    split               : search | dev | test | heldout_model | production_shadow
    task_set_ref        : CID
    model_profile_ref   : CID
    runtime_closure_ref : CID
    eval_policy_ref     : CID
  }
```

## 3. Evidence

Every improvement claim MUST cite one or more `EvalReceipt` objects. A claim
MUST NOT cite raw scores without the receipts that bind them.

## 4. V0 confidence

V0 fixture comparisons SHOULD use:

```text
confidence: descriptive_only
```

because a tiny benchmark is not sufficient evidence for generalization.
