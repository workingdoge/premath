---
slug: raw
shortname: UNIVERSE
title: workingdoge.com/premath/UNIVERSE
name: Universe and Comprehension Extension (Optional)
status: raw
category: Standards Track
tags:
  - premath
  - kernel
  - universe
  - comprehension
  - tarski
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

This document defines an OPTIONAL extension that adds a **universe / smallness**
discipline and **comprehension** (context extension) on top of the Premath kernel.

The Premath kernel (`draft/PREMATH-KERNEL`) does not require universes, codes, or
comprehension structure.

This extension exists for settings that want:

- a Tarski-style “codes + decoding” view of definables,
- controlled size stratification,
- and a canonical notion of context extension `Γ.A`.

## 2. High-level intent

A universe extension adds:

1. a predicate/subassignment `Def_small(Γ) ⊂ Def(Γ)` of “small definables”, and
2. a classifier that represents small definables by maps into a universal context.

This is the semantic form of a Tarski universe. Operationally, it corresponds to
introducing **codes** and a **decode** operation.

## 3. Required additional structure

Let `(C, Cov, Def)` be a Premath world.

A universe extension MUST provide:

- an assignment `Def_small : C^{op} → V` together with a monomorphism-like inclusion
  `ι_Γ : Def_small(Γ) → Def(Γ)` (in the sense of the ambient `V`), and
- a context `U ∈ C` (“universe context”), and
- a “generic display map” `π : E → U` presented as a definable-over-`U`.

The intended classifier law is:

- for each `Γ`, small definables over `Γ` correspond to maps `Γ → U`.

Because `C` is an abstract context category, the notion of “map into U” is whichever
representation of morphisms `C` uses.

## 4. Classifier law (normative)

For each `Γ ∈ C`, there MUST be a natural equivalence:

- `Def_small(Γ) ≈ Hom_C(Γ, U)`

where `≈` is sameness in the ambient coherence level.

This equivalence is the definition of “U classifies small definables.”

## 5. Comprehension (context extension)

Given a small definable `A ∈ Def_small(Γ)`, the extension MUST provide a context
`Γ.A` and a projection `p_A : Γ.A → Γ` such that:

- `A ≈ p_A^*(E)` under the classifier correspondence.

Concretely, `Γ.A` is the pullback of `π : E → U` along the classifying map `Γ → U`
corresponding to `A`.

The extension MUST also provide the usual substitution stability of comprehension:

- for `f : Γ' → Γ`, `(f^*A)` is classified by `Γ' → Γ → U`, and `Γ'.(f^*A)` is equivalent
  to the pullback of `Γ.A` along `f`.

## 6. Size stratification (informative)

To avoid paradoxes, implementations SHOULD stratify universes (a tower
`U_0 : U_1 : U_2 : ...`) or otherwise ensure `Def_small` is predicative.

This document does not mandate a particular stratification strategy.

## 7. Relationship to Gate and BIDIR-DESCENT

- The base Gate laws continue to apply to `Def`.
- If a universe extension is present, implementations MAY add Gate checks ensuring
  `Def_small` is stable/local/gluable and closed under required constructors.

Operationally, codes/decoding and universe reasoning live above the kernel and
SHOULD be treated as Layer-1 “user-space algebra” unless explicitly required by
a profile.

