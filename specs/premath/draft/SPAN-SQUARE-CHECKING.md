---
slug: draft
shortname: SPAN-SQUARE-CHECKING
title: workingdoge.com/premath/SPAN-SQUARE-CHECKING
name: Span and Square Witness Contract for Pipeline/Base-Change Commutation
status: draft
category: Standards Track
tags:
  - premath
  - coherence
  - spans
  - squares
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

## 1. Scope

This spec defines a minimum typed witness surface for making
pipeline/base-change commutation explicit:

- spans encode edge morphisms (`input <- apex -> output`),
- squares encode commutation claims over span edges with deterministic witness
  identity.

This surface is checker-facing. It does not introduce independent semantic
authority.

## 2. Typed Shapes

### 2.1 `SpanRef`

A span row MUST include:

- `id` (non-empty string),
- `kind` (non-empty string),
- `left` (JSON value),
- `apex` (JSON value),
- `right` (JSON value).

`id` MUST be unique within one artifact payload.

### 2.2 `SquareWitness`

A square row MUST include:

- `id` (non-empty string),
- `top` (span id),
- `bottom` (span id),
- `left` (span id),
- `right` (span id),
- `result` (`accepted|rejected`),
- `failureClasses` (string array, deterministic order after canonicalization),
- `digest` (non-empty string).

All edge references MUST resolve to declared span ids.

### 2.3 Digest binding

Square witness digest is bound to canonical core fields:

```text
SquareCore {
  top, bottom, left, right, result, failureClasses
}
digest = "sqw1_" + SHA256(JCS(SquareCore))
```

Implementations MUST reject when declared digest does not match canonical
digest.

## 3. Commutation Contract

For each `SquareWitness`:

- `result = accepted` MUST imply:
  - `failureClasses` is empty,
  - semantic digest(top span) == semantic digest(bottom span).
- `result = rejected` MUST imply:
  - `failureClasses` is non-empty.

These constraints are checker constraints. They do not authorize semantic gate
acceptance on their own.

## 4. Coherence Integration

`premath coherence-check` integrates this surface via obligation id:

- `span_square_commutation`

Executable vectors are carried in the site fixture root
(`tests/conformance/fixtures/coherence-site`).

## 5. Deterministic Failure Class

Violation class:

- `coherence.span_square_commutation.violation`

For fixed contract bytes + repository state, failure class emission MUST be
deterministic.
