---
slug: raw
shortname: SPLIT-PRESENTATION
title: workingdoge.com/premath/SPLIT-PRESENTATION
name: Split Presentation and IR Boundary (Implementation Guidance)
status: raw
category: Best Current Practice
tags:
  - premath
  - kernel
  - implementation
  - strictification
  - ir
  - normalization
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
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** are to be
interpreted as described in RFC 2119 (and RFC 8174 for capitalization).

## 1. Scope

This document is **best current practice**. It describes one recommended way to
realize the Premath kernel operationally:

- represent semantics (up to `≈`) by a strict, computable intermediate
  representation (IR), and
- use a normalizer to obtain canonical representatives.

This document does not change the kernel (`draft/PREMATH-KERNEL`) or Gate (`draft/GATE`).

## 2. Semantic vs definitional equality

Premath distinguishes:

- `≈` : semantic sameness (equality / iso / equivalence) in the chosen coherence level, and
- `≡` : definitional/algorithmic equality inside an implementation.

A practical implementation MUST choose a definitional equality `≡` for its IR.

The purpose of normalization is to make:

- semantic equality testable by definitional equality of normal forms.

## 3. Split presentation

A common pattern is to present an abstract indexed semantics `Def : C^{op} → V` via a
strict structure with chosen pullbacks (“split cleavage”).

Informally:

- semantic reindexing `f^*` is coherent only up to `≈`,
- the implementation chooses canonical representatives so reindexing computes strictly.

This is what a bidirectional checker + normalizer is doing.

## 4. Relationship to this spec set

In this repository bundle:

- `draft/NF` provides a compact IR (ObjNF / MorNF bytes).
- `raw/OPCODES` provides the verification contracts that construct/transform those IR forms.
- `draft/NORMALIZER` specifies the canonicalization policy that turns IR into a comparison key.
- `draft/BIDIR-DESCENT` specifies how obligations are generated and discharged.

This combination is an instance of a split presentation:

- semantic equality (Premath `≈`) is checked by comparing normalized references (`cmpRef`) produced
  under a pinned `policyDigest`.

## 5. Recommendation

Implementations SHOULD:

1. treat IR byte grammars (`draft/NF`) as the definition of definitional equality (`≡`) after
   canonicalization,
2. treat semantic equivalence claims as obligations discharged in `normalized` mode via
   `draft/NORMALIZER`, and
3. record any policy/normalizer versions in emitted artifacts to prevent replay across changes.

