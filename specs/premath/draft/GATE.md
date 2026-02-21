---
slug: draft
shortname: GATE
title: workingdoge.com/premath/GATE
name: Admissibility Gate
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - gate
  - admissibility
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

## 1. Overview

This specification defines the **admissibility gate** for Premath.

The Premath kernel (see `draft/PREMATH-KERNEL`) treats definability as behavior
under:

1. stability under context change (reindexing),
2. locality under covers,
3. descent/gluing with contractible uniqueness, and
4. coherence for context-change pushforwards (when advertised).

`draft/GATE` expresses these laws in a form suitable for deterministic verifier
implementations.

## 2. Constructor interface

A constructor is a tuple:

- `K = (C, Cov, Def, ~, Reindex, Sigma, Pi)`

where:

- `C` is a category of contexts.
- `Cov` is a coverage on `C`.
- `Def` is an indexed assignment of definables over contexts.
- `~` is the constructor's sameness relation on definables.
- `Reindex` gives pullback/reindexing maps along morphisms.
- `Sigma` and `Pi` are optional context-change pushforwards.

### 2.1 Context category

A conforming constructor MUST provide:

- objects `Gamma` (contexts),
- morphisms `f : Gamma' -> Gamma`,
- identity morphisms, and
- composable morphisms.

### 2.2 Coverage

For each context `Gamma`, `Cov(Gamma)` is a set of covering families
`U = {u_i : Gamma_i -> Gamma}`.

A conforming implementation MUST provide:

- cover membership checks,
- overlap projections for cover components, and
- cover pullback/restriction operations needed by descent checks.

### 2.3 Indexed definables

For each context `Gamma`, `Def(Gamma)` is the space/groupoid/category of
definables over `Gamma`.

A conforming implementation MUST provide:

- membership (or judgment) checks for `A in Def(Gamma)`,
- restriction/reindexing `f* : Def(Gamma) -> Def(Gamma')`.

### 2.4 Sameness relation

`~` MUST be an equivalence-compatible notion of sameness appropriate for the
constructor's level (equality, isomorphism, or higher equivalence).

All gate laws in this specification are stated **up to `~`** unless explicitly
noted otherwise.

## 3. Admissibility laws

A judgment `Gamma |- A` is Gate-valid iff all laws below hold.

### 3.1 Stability (functorial reindexing)

For every `f : Gamma' -> Gamma` and `g : Gamma'' -> Gamma'`:

(We use standard categorical composition order: `f o g` means "first g, then f".)

- identity law: `(id_Gamma)* A ~ A`
- composition law: `(f o g)* A ~ g*(f* A)`

An implementation MUST reject if either law fails for a claimed admissible
judgment.

### 3.2 Locality (cover restriction)

For every cover `U = {u_i : Gamma_i -> Gamma}` in `Cov(Gamma)`, each
restriction `u_i* A`
MUST exist.

An implementation MUST reject claims that cannot be restricted along a declared
cover.

### 3.3 Descent (gluing existence)

Given a cover `U` over `Gamma`, local definables `A_i in Def(Gamma_i)`, and
overlap compatibilities `phi_ij : p1* A_i ~ p2* A_j` satisfying cocycle
coherence, there MUST exist at least one global `A in Def(Gamma)` with compatible
restrictions.

### 3.4 Stack-safe uniqueness (contractible glue space)

Let `Glue(U; A_i, phi_ij)` denote the space/category/groupoid of global
glue candidates with compatibility witnesses.

For Gate-valid judgments, `Glue(U; A_i, phi_ij)` MUST be contractible (unique up
to unique `~`).

Set-level implementations MAY realize this as "exactly one result modulo
equality"; higher-level implementations MUST provide coherent uniqueness at the
appropriate level.

### 3.5 Adjoint triple coherence (Sigma/Pi)

When `Sigma_f` and `Pi_f` are provided for a morphism `f`, they MUST satisfy:

- `Sigma_f -| f* -| Pi_f` (up to `~` with explicit coherence data),
- Beck-Chevalley compatibility on pullback squares,
- descent-compatibility with gluing/restriction.

If an implementation advertises adjoint-triple support, failure of any required
coherence MUST be rejection.

If an implementation does **not** expose adjoint-triple structure, it MUST
declare this profile explicitly in conformance output.

## 4. Gate result and witnessing

A Gate check MUST produce one of:

- `accepted` with law witnesses, or
- `rejected` with at least one failing law class.

At minimum, rejection diagnostics MUST classify one of:

- `stability_failure`
- `locality_failure`
- `descent_failure`
- `glue_non_contractible`
- `adjoint_triple_coherence_failure`

### 4.1 Gate failure witness format (normative)

For rejected checks, implementations MUST emit a JSON document with:

- `witnessSchema` (integer schema version; current value `1`)
- `profile` (`"full"`)
- `result` (`"rejected"`)
- `failures` (array of failure witnesses)

Each failure witness MUST include:

- `witnessId` (stable deterministic id; MUST be computed per `draft/WITNESS-ID`)
- `class` (one of the failure classes above)
- `lawRef` (section reference, e.g. `GATE-3.1`)
- `message` (human-readable summary)

Each failure witness SHOULD include when available:

- `context` (serialized context key or structured map)
- `tokenPath` (affected token/definable path)
- `sources` (array of provenance records)
- `details` (class-specific machine-readable object)

Witness arrays MUST be deterministically ordered by:

1. `class`
2. `lawRef`
3. `tokenPath` (if present)
4. `context` (if present)
5. `witnessId`

## 5. Alignment with BIDIR-DESCENT (informative)

`draft/BIDIR-DESCENT` defines how implementations produce and discharge obligations and map them to Gate
failure classes deterministically.

## 6. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.profile.evidence` (verdict class and failure-class invariance only)
- `dm.policy.rebind` (requires explicit binding mismatch/rebind handling)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.profile.execution` (handled by runtime/CI layer)
- `dm.presentation.projection` (handled by projection layer)
