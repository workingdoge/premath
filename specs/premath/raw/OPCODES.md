---
slug: raw
shortname: OPCODES
title: workingdoge.com/premath/OPCODES
name: Opcode Registry and Contracts
status: raw
category: Standards Track
tags:
  - premath
  - kernel
  - opcodes
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

This specification defines:

- registries for `(sort, opcode)` bytes used by KCIR nodes (`draft/KCIR-CORE`),
- normative verification contracts for each opcode.

This greenfield bundle defines a minimal opcode set sufficient to construct and
transport normal forms (NF), covers, and Beck–Chevalley square witnesses.

Outputs are bound to references using the active profile’s `project_ref` and
payload rules from `draft/REF-BINDING`.

## 2. Sort registry

The KCIR `sort` field is a single byte:

| Sort name | Byte |
|----------:|:----:|
| COVER     | 0x01 |
| MAP       | 0x02 |
| OBJ       | 0x03 |
| MOR       | 0x04 |

Unknown `sort` values MUST be rejected.

## 3. Base API requirements (normative)

Opcode verification relies on a Base API that provides deterministic functions:

- `isIdMap(mapId: bytes32) -> bool`
- `composeMaps(outer: bytes32, inner: bytes32) -> bytes32`
- `validateCover(coverSig: bytes32, coverData: bytes) -> bool`
- `coverLen(coverSig: bytes32) -> int`
- `pullCover(pId: bytes32, uSig: bytes32) -> (wSig: bytes32, mapWtoU: [u32], projIds: [bytes32])`
- `BCAllowed(pId: bytes32, fId: bytes32) -> bool`
- `bcSquare(pushId: bytes32, pullId: bytes32) -> (fPrime: bytes32, pPrime: bytes32)`

These functions MUST be deterministic.

## 4. Reference binding helper (normative)

Let `project_nf(domain, nf_bytes, envSig, uid)` denote:

- `project_ref(domain, envSig || uid || nf_bytes)` under the active profile, per `draft/REF-BINDING`.

Opcode contracts that construct NF bytes MUST check that `out == project_nf(...)`.

## 5. COVER opcodes

### 5.1 C_LITERAL (sort=COVER opcode=0x01)

**Args:** `coverSig:Bytes32`
**Deps:** none

Verify:

- The verifier MUST check `out.domain == "kcir.cover"` if the implementation uses a cover domain (implementation-defined).
- The verifier MUST check `out.digest` encodes `coverSig` under the active profile’s conventions, or MUST treat `out` as an opaque cover reference if coverSig is directly the out digest.
- The verifier MUST check `validateCover(coverSig, CoverStore[coverSig]) == true`.

Meta:

- `{ kind: "C_LITERAL", coverSig }`

*(Note: cover referencing is intentionally left flexible in this greenfield bundle; many implementations represent `coverSig` directly as the committed digest.)*

### 5.2 C_PULLCOVER (sort=COVER opcode=0x02)

**Args:** `encListU32(mapWtoU) || encListB32(projIds)` (implementation-defined wire)
**Deps:** one MAP dep (`pId`), one COVER dep (`uSig`)

Verify:

1. Compute `(wSigExp, mapWtoUExp, projIdsExp) = pullCover(pId, uSig)`.
2. Check `out` matches `wSigExp` under the cover reference convention.
3. Decode args and check equality with computed witness arrays.
4. Range-check each `mapWtoUExp[k] < coverLen(uSig)`.

Meta:

- `{ kind:"C_PULLCOVER", pId, uSig, wSig: out, mapWtoU, projIds }`

## 6. MAP opcodes

### 6.1 M_LITERAL (sort=MAP opcode=0x01)

**Args:** `mapId:Bytes32`
**Deps:** none

Verify:

- `out` binds to `mapId` under the map reference convention (implementation-defined), or `out.digest == mapId` in digest-only variants.

Meta: `{ kind:"M_LITERAL", mapId }`

### 6.2 M_BC_FPRIME (sort=MAP opcode=0x10)

**Args:** none
**Deps:** two MAP deps `(pullId, pushId)`.

Verify:

- Compute `(fPrime, _) = bcSquare(pushId, pullId)`.
- Check `out` matches `fPrime` under the map reference convention.

Meta: `{ kind:"M_BC_FPRIME", pullId, pushId }`

### 6.3 M_BC_GPRIME (sort=MAP opcode=0x11)

**Args:** none
**Deps:** two MAP deps `(pullId, pushId)`.

