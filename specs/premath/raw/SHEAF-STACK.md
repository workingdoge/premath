---
slug: raw
shortname: SHEAF-STACK
title: workingdoge.com/premath/SHEAF-STACK
name: Presheaf, Sheaf, and Stack Contracts over Ctx
status: raw
category: Informational
tags:
  - premath
  - sheaf
  - stack
  - descent
  - naturality
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

This document gives an informational contract for presheaf/sheaf/stack
interpretations over `(Ctx, J)` (see `raw/CTX-SITE`).

Normative authority remains in:

- `draft/PREMATH-KERNEL`
- `draft/GATE`
- `draft/BIDIR-DESCENT`
- `draft/PREMATH-COHERENCE`

## 2. Presheaf contract

A context-indexed semantic object `F` is a presheaf when:

- each context `Gamma` has a section-space `F(Gamma)`,
- each refinement `rho: Gamma' -> Gamma` has restriction map
  `F(rho): F(Gamma) -> F(Gamma')`,
- reindexing is functorial:
  - `F(id) = id`
  - `F(rho o sigma) = F(sigma) o F(rho)`.

Operationally: every adapter/kernel output interpreted as context-indexed data
SHOULD admit this restriction shape.

## 3. Sheaf contract (set-level descent)

For cover `{Gamma_i -> Gamma}`:

- local sections `s_i in F(Gamma_i)` with overlap agreement
- MUST determine a unique global section `s in F(Gamma)`.

This is the strict sheaf reading of contractible descent.

## 4. Stack contract (witnessed descent)

For higher-coherence implementations, agreement is witness/equivalence data
rather than strict equality.

The stack reading is:

- local data + overlap equivalences + coherence witnesses,
- with a contractible glue space of global realizations.

Operationally this is the correct target when adapter overlap compatibility is
witness-bearing and not strictly syntactic-equality based.

## 5. Naturality contract

A construction `eta: F => G` is natural when for every
`rho: Gamma' -> Gamma`, the square commutes:

- `G(rho) o eta_Gamma = eta_Gamma' o F(rho)`.

Premath reading: transport-stable means base-change stable; no local ad hoc
choice is permitted if naturality is required.

## 6. Glue-or-witness boundary

Given a cover and local candidate data, checker behavior SHOULD be:

1. accept with a global glue and deterministic witness linkage, or
2. reject with deterministic obstruction/failure classes.

When mapped to current checker vocabulary, failures SHOULD reuse existing Gate
classes (for example:
`locality_failure`, `descent_failure`, `glue_non_contractible`) unless a new
class is strictly necessary.

## 7. Minimum-encoding rule

This document defines semantic shape, not a second execution authority.

- The checker decides admissibility.
- The sheaf/stack interpretation explains the meaning of those checks.
- Any implementation-specific evidence format remains a projection over the
  same obligation/discharge boundary.
