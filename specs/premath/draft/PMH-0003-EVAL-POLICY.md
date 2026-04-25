---
slug: draft
shortname: PMH-0003-EVAL-POLICY
title: workingdoge.com/premath/PMH-0003-EVAL-POLICY
name: Premath Meta-Harness EvalPolicy
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - eval
  - visibility
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`EvalPolicy` defines the task set, grading law, visibility law, metric order,
and failure policy under which a candidate may produce admissible evidence.

## 2. Shape

```text
EvalPolicy :=
  Sigma {
    id               : CID
    benchmark_ref    : CID
    split            : search | dev | test
    task_set_ref     : CID
    grader_refs      : List CID
    visibility_law_ref: CID
    metric_order_ref : CID
    required_outputs : List RequiredOutput
    failure_policy   : FailurePolicy
  }
```

```text
RequiredOutput :=
  answer_nf
| trace_bundle
| visibility_receipts
| score_bundle
| outcome_bundle
| eval_receipt
```

```text
FailurePolicy :=
  Sigma {
    emit_obstruction_on : List FailureClass
  }
```

Minimum failure classes:

```text
forbidden_read
| missing_trace
| missing_visibility_receipt
| invalid_answer_schema
| grader_error
| timeout
```

## 3. Outcome priority

Transcript quality MAY be measured, but PMH evaluation is outcome-first. A
valid result MUST bind the final outcome bundle and receipt validity, not only
the model transcript.

## 4. Visibility law

The `visibility_law_ref` defines which artifacts the evaluated actor or
learning actor may list, read, write, or execute. Archive access that is
allowed or denied under this law MUST produce a `VisibilityReceipt`.

## 5. Metric order

`metric_order_ref` defines how scores, costs, latency, side effects,
obstructions, and tie breakers are ordered for scoped comparison. The metric
order is part of every `ImprovementClaim`.
