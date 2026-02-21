---
slug: raw
shortname: BASEAPI-TOY-VIEWS
title: workingdoge.com/premath/BASEAPI-TOY-VIEWS
name: Toy Views Base API (Reference Constructor)
status: raw
category: Informational
tags:
  - premath
  - baseapi
  - toy
  - vectors
  - examples
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

## 1. Purpose

This document defines a **toy, fully explicit** instantiation of the Premath
Base API requirements (see `raw/OPCODES`, section "Base API requirements").

It exists for two reasons:

1. Provide a **hello-world constructor** (contexts + maps + covers) that makes
   Gate laws concrete.
2. Provide a deterministic substrate for **generating test vectors**
   (see `raw/TOY-VECTORS`).

This is not intended to be "the" production context model. It is a small,
auditable reference instance.

## 2. Contexts

A **ToyViews context** is a finite subset of view-indices from a fixed universe:

- `U = {0,1,2,...,31}`.

A context is represented canonically by a 32-bit bitmask `mask: u32`:

- index `i` is in the context iff bit `i` of `mask` is 1.

The empty context is `mask = 0`.

### 2.1 Intersection (pullbacks)

ToyViews has pullbacks: for inclusions (below), pullback is set intersection:

- `mask(A ×_C B) = mask(A) & mask(B)`.

Implementations MUST compute intersections using bitwise AND.

## 3. Maps (context morphisms)

ToyViews maps are **inclusions only**.

A map `p : W -> U` exists iff `W ⊆ U`, i.e.:

- `W.mask & ~U.mask == 0`.

### 3.1 MapId encoding (Bytes32)

A map identifier `mapId: Bytes32` is encoded as:

- `srcMask: u32 little-endian` (bytes 0..3)
- `tgtMask: u32 little-endian` (bytes 4..7)
- bytes 8..31 MUST be zero.

This encoding is canonical. Implementations MUST reject non-canonical encodings
(bytes 8..31 non-zero) in hardened modes.

We write:

- `src(mapId)` for the decoded `srcMask`
- `tgt(mapId)` for the decoded `tgtMask`

A decoded map MUST satisfy `src ⊆ tgt` or be rejected.

### 3.2 Base API map operations

`isIdMap(mapId)` MUST return true iff `src(mapId) == tgt(mapId)`.

`composeMaps(outer, inner)` (categorical composition `outer ∘ inner`) MUST:

1. decode `inner : A -> B` and `outer : B -> C` (i.e. `tgt(inner) == src(outer)`),
2. return the canonical mapId encoding of `A -> C`.

If `tgt(inner) != src(outer)`, composition MUST be rejected.

## 4. Covers

A cover over a base context `U` is a finite family of inclusion legs
`u_i : U_i -> U` such that:

- each `U_i ⊆ U`, and
- `⋃_i U_i = U`.

ToyViews covers are represented by **cover data**:

```text
CoverData {
  baseMask: u32,
  legs: list<u32>  // each a legMask
}
```

### 4.1 Canonical cover normalization

To make cover behavior deterministic:

- leg masks MUST be non-zero
- leg masks MUST be subsets of baseMask
- legs MUST be sorted strictly increasing by numeric value
- duplicate legs MUST be rejected

A cover is valid iff the bitwise OR of its leg masks equals `baseMask`.

### 4.2 coverSig encoding and store

ToyViews uses a store keyed by `coverSig: Bytes32`.

The cover payload bytes are:

- `baseMask:u32le || nLegs:u32le || legMask[0]:u32le || ... || legMask[n-1]:u32le`

where `legMask[]` is the canonical sorted list.

The cover signature is:

- `coverSig = SHA256( "ToyCover" || coverPayloadBytes )`

This `coverSig` is only an identifier for retrieving `CoverData` in tests and
fixtures; it is not a security boundary.

`validateCover(coverSig, coverData)` MUST recompute `coverSig` from canonical
coverPayloadBytes and return true iff they match and the cover validity rules
hold.

`coverLen(coverSig)` MUST return the number of legs in its stored `CoverData`.

## 5. Pulling covers

The Base API function:

- `pullCover(pId, uSig) -> (wSig, mapWtoU[], projIds[])`

is defined as follows.

Inputs:

- `pId : W -> U` (decoded by §3)
- `uSig` is a valid cover signature for a cover over `U`

Let the cover legs be `U_i ⊆ U` in canonical order (increasing masks).

Define the pulled cover over `W` by intersecting each leg:

- `W_i = W ∩ U_i` (bitwise AND)

Discard any empty intersections (`W_i = 0`).

The pulled cover legs are the non-empty `W_i` in the same order as the source
legs.

Outputs:

- `wSig` is the coverSig of the pulled cover (computed by §4.2).
- `mapWtoU[k]` is the source-leg index `i` that produced the k-th pulled leg.
- `projIds[k]` is the canonical mapId for the inclusion `W_k -> U_i`
  (src = `W_k`, tgt = `U_i`).

All three outputs MUST be deterministic and MUST match the algorithms above.

## 6. Beck–Chevalley squares (toy)

ToyViews provides a deterministic "BC square" constructor for inclusions.

`BCAllowed(pId, fId)` MUST return true iff:

- `tgt(pId) == tgt(fId)` (both map into the same context), and
- both mapIds are valid inclusions.

`bcSquare(pushId, pullId)` where:

- `pushId : A -> B`
- `pullId : C -> B`

MUST return `(fPrime, pPrime)` where the pullback context is:

- `D = A ∩ C` (bitwise AND)

and:

- `fPrime : D -> C` (src = D, tgt = C)
- `pPrime : D -> A` (src = D, tgt = A)

encoded as canonical mapIds per §3.1.

## 7. Notes

- This toy Base API is intentionally finite and inclusion-only.
- It is sufficient to exercise locality, descent, and BC-style base-change in a
  small, auditable setting.
- Production deployments MAY use different context models; they should add new
  Base API specifications and conformance suites rather than mutating this one.
