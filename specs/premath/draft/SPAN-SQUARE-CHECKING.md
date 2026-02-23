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
  identity,
- composition laws encode identity/associativity/interchange behavior for
  composed span/square witnesses.

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

## 4. Composition Law Surface (Bicategory Profile)

Composition law witnesses are OPTIONAL. When present, they MUST be checked
deterministically.

### 4.1 `compositionLaws`

`artifacts.spanSquare.compositionLaws` MUST be an object with:

- `identitySpanIds` (string array, optional; defaults to empty),
- `identitySquareIds` (string array, optional; defaults to empty),
- `laws` (non-empty array of `CompositionLaw` rows).

### 4.2 `CompositionLaw`

A composition-law row MUST include:

- `id` (non-empty string, unique within `laws`),
- `kind` (`span|square`),
- `law` (non-empty string),
- `left` (expression object),
- `right` (expression object),
- `result` (`accepted|rejected`),
- `failureClasses` (string array, deterministic order after canonicalization),
- `digest` (non-empty string).

Allowed `law` values:

- span laws:
  - `span_identity`
  - `span_associativity`
- square laws:
  - `square_identity`
  - `square_associativity_horizontal`
  - `square_associativity_vertical`
  - `square_hv_compatibility`
  - `square_interchange`

When `compositionLaws` is present, `laws` MUST include at least one accepted
row for each allowed `law` value above.

### 4.3 Expression forms

Span expressions:

- atomic: `{"span":"<span-id>"}`,
- composition:
  - `{"compose":{"left":<SpanExpr>,"right":<SpanExpr>}}`.

Square expressions:

- atomic: `{"square":"<square-id>"}`,
- composition:
  - `{"compose":{"mode":"horizontal|vertical","left":<SquareExpr>,"right":<SquareExpr>}}`.

### 4.4 Law digest binding

Law witness digest is bound to canonical law fields:

```text
LawCore {
  kind, law, left, right, result, failureClasses
}
digest = "sqlw1_" + SHA256(JCS(LawCore))
```

Implementations MUST reject when declared digest does not match canonical
digest.

### 4.5 Composition semantics

For each `CompositionLaw`:

- `result = accepted` MUST imply:
  - `failureClasses` is empty,
  - normalized(`left`) == normalized(`right`).
- `result = rejected` MUST imply:
  - `failureClasses` is non-empty.

Normalization model:

- Span expressions normalize by flattening composition and removing
  `identitySpanIds`.
- Square expressions normalize to a rectangular token grid:
  - horizontal composition concatenates columns row-wise,
  - vertical composition concatenates rows,
  - composition MUST reject on shape mismatch,
  - `identitySquareIds` are neutral for both composition modes.

Checker implementations MUST evaluate square compositions through both modes so
horizontal/vertical compatibility and interchange rows are deterministic.

## 5. Coherence Integration

`premath coherence-check` integrates this surface via obligation id:

- `span_square_commutation`

Executable vectors are carried in the site fixture root
(`tests/conformance/fixtures/coherence-site`).

## 6. Deterministic Failure Class

Violation class:

- `coherence.span_square_commutation.violation`

For fixed contract bytes + repository state, failure class emission MUST be
deterministic.