Verify:

- Compute `(_, pPrime) = bcSquare(pushId, pullId)`.
- Check `out` matches `pPrime` under the map reference convention.

Meta: `{ kind:"M_BC_GPRIME", pullId, pushId }`

## 7. OBJ opcodes

OBJ opcodes construct ObjNF bytes per `draft/NF` and bind them via `project_nf("kcir.obj_nf", ...)`.

### 7.1 O_UNIT (sort=OBJ opcode=0x01)

**Args:** none
**Deps:** none

Verify:

- `objBytes = 0x01`
- `expOut = project_nf("kcir.obj_nf", objBytes, envSig, uid)`
- Check `out == expOut`.

### 7.2 O_PRIM (sort=OBJ opcode=0x02)

**Args:** `primId:Bytes32`
**Deps:** none

Verify:

- `objBytes = 0x02 || primId`
- `expOut = project_nf("kcir.obj_nf", objBytes, envSig, uid)`
- Check `out == expOut`.

### 7.3 O_MKTENSOR (sort=OBJ opcode=0x03)

**Args:** `encListDigest(factors:[objDigest])`
**Deps:** none

Verify:

- Decode factors.
- Apply canonicalization policy as defined by `raw/NORMALIZER` for `mkTensorObj`.
- Encode canonical ObjNF bytes.
- Bind via `project_nf` and check equality to `out`.

Meta: `{ kind:"O_MKTENSOR", factors: canonicalFactors }`

### 7.4 O_MKGLUE (sort=OBJ opcode=0x04)

**Args:** `wSig:Bytes32`
**Deps:** `coverNodeRef || encListDigest(localObjNodeRefs)` (profile-defined encoding of node refs)

This opcode constructs an explicit *glue candidate* object in ObjNF form:

`Glue(wSig, locals)` where `locals` is a list of object references (in canonical cover-leg order).

This is a **proof-carrying descent trace seam**: the Gate checks the sheaf/descent law;
`O_MKGLUE` only commits to a concrete witness object derived from its dependencies.

Verify:

1. Parse `wSig`.
2. Load cover data for `wSig` via Base API and compute `n = coverLen(wSig)`.
3. Require deps of the form:
   - first dep is a `COVER/C_LITERAL` node with `out == wSig`.
   - followed by exactly `n` OBJ nodes (one per cover leg), whose `out` values are the local object refs.
4. Let `locals = [dep[i].out]` for the local deps in order.
5. Build `objBytes = 0x06 || wSig || encListDigest(locals)` (i.e. ObjNF `Glue`).
6. Bind and check:
   - `expOut = project_nf("kcir.obj_nf", objBytes, envSig, uid)`
   - `out == expOut`.

Meta: `{ kind:"O_MKGLUE", wSig, locals }`

### 7.4.1 O_ASSERT_OVERLAP (sort=OBJ opcode=0x05) [optional Gate witness]

**Args:** `ovMask:u32le`
**Deps:** exactly two OBJ nodes, interpreted as the *i*th and *j*th local definables.

This opcode is a **proof-carrying descent seam**: it certifies that two local definables
agree on their pairwise overlap.

Verify (parameterized by the active constructor semantics):

1. Decode `ovMask`.
2. Load the two dependent ObjNF bytes and interpret them as definables in the active constructor.
3. Compute the overlap restrictions along the overlap context `ovMask`.
4. Reject unless the two restricted definables are equal up to the constructor's sameness relation (`≈`).

Output:

- `out` MUST be `Unit` (the unique trivial definable witness object) in the active ObjNF presentation.

Determinism requirement:

- overlap restriction and equality checks MUST be deterministic for fixed inputs.

### 7.4.2 O_ASSERT_TRIPLE (sort=OBJ opcode=0x06) [optional Gate witness]

**Args:** `triMask:u32le`
**Deps:** exactly three OBJ nodes, interpreted as local definables.

This opcode certifies triple-overlap coherence (a "cocycle" check): the three locals agree
when restricted to the triple overlap context `triMask`.

Verify:

1. Decode `triMask`.
2. Restrict each local definable to `triMask`.
3. Reject unless all three restricted definables are equal up to `≈`.

Output:

- `out` MUST be `Unit`.

Notes:

- In set-level constructors with functional restriction, pairwise agreement often implies triple agreement,
  but higher-level constructors may require explicit coherence witnesses. This opcode provides a uniform
  certificate slot.

### 7.4.3 O_ASSERT_CONTRACTIBLE (sort=OBJ opcode=0x07) [optional Gate witness]

