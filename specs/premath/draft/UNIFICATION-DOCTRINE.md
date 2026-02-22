---
slug: draft
shortname: UNIFICATION-DOCTRINE
title: workingdoge.com/premath/UNIFICATION-DOCTRINE
name: Minimum Encoding, Maximum Expressiveness Doctrine
status: draft
category: Standards Track
tags:
  - premath
  - doctrine
  - architecture
  - unification
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

This doctrine defines the architectural rule for Premath evolution:

- minimum canonical encoding at authority boundaries,
- maximum expressiveness via typed projections, overlays, and capability claims.

It applies to instruction/proposal/checking, issue memory, conformance surfaces,
and interop artifacts.

## 2. Core Principle

For any semantic boundary `B`, implementations MUST prefer:

1. one canonical representation at `B`,
2. many deterministic views derived from that representation.

Premath systems SHOULD add expressiveness by adding projections and obligation
routes, not by forking canonical encodings.

## 3. Canonical Boundary Rules

### 3.1 Single authority encoding

Each authority boundary MUST define one canonical payload shape and one
deterministic identity function.

Examples:

- instruction proposals: canonical proposal payload + deterministic
  `proposalDigest`/`proposalKcirRef`,
- issue memory: `issue.event.v1` append-only substrate + deterministic replay.

### 3.2 Derived view discipline

Derived views MUST be deterministic projections of canonical payloads.

Derived views MAY optimize for workflow semantics (execution, GTD, groupoid,
profile overlays), but MUST NOT introduce independent semantic authority.

### 3.3 Binding discipline

Any normalized/evidence-producing route MUST carry deterministic binding
material:

- `normalizerId`,
- `policyDigest`,
- canonical refs where applicable (for example `kcir1_*`, `cmp1_*`, `ev1_*`,
  `iss1_*`).

## 4. Expressiveness Without Forks

Expressiveness SHOULD be introduced by:

- capability-scoped overlays,
- obligation compilation/discharge hints,
- additional deterministic projections,
- richer witness annotations.

Expressiveness MUST NOT be introduced by:

- parallel canonical schemas for the same authority boundary,
- implicit authority in planner/proposal outputs,
- unverifiable side-channel state.

## 5. Migration Rules

When replacing or tightening a boundary representation:

1. implementations SHOULD provide deterministic projection/replay between old
   and new surfaces,
2. compatibility aliases MAY exist temporarily,
3. canonical authority MUST move to one boundary before compatibility aliases
   are removed.

Compatibility fields (for example digest aliases) MUST stay bound to the same
canonical payload while they coexist.

## 6. Conformance Expectations

Implementations following this doctrine SHOULD:

- expose deterministic witness lineage from canonical payload to final verdict,
- fail closed on unknown/unbound classifications at authority boundaries,
- run doctrine/traceability/coherence checks in merge-gated command surfaces.

## 7. Relationship to Other Specs

This doctrine constrains how existing specs compose:

- `draft/SPEC-INDEX` (normative scope and claims),
- `draft/LLM-INSTRUCTION-DOCTRINE` and `draft/LLM-PROPOSAL-CHECKING`
  (checking-mode authority split),
- `draft/PREMATH-COHERENCE` (cross-surface parity obligations),
- `draft/CHANGE-MORPHISMS` (deterministic change projections),
- `draft/KCIR-CORE`, `draft/NF`, `draft/NORMALIZER` (interop identity surfaces).
