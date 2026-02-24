---
slug: draft
shortname: CHANGE-INF
title: workingdoge.com/premath/CHANGE-INF
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

### 3.1 Composition

ChangeRecords compose when the target of one equals the source of the next:

```text
compose(f1: ChangeRecord, f2: ChangeRecord) → ChangeRecord
```

Precondition: `f1.toRef = f2.fromRef` (the records are composable in the
change category).

Construction:

- `fromRef = f1.fromRef`, `toRef = f2.toRef`.
- `contextMapRef = f2.contextMapRef ∘ f1.contextMapRef`.
- `totalMapRef = f2.totalMapRef ∘ f1.totalMapRef`.
- `preservationClaims = intersection(f1.preservationClaims,
  f2.preservationClaims)`.
- `morphismKind`: if both components share the same kind, inherit; otherwise
  `"mixed"`.
- `commutationCheck`: `"accepted"` only when the composed square commutes
  (see §4.1).

Associativity: `compose(compose(f1, f2), f3) = compose(f1, compose(f2, f3))`
holds because map composition is associative and `fromRef`/`toRef` threading
is transitive.

Identity: the identity ChangeRecord for a state `s` is:

- `fromRef = toRef = digest(s)`,
- `contextMapRef = totalMapRef = id`,
- `mutations = []` (empty mutation list),
- `commutationCheck = "accepted"`,
- `preservationClaims` = all defined claim IDs.

The identity is a two-sided unit: `compose(id_s, f) = f = compose(f, id_t)`
for any `f: s → t`.

### 3.2 Change category

For a fixed `concernId`, ChangeRecords form a category **Change(C)**:

- **Objects**: spec states, identified by digests of the normative source.
  Each object is a snapshot of the canonical source referenced by
  `normativeRef` in the ConcernMap.
- **Morphisms**: ChangeRecords from one state to another. A morphism
  `f: s → t` has `fromRef = digest(s)` and `toRef = digest(t)`.
- **Composition**: via §3.1.
- **Identity**: via §3.1 (empty mutation list, identity maps).

The functor `g: Change(C) → State(C)` that applies changes to states MUST be
functorial:

- `g(id_s) = id_{g(s)}` (applying the identity change is a no-op),
- `g(f2 ∘ f1) = g(f2) ∘ g(f1)` (applying a composed change equals applying
  components sequentially).

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

### 4.1 Fibred commutation under composition

The commuting-square rule extends to composed changes. For a composition
`f2 ∘ f1`:

1. **Decomposition requirement.** The witness for the composed square MUST
   decompose into witnesses for `f1` and `f2` individually. That is, if the
   composed square commutes, each component square MUST also commute.

2. **Glue condition.** The composed witness is the glue of the component
   witnesses. It exists when:
   - both component squares commute independently,
   - the intermediate projections agree:
     `f1.projectionAfter = f2.projectionBefore` (up to declared equality).

3. **Obstruction.** Composition commutation can fail even when components
   commute individually. This occurs when `f2` references structure that `f1`
   removes or modifies incompatibly. For example:
   - `f1` removes a node, `f2` adds an edge to that node.
   - `f1` changes a morphism set on an edge, `f2` assumes the old set.

   Such obstructions are glue failures — the component witnesses exist but
   cannot be composed. This is a descent condition on the change fibre.

### 4.2 Change descent

A cover of a composed change by component changes satisfies descent iff:

1. **Component acceptance.** Each component has `commutationCheck = "accepted"`.

2. **Glue existence.** The composition witness exists — applying the composed
   change produces the same result as applying components sequentially (per
   §3.1 associativity and §3.2 functoriality of `g`).

3. **Contractibility.** The glue is contractible — there is exactly one way
   to factor the composed witness into component witnesses. When multiple
   decompositions produce the same composed result, they MUST be equivalent.

4. **No obstruction class.** None of the obstruction conditions from §4.1
   fire.

When descent fails, the composed change MUST be rejected. The failure class
SHOULD indicate whether the failure is a component rejection (individual
square fails) or a glue obstruction (components pass but composition fails).

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

### 7.1 Composition rejection conditions

In addition to the single-change rejections in §7, an implementation MUST
reject a composed change record when any of the following holds:

- **Dangling reference after composition.** A mutation in a later component
  references an object removed or never created by a prior component. For
  example: `f2` adds an edge to a node that `f1` removes.

- **Ambiguous composition.** Overlapping mutations on the same object across
  components produce non-deterministic results. For example: both `f1` and
  `f2` contain `UpdateCover` on the same cover ID with different parts, and
  no ordering discipline resolves the conflict.

- **Non-associative composition.** Order-dependent mutations that violate
  associativity. For example: three mutations whose result changes depending
  on grouping — `compose(compose(f1, f2), f3) ≠ compose(f1, compose(f2, f3))`.
  This SHOULD NOT occur when mutations are applied sequentially on
  well-typed state, but implementations MUST detect and reject it if it does.

- **Glue obstruction.** Sequential application of components produces a
  different final state than application of the composed mutation list from
  the first component's `fromRef` state. That is:
  `apply(apply(s, f1), f2).digest ≠ apply(s, f2 ∘ f1).digest`.

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
