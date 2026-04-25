---
slug: draft
shortname: PMH-0009-HARNESS-MUTATION-INTENT
title: workingdoge.com/premath/PMH-0009-HARNESS-MUTATION-INTENT
name: Premath Meta-Harness HarnessMutationIntent
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - mutation
  - intent
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

`HarnessMutationIntent` is the proposal object emitted by a learning actor. It
does not mutate authority state and does not authorize execution by itself.

## 2. Shape

```text
HarnessMutationIntent :=
  Sigma {
    id              : CID
    base_candidate  : CID
    proposer        : CID
    patch_ref       : CID
    rationale_ref   : CID
    expected_effect : ImprovementHypothesis
    cited_evidence  : List CID
    requested_authority: List CID
  }
```

## 3. Authority boundary

The proposer MAY propose intents. The proposer MUST NOT:

- append candidate records,
- alter eval policy,
- mutate the archive directly,
- write trace or visibility receipts,
- admit its own proposal,
- claim improvement without receipt evidence.

Mutation becomes executable only after an `AdmissionDecision` accepts it.

## 4. V0 status

This object is defined in v0 but SHOULD remain dormant until receipt-first
evaluation is real.
