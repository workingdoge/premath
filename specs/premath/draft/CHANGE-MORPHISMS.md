---
slug: draft
shortname: CHANGE-MORPHISMS
title: workingdoge.com/premath/CHANGE-MORPHISMS
name: Change Morphisms and Canonical Concern Mapping
status: draft
category: Standards Track
tags:
  - premath
  - conformance
  - change-management
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Purpose

This specification defines a canonical change discipline so ecosystem evolution
remains runtime-invariant:

- one canonical normative source per concern,
- one canonical change record shape,
- fibred change morphism checks (commuting-square discipline),
- explicit preservation claims checked by vectors.

This document is normative when capability
`capabilities.change_morphisms` is claimed.

## 2. Canonical concern mapping

Implementations claiming this capability MUST maintain a **ConcernMap** where
each concern has exactly one normative source.

### 2.1 Concern map entry

Each entry MUST contain:

- `concernId: string` (stable identifier),
- `normativeRef: string` (single canonical source reference),
- `projectionRefs: list<string>` (optional non-normative projections).

Rules:

- `concernId` MUST be unique in a map.
- `normativeRef` MUST be singular (no multi-source authority).
- projection refs MUST NOT be treated as normative authority.

## 3. Change record shape

A change is represented by a **ChangeRecord**:

```text
ChangeRecord {
  schema: 1,
  changeId: string,
  concernId: string,
  fromRef: string,
  toRef: string,
  projectionBefore: string,   // p0_before
  projectionAfter: string,    // p0_after
  contextMapRef: string,      // f
  totalMapRef: string,        // h
  morphismKind: "vertical" | "horizontal" | "mixed",
  commutationCheck: "accepted" | "rejected",
  preservationClaims: list<PreservationClaimId>,
  witnessRefs: list<string>
}
```

`changeId` SHOULD be deterministic from canonical serialization of the record
excluding `witnessRefs` and other debug-only payloads.

## 4. Fibred commuting-square rule

For each change record, implementation MUST check:

`projectionAfter ∘ totalMap == contextMap ∘ projectionBefore`

where:

- `projectionBefore` is `p0_before: E_before -> C_before`,
- `projectionAfter` is `p0_after: E_after -> C_after`,
- `contextMap` is `f: C_before -> C_after`,
- `totalMap` is `h: E_before -> E_after`.

`commutationCheck` MUST be:

- `accepted` only when the square commutes under declared check semantics,
- `rejected` otherwise.

## 5. Morphism kind classification

Classification rules:

- `vertical`: base/context map is identity up to declared equality mode.
- `horizontal`: total map and base map both change while preserving projection law.
- `mixed`: other accepted commuting changes.

Classification MUST be deterministic for fixed inputs.

## 6. Preservation claims

`preservationClaims` is a declared set of preserved semantics.

Defined claim IDs:

- `kernel_verdict_invariant`
- `gate_class_invariant`
- `policy_binding_invariant`
- `witness_schema_compatible`

When `commutationCheck = accepted`, at least
`kernel_verdict_invariant` and `gate_class_invariant` MUST be declared for
semantic/runtime concerns.

## 7. Rejection conditions

An implementation MUST reject a change record when any of the following holds:

- `concernId` has no unique canonical `normativeRef`,
- `concernId` is missing from concern map,
- commuting-square check fails,
- `morphismKind` classification is invalid for declared maps,
- required preservation claims are absent.

## 8. Conformance linkage

Vector requirements for this capability are defined in:

- `draft/CAPABILITY-VECTORS` (`capabilities.change_morphisms`)

Conformance profile requirements are defined in:

- `draft/CONFORMANCE`

## 9. Non-goals

This document does not prescribe:

- branch or forge policy,
- specific CI vendor integration,
- specific proof object formats for witness internals.

It prescribes only authority, shape, and invariance constraints for changes.
