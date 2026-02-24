---
slug: raw
shortname: TOY-VECTORS
title: workingdoge.com/premath/TOY-VECTORS
name: Toy Semantic Vector Suite (Hello World)
status: raw
category: Informational
tags:
  - premath
  - vectors
  - toy
  - gate
  - determinism
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

Premath conformance is ultimately established by **running code** on canonical
vectors (`draft/CONFORMANCE`).

Before a full KCIR compiler/verifier stack exists, it is useful to have a
minimal, auditable, **semantic** vector suite that exercises the core Gate law
classes directly on a tiny constructor.

This document defines such a suite:

- a fixed toy constructor of contexts/maps/covers (`raw/BASEAPI-TOY-VIEWS`)
- several toy "Def" assignments that are intentionally good/bad
- deterministic expected Gate outcomes (`accepted` or `rejected` + witness class)

This suite exists to prevent semantic drift while implementation work proceeds.

## 2. Directory layout

The repository layout is:

- `tests/toy/fixtures/<case_id>/case.json`
- `tests/toy/fixtures/<case_id>/expect.json`

A runner script MAY also emit an `out.json` next to the case.

## 3. case.json schema (v1)

`case.json` MUST be a JSON object:

```json
{
  "schema": 1,
  "world": "sheaf_bits" | "bad_constant" | "non_separated" | "bad_stability" | "partial_restrict",
  "check": { ... }
}
```

### 3.1 Worlds

The `world` field selects a small constructor instance for `Def`:

- `sheaf_bits`:
  - `Def(mask)` = total functions from the set of bits in `mask` to `{0,1}`.
  - reindexing is restriction.
  - descent glues by union and is unique.

- `bad_constant` (descent failure):
  - `Def(mask!=0)` = `{0,1}`, `Def(0)` = `{*}`.
  - restrictions to non-empty are identity; restriction to `0` maps both `0,1` to `*`.
  - local data on disjoint covers can be compatible but fail to globalize.

- `non_separated` (non-contractible glue space):
  - `Def(mask!=0)` = `{0,1}`, `Def(0)` = `{*}`.
  - restriction from non-empty to non-empty is the constant map to `0`.
  - some compatible local data admits multiple global realizations.

- `bad_stability` (non-functorial restriction):
  - like `bad_constant`, but with at least one pair of composable inclusions
    where `(f o g)*` and `g* o f*` disagree.

- `partial_restrict` (locality failure):
  - like `bad_constant`, but with at least one cover leg `u_i` such that `u_i*`
    is undefined for the claimed definable.

The worlds above are intended to directly target the Gate failure classes:

- `stability_failure`
- `locality_failure`
- `descent_failure`
- `glue_non_contractible`

### 3.2 Checks

A check is one of:

#### 3.2.1 Descent check

```json
{
  "kind": "descent",
  "baseMask": 7,
  "legs": [3, 6],
  "locals": [ ... ],
  "tokenPath": null
}
```

Interpretation:

- `baseMask` is the base context Γ.
- `legs[i]` is the i-th cover leg context Γᵢ (each MUST be a subset of Γ).
- `locals[i]` is a local definable in `Def(legs[i])` for the selected world.
- Overlap compatibility is computed by restricting locals to each pairwise
  intersection `legs[i] & legs[j]`.

The checker MUST evaluate:

1. locality (locals are well-typed),
2. overlap compatibility,
3. existence of at least one global glue, and
4. contractibility (uniqueness up to equality, since these toy worlds are set-level).

#### 3.2.2 Stability check

```json
{
  "kind": "stability",
  "gammaMask": 7,
  "a": ...,
  "f": {"src": 3, "tgt": 7},
  "g": {"src": 1, "tgt": 3},
  "tokenPath": null
}
```

Interpretation:

- `a ∈ Def(gammaMask)`.
- `f : Γ' -> Γ` and `g : Γ'' -> Γ'` are inclusions encoded by masks.
- The checker MUST verify the Gate law:
  `(f o g)* a = g*(f* a)`.

#### 3.2.3 Locality check

```json
{
  "kind": "locality",
  "gammaMask": 3,
  "a": ...,
  "legs": [1, 2],
  "tokenPath": null
}
```

Interpretation:

- Check that for each leg inclusion `u_i : Γᵢ -> Γ`, the restriction `u_i* a`
  exists.

## 4. expect.json schema

`expect.json` MUST be a Gate witness payload in the format of `draft/GATE` §5.1.

At minimum:

- for accepted: `{ result:"accepted", failures:[] }`
- for rejected: at least one failure with:
  - `class` and `lawRef`
  - a correct deterministic `witnessId` per `draft/WITNESS-ID`

Runners MAY ignore `message` and other non-key fields when comparing, but MUST
enforce `class`, `lawRef`, and `witnessId`.

## 5. Relationship to KCIR conformance

This suite is **not** a replacement for KCIR-based conformance.

It is a stopgap to lock the meaning of the Gate law classes while the KCIR
compiler/verifier is developed.

Once KCIR-based Gate vectors exist, these toy vectors MAY remain as a fast
semantic sanity check.
