---
slug: profile
shortname: ADJOINTS-AND-SITES
title: workingdoge.com/premath/ADJOINTS-AND-SITES
name: Adjoints and Sites Profile (Normative)
status: draft
category: Standards Track
tags:
  - premath
  - profile
  - site
  - adjoint
  - descent
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

## 0. Purpose

This profile specifies a minimal, checkable core for Premath based on:

- a Grothendieck site of contexts,
- an indexed doctrine (fibration) of definables over that site,
- a capability-scoped adjoint triple on a declared class of maps.

All operational behavior MUST compile into finite obligations discharged by a
deterministic normalizer bound to `(normalizerId, policyDigest)`.

This profile is an additive overlay on `draft/PREMATH-KERNEL`,
`draft/GATE`, and `draft/BIDIR-DESCENT`. It does not change the kernel
accept/reject criterion or Gate failure-class vocabulary.

## 1. Base site of contexts

### 1.1 Context category

Let `C` be a category:

- objects: contexts `Gamma`
- morphisms: context maps `f : Gamma' -> Gamma`

### 1.2 Coverage (Grothendieck topology)

A cover of `Gamma` is a finite family `{u_i : Gamma_i -> Gamma}`. The set of
covers defines a Grothendieck topology `J` on `C`.

A conforming profile implementation MUST support:

- identity cover: `{id_Gamma}`
- pullback stability of covers: if `{u_i}` covers `Gamma` and `f: Gamma'->Gamma`,
  then the pullback family covers `Gamma'`
- transitivity: refining covers composes

### 1.3 Refinement

A refinement is a morphism of covers `U <= V` (implementation-specific
encoding), inducing a canonical comparison of Cech data.

The checker MUST ensure: refinement does not change meaning after discharge (see
Section 5.4).

## 2. Doctrine: definables as an indexed structure

Let `p : E -> C` be a fibration (or equivalent indexed family) where:

- fiber `E_Gamma` is the space of definables over `Gamma`
- every map `f : Gamma'->Gamma` induces reindexing
  `f* : E_Gamma -> E_Gamma'`

The implementation MUST satisfy functoriality up to definitional equality in the
chosen normal form:

- `(id_Gamma)* = id`
- `(f o g)* = g* o f*`

## 3. Adjoint triple as a capability (not a global axiom)

### 3.1 Admissible maps

Define a predicate/classifier `Admissible(f)` meaning the adjoint triple exists
for `f`.

`Admissible(f)` MUST be decidable by a stable allowlist keyed by
`policyDigest` (for example map kinds: `projection`, `cover_inclusion`,
`forgetful_adapter`).

### 3.2 Adjoint triple

For `Admissible(f : Gamma'->Gamma)`, provide:

- `Sigma_f -| f* -| Pi_f`

Operational meaning:

- `f*` = transport/restriction
- `Sigma_f` = aggregation/dependent sum
- `Pi_f` = quantification/dependent product

### 3.3 Laws (unit/counit)

For each admissible `f`, the checker MUST be able to demand evidence of triangle
identities in the chosen equality notion:

- `Sigma_f -| f*`: unit `eta : id -> f*Sigma_f`, counit
  `epsilon : Sigma_f f* -> id`
- `f* -| Pi_f`: unit `eta' : id -> Pi_f f*`, counit
  `epsilon' : f*Pi_f -> id`

Evidence is not required to be an explicit proof object. Evidence MAY be
discharged by deterministic normalization to `cmpRef` (Section 7).

## 4. Coherence: Beck-Chevalley (required)

For any pullback square in `C`:

```
Gamma'' --g'--> Gamma'
  |              |
  f'             f
  |              |
  v              v
Delta  --g-->   Gamma
```

with `f` and `f'` admissible (and required structure existing), the checker MUST
support Beck-Chevalley obligations:

- `Sigma-BC`: `Sigma_f' o (g')*  ~=  g* o Sigma_f`
- `Pi-BC`: `Pi_f' o (g')*  ~=  g* o Pi_f`

where `~=` is this profile's equality, discharged via `cmpRef`.

Optional and off by default extensions include Frobenius reciprocity and
monoidal coherence.

## 5. Descent and contractibility (site laws as obligations)

### 5.1 Descent (existence)

Given a cover `{u_i : Gamma_i->Gamma}` and compatible local data in fibers
`E_Gamma_i`, the checker MUST be able to require a gluing candidate in `E_Gamma`.

### 5.2 Contractibility (uniqueness)

If two gluings exist for the same descent data, the checker MUST be able to
require a canonical identification (uniqueness up to chosen equality).

### 5.3 Stability

Reindexing MUST preserve meaning. Stability obligations connect `f*` with
restriction along covers and refinements.

### 5.4 Refinement invariance

If `U <= V` is a refinement, discharge MUST produce equal `cmpRef` results for
meanings computed via `U` versus `V` (or emit a witness).

## 6. Compilation to obligations (checker contract)

All semantic requirements above MUST compile to finite, typed obligations.
Minimum obligation kinds:

- `stability`
- `locality`
- `descent_exists`
- `descent_contractible`
- `adjoint_triangle` (Sigma/f*/Pi units and counits)
- `beck_chevalley_sigma`
- `beck_chevalley_pi`
- `refinement_invariance`

The checker MUST:

- emit obligations in checking mode (goal-directed)
- NEVER treat a proposal (human or LLM) as authoritative without discharge

For failure-class mapping and witness ordering at the Gate boundary, use
`draft/BIDIR-DESCENT` and `draft/GATE`.

## 7. Deterministic discharge and identity binding

### 7.1 Binding

Every discharge MUST be bound to:

- `normalizerId` (algorithm/version)
- `policyDigest` (profile + allowlist + rewrite policy)

### 7.2 Comparison key

Discharge produces a canonical comparison reference `cmpRef`.

Two claims are equal if and only if their discharged `cmpRef` are equal under
the same `(normalizerId, policyDigest)`.

### 7.3 Witnesses

On failure, the system MUST output a witness object whose fields include:

- obligation kind
- inputs (refs and digests)
- `(normalizerId, policyDigest)`
- mismatch evidence (for example both cmpRefs, differing normalized traces,
  missing pullback, non-unique glue)

## 8. LLM interaction (non-authority rule)

LLM outputs MUST be treated as untrusted proposals that can:

- suggest admissible maps, covers, glue candidates, and obligation discharge
  strategies
- but MUST NOT introduce new admissible map kinds, new laws, or bypass
  discharge

LLM proposals live entirely in checking mode.

## 9. Implementation checklist (dev)

- Encode site `C` and cover structure `J` with pullback/refinement operations.
- Implement `Admissible(f)` allowlist keyed by `policyDigest`.
- Represent reindexing `f*` and, where admissible, `Sigma_f` and `Pi_f`.
- Emit obligations for triangles, Beck-Chevalley, descent/contractibility, and
  refinement invariance.
- Implement deterministic normalizer producing `cmpRef` bound to
  `(normalizerId, policyDigest)`.
- Standardize witness schema for all failed obligations.