**Args:** `schemeId:Bytes32 || proofBytes:bytes`

**Deps:** exactly one OBJ node, the glue-candidate constructor `O_MKGLUE`.

This opcode certifies **stack-safe uniqueness** for the glue datum: the space of global
realizations is contractible (unique up to unique `≈`).

In a set-level constructor this reduces to: “there exists exactly one global definable whose
restrictions match the local data.” In higher constructors, the verifier MUST check the
appropriate notion of contractibility (e.g., unique up to unique isomorphism / equivalence),
but the opcode shape is uniform.

Verify (parameterized by the active constructor semantics and proof scheme):

1. Require the single dep to be an `OBJ/O_MKGLUE` node.
2. Interpret the dep’s output as a descent datum `(U ⊳ Γ; A_i)`.
3. Compute the space of global glue candidates `Glue(U; A_i, φ_ij)` (where overlap/cocycle
   compatibilities are either:
   - provided by sibling witness opcodes (`O_ASSERT_OVERLAP` / `O_ASSERT_TRIPLE`), or
   - reconstructed deterministically from the constructor’s semantics when possible).
4. Decode `schemeId` and interpret `proofBytes` as an *opaque* proof payload for that scheme.
5. Reject unless the verifier’s active constructor semantics accepts the proof payload as a
   valid certificate that the glue space is **contractible** in the constructor’s sameness level.

Output:

- `out` MUST be `Unit`.

Notes:

- This opcode is deliberately “Gate-adjacent”: it is a certificate slot that lets a KCIR trace
  carry its own contractibility proof. In a full-profile verifier, the Gate MAY treat the
  presence of this witness as sufficient to skip recomputing uniqueness.

- `schemeId` is an **opaque Bytes32 label**. Implementations MAY define local registries of
  supported schemes. Unsupported `schemeId` values MUST be rejected.

### 7.5 O_PULL (sort=OBJ opcode=0x10)

**Args:** `pId:Bytes32 || inObjRef:Ref || stepTag:u8` (wire encoding profile-defined)
**Deps:** branch-dependent

Verify prelude:

1. If `isIdMap(pId)`, `stepTag` MUST be `0x00` (ID) and `out == inObjRef`.
2. Otherwise load and parse `inObjRef` as ObjNF bytes.
3. Compute `expStep = classify_obj_pull(pId, inObjBytes)` and check `stepTag == expStep`.
4. Dispatch by `stepTag`, rebuilding canonical bytes using `raw/NORMALIZER` constructors.

`classify_obj_pull` MUST be deterministic and MUST depend only on `pId` and the parsed ObjNF constructor.

*(This bundle does not fix the complete step tag taxonomy; implementations MAY use the step tags from the legacy opcode set, provided they publish conformance vectors and keep determinism.)*

## 8. MOR opcodes (minimal)

MOR opcodes construct MorNF bytes per `draft/NF` and bind them via `project_nf("kcir.mor_nf", ...)`.

### 8.1 M_ID (sort=MOR opcode=0x01)

**Args:** `srcRef: Ref`
**Deps:** none

Verify:

- `morBytes = 0x11 || encDigest(srcRef.digest)` (using the NF encoding rules)
- `expOut = project_nf("kcir.mor_nf", morBytes, envSig, uid)`
- Check `out == expOut`.

### 8.2 M_MKCOMP (sort=MOR opcode=0x03)

**Args:** `srcRef || tgtRef || encListDigest(parts)`
**Deps:** none

Verify:

- Decode and apply canonicalization policy as defined by `raw/NORMALIZER` `mkCompMor`.
- Encode canonical MorNF bytes and bind via `project_nf`.

### 8.3 M_PULL (sort=MOR opcode=0x10)

**Args:** `pId || inMorRef || stepTag`
**Deps:** branch-dependent

Verify prelude:

1. If `isIdMap(pId)`, `stepTag` MUST be `0x00` and `out == inMorRef`.
2. Otherwise load and parse MorNF bytes.
3. Compute `expStep = classify_mor_pull(pId, inMorBytes, adoptPullAtomMor)` and check equality.
4. Dispatch by `stepTag` and rebuild via `raw/NORMALIZER` canonical constructors.

## 9. Relationship to Gate

This document defines execution-level contracts. Admissibility (stability/locality/descent/coherence)
is enforced by `draft/GATE` using the bidirectional obligation discipline in `draft/BIDIR-DESCENT`.
