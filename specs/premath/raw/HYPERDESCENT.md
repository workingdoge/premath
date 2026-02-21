---
slug: raw
shortname: HYPERDESCENT
title: workingdoge.com/premath/HYPERDESCENT
name: Hyperdescent Extension (Optional Capability)
status: raw
category: Standards Track
tags:
  - premath
  - kernel
  - gate
  - hyperdescent
  - hypercovers
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

This document defines an **optional capability** that strengthens the Premath
kernel and Gate descent requirements from Čech descent on covers to **hyperdescent
on hypercovers**.

This capability is OPTIONAL. Not claiming it MUST NOT weaken conformance to the
base Premath kernel.


## 2. Capability name

Capability identifier:

- `hyperdescent`

An implementation that claims `hyperdescent` MUST enforce the additional laws in
this document.

## 3. Hypercovers (informal definition)

A `J`-hypercover of a context `Γ` is a simplicial object `U_• → Γ` such that:

- `U_0 → Γ` is a `J`-cover, and
- for each `n>0`, `U_n → M_n(U)` is a `J`-cover of the matching object `M_n(U)`.

Implementations MAY represent hypercovers in any deterministic format.
This specification does not mandate a specific encoding.

## 4. Hyperdescent law (normative)

Let `Def : C^{op} → V` be the definables assignment (as in `draft/PREMATH-KERNEL`).

For any `J`-hypercover `U_• → Γ`, define the ∞-groupoid of hyperdescent data as:

- `Desc_{U_•}(Γ) := lim_{[n]∈Δ} Def(U_n)`

where the limit is taken in the ambient coherence level `V`.

There is a canonical restriction map:

- `res_{U_•} : Def(Γ) → Desc_{U_•}(Γ)`.

**Hyperdescent requirement:** if the `hyperdescent` capability is claimed, then
for every `J`-hypercover `U_• → Γ`, `res_{U_•}` MUST be an equivalence in `V`.

Equivalently, the homotopy fiber over any hyperdescent datum MUST be contractible.

## 5. Relationship to base Premath descent

- Base Premath requires contractible descent for `J`-covers (Čech descent).
- Hyperdescent implies Čech descent (so the capability strictly strengthens the kernel).

## 6. Operational obligations (BIDIR-DESCENT)

Implementations claiming `hyperdescent` MUST:

1. extend obligation generation to include hyperdescent obligations when evaluating
   in `normalized` mode, and
2. map failures deterministically to Gate failure classes:

- missing hyperglue existence -> `descent_failure` (`GATE-3.3`)
- non-contractible hyperglue -> `glue_non_contractible` (`GATE-3.4`)

Implementations MAY choose any deterministic strategy to select which hypercovers
are checked (e.g. bounded dimension, policy-selected generators), but MUST bind
that strategy into `policyDigest` when operating in `normalized` mode.

## 7. Conformance vectors (informative)

Recommended vectors for the `hyperdescent` capability:

- a case where Čech descent holds but hyperdescent fails (adversarial),
- a case where both hold (golden),
- determinism of witness IDs and ordering.

