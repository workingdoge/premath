---
slug: raw
shortname: SEMANTICS-INFTOPOS
title: workingdoge.com/premath/SEMANTICS-INFTOPOS
name: Premath∞ Semantics (Model in an ∞-Topos)
status: raw
category: Informational
tags:
  - premath
  - kernel
  - semantics
  - infty-topos
  - sheaves
  - hyperdescent
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

This document is **informational**. It provides a reviewer-facing, model-theoretic
semantics for the Premath kernel.

It does **not** change the normative requirements of:

- `draft/PREMATH-KERNEL` (kernel doctrine),
- `draft/GATE` (admissibility laws),
- `draft/BIDIR-DESCENT` (operational model), or
- `draft/REF-BINDING` (backend-generic commitment boundary).

The intent is to show that Premath’s axioms are exactly the “stack/sheaf laws”
that are native in higher topos theory, while preserving Premath’s original
kernel intention: **a criterion/filter on definability**, not a commitment to a
single foundational ontology.

## 2. Ambient setting

Fix:

- an ∞-category of contexts `C` with pullbacks, and
- a Grothendieck topology `J` on `C`.

Let:

- `Sh(C,J)` denote the ∞-topos of sheaves of spaces on `(C,J)`.
- `Sh^∧(C,J)` denote its hypercompletion (optional).

A *Premath∞ semantics* chooses its ambient coherence level as:

- `V := S_∞` (spaces / ∞-groupoids),

and interprets sameness `≈` as equivalence in `V`.

Truncations are available by restricting to `n`-truncated objects in `Sh(C,J)`.

## 3. Definables as a sheaf object

A definability assignment in this semantics is an object:

- `Def ∈ Sh(C,J)` (or `Def ∈ Sh^∧(C,J)` if hyperdescent is desired).

For each context `Γ ∈ C`, the fiber `Def(Γ)` is a space.

Reindexing along `f: Γ' → Γ` is induced by functoriality:

- `f^*: Def(Γ) → Def(Γ')`.

All higher unit/associativity coherences required by `draft/PREMATH-KERNEL` are
carried natively by the ∞-categorical structure.

## 4. Descent and contractible gluing

For a cover `U ⊳ Γ` in the topology `J`, define the ∞-groupoid of descent data
`Desc_U(Γ)` as the limit of the usual Čech nerve diagram evaluated in spaces.

There is a canonical restriction map:

- `res_U: Def(Γ) → Desc_U(Γ)`.

If `Def` is a sheaf (i.e. `Def ∈ Sh(C,J)`), then `res_U` is an equivalence.

Equivalently, for every descent datum `d ∈ Desc_U(Γ)`, the homotopy fiber
`fib_d(res_U)` is contractible.

This is exactly Premath’s contractible gluing axiom.

## 5. Hyperdescent (optional strengthening)

If `Def` lies in the hypercompletion `Sh^∧(C,J)`, then `Def` satisfies hyperdescent:
restriction along any `J`-hypercover yields an equivalence.

Premath kernels MAY adopt hyperdescent as an optional capability; see
`raw/HYPERDESCENT`.

## 6. What this semantics does and does not claim

- This semantics shows Premath’s kernel axioms are *native* “stack/sheaf laws”
  in an ∞-topos.
- It does not imply Premath requires a topos-theoretic foundation.
- It does not change the backend-generic commitment boundary: operational kernels
  remain free to compile canonical bytes into different commitment schemes via
  `draft/REF-BINDING`.

