---
slug: raw
shortname: NORMALIZER
title: workingdoge.com/premath/NORMALIZER
name: Normalization and Comparison Keys for NF
status: raw
category: Standards Track
tags:
  - premath
  - kernel
  - normalizer
  - nf
  - canonicality
  - policy
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

This specification defines a deterministic **normalizer** for Premath kernel
implementations, in terms of existing normal-form constructors (`draft/NF`) and
kernel operations (`raw/OPCODES`-style semantics).

The normalizer exists to make the following operationally meaningful:

- `normalized` comparison mode in `draft/BIDIR-DESCENT`,
- deterministic witness binding to a policy and normalizer version,
- canonicality rejection / normalization behavior.

The normalizer is **backend-generic**: it computes canonical NF bytes, then
computes a comparison reference using `project_ref` as defined by `draft/REF-BINDING`.

This document does **not**:
- prescribe a cryptographic hash scheme,
- prescribe a specific commitment backend,
- require emitting additional KCIR nodes.

## 2. Goals

A conforming normalizer MUST provide:

1. **Determinism**: fixed inputs yield identical outputs.
2. **Policy pinning**: outputs are bound to `policyDigest` and `normalizerId`.
3. **Minimal encoding**: comparisons reduce to equality on a committed key (`Ref`).
4. **Maximum expressiveness**: policies may enable additional canonicalization modules
   without changing NF encoding (policy is pinned by digest).

## 3. Terminology

### 3.1 NF kinds

`kind ∈ {"obj","mor"}`.

- `obj` corresponds to ObjNF bytes (`draft/NF`).
- `mor` corresponds to MorNF bytes (`draft/NF`).

### 3.2 Environment invariants

Normalization is parameterized by the KCIR DAG invariants:

- `envSig: bytes32`
- `uid: bytes32`

These are treated as fixed for a verification run.

### 3.3 Capabilities

Normalization is parameterized by an implementation capability set.

At minimum:

- `adoptPullAtomMor: bool`

If a capability is not claimed, the normalizer MUST reject corresponding NF tags/constructors.

## 4. Normalizer identity and policy digest

### 4.1 Normalizer ID (required)

Each implementation MUST define:

- `normalizerId: string`

This MUST change if any normalization behavior that affects comparisons changes.

### 4.2 Policy descriptor (normative)

A normalization policy is described by the tuple:

- `policyName: string`
- `capabilityMask: u64` (bit 0 = `adoptPullAtomMor`; higher bits reserved)
- `featureMask: u64` (see §4.4)
- `featureParams: list<(featureId:u16, bytes)>` (OPTIONAL; sorted by `featureId`)

### 4.3 Policy digest (required)

Implementations MUST compute:

- `policyDigest: bytes32`

as:

```
policyDigest = SHA256(
  "PremathPolicy" ||
  0x00 ||
  utf8(normalizerId) || 0x00 ||
  utf8(policyName)   || 0x00 ||
  le_u64(capabilityMask) ||
  le_u64(featureMask) ||
  encParams(featureParams)
)

encParams(params) =
  le_u32(len(params)) ||
  concat_{(id,bytes) in params sorted by id}(
    le_u16(id) || le_u32(len(bytes)) || bytes
  )
```

All integer encodings are little-endian fixed-width.

### 4.4 Feature registry (normative)

`featureMask` bits define which canonicalization modules are active.

Bit assignments:

- `0`: `rejectNonCanonicalInputs`
- `1`: `tensorFlattenObj`
- `2`: `tensorUnitElimObj`
- `3`: `tensorFlattenMor`
- `4`: `compFlattenMor`
- `5`: `compIdElimMor`
- `6`: `glueTrivialElimObj`
- `7`: `glueTrivialElimMor`
- `8`: `pushAtomFuse`
- `9`: `pullAtomFuse` (requires `adoptPullAtomMor`)

Unassigned bits are reserved and MUST be zero for conforming policy digests.

### 4.5 Required baseline policy

Full-profile implementations MUST support at least one policy:

- `policyName = "minimal"`

whose `featureMask` MUST implement the mandatory canonicalization implied by the kernel
constructors (0/1 tensor cases, identity-elimination/fusion for spines, and basic arity checks for Glue).

## 5. Normalized output record

`normalize_nf` returns:

```text
Normalized {
  kind: "obj" | "mor",
  cmpRef: Ref,
  normalizerId: string,
  policyDigest: bytes32
}
```

`cmpRef` MUST be computed by applying the active profile’s `project_ref` to the
normalized NF payload bytes, per `draft/REF-BINDING` §2.

## 6. Canonical constructors (normative)

Normalization is defined by rebuilding NF bytes using canonical constructors.

This spec uses shorthand `load_obj(h)` and `load_mor(h)` for loading and parsing NF bytes.
How these bytes are stored is implementation-defined.

### 6.1 mkTensorObj(factors:[ObjRef]) -> ObjRef

Applies the object tensor canonicalization policy.

Minimum required behavior:

- if `len(factors)==0`, return `Unit`
- if `len(factors)==1`, return that factor
- else return `Tensor(factors)` (optionally with additional canonicalization)

If `tensorFlattenObj` is enabled:
- flatten nested `Tensor` factors (preserve order).

If `tensorUnitElimObj` is enabled:
- remove `Unit` factors (after flattening if both enabled).

### 6.2 mkCompMor(src,tgt,parts:[MorRef]) -> MorRef

Applies the morphism composition canonicalization policy.

Minimum required behavior:

- handle 0/1 cases deterministically (implementation-defined but MUST be stable and documented).
- if `compFlattenMor` is enabled, flatten nested `Comp` parts (preserve order).
- if `compIdElimMor` is enabled, eliminate `Id` morphisms inside `Comp`.

### 6.3 mkPullSpine(pId, base) -> ObjRef

Canonical pull spine constructor.

Rules:

1. If `isIdMap(pId)==true`, return `base`.
2. If `base` decodes to `PullSpine(p0, base0)` then return:
   - `mkPullSpine(composeMaps(p0, pId), base0)`
3. Otherwise return `PullSpine(pId, base)`.

### 6.4 mkPushSpine(fId, base) -> ObjRef

Canonical push spine constructor.

Rules:

1. If `isIdMap(fId)==true`, return `base`.
2. If `base` decodes to `PushSpine(f0, base0)` then return:
   - `mkPushSpine(composeMaps(fId, f0), base0)`
3. Otherwise return `PushSpine(fId, base)`.

### 6.5 mkGlueObj(wSig, locals:[ObjRef]) -> ObjRef

Canonical object glue constructor.

Minimum required behavior:

- MUST reject if `len(locals) != coverLen(wSig)`.

If `glueTrivialElimObj` is enabled:
- if `coverLen(wSig)==1`, return `locals[0]`.

Otherwise return `Glue(wSig, locals)`.

### 6.6 mkGlueMor(src,tgt,wSig,locals:[MorRef]) -> MorRef

Canonical morphism glue constructor.

Minimum required behavior:

- MUST reject if `len(locals) != coverLen(wSig)`.

If `glueTrivialElimMor` is enabled:
- if `coverLen(wSig)==1`, return `locals[0]`.

Otherwise return `GlueAtom(src,tgt,wSig,locals)`.

### 6.7 mkPushAtom_full(fId, inner) -> MorRef

Build a canonical `PushAtom` from `fId` and an inner morphism.

Rules:

1. If `isIdMap(fId)==true`, return `inner`.
2. Load and parse `inner` to obtain endpoints `(srcInner, tgtInner)`.
3. If `pushAtomFuse` is enabled and `inner` decodes to `PushAtom(_,_, f0, inner2)` then return:
   - `mkPushAtom_full(composeMaps(fId, f0), inner2)`
4. Let `src = mkPushSpine(fId, srcInner)`.
5. Let `tgt = mkPushSpine(fId, tgtInner)`.
6. Return `PushAtom(src, tgt, fId, inner)`.

### 6.8 pull_obj(pId, inObj) -> ObjRef

`pull_obj` MUST compute the same output that a valid `O_PULL(pId, inObj)` evaluation would produce
under the kernel’s object-pull semantics (classify + canonical rebuilding). Implementations may realize
this by reusing existing opcode logic or by implementing an equivalent pure function.

### 6.9 mkPullAtom_full(pId, inner) -> MorRef  (requires adoptPullAtomMor)

Build a canonical `PullAtom` from `pId` and an inner morphism.

Rules:

1. If `isIdMap(pId)==true`, return `inner`.
2. Load and parse `inner` to obtain endpoints `(srcInner, tgtInner)`.
3. If `pullAtomFuse` is enabled and `inner` decodes to `PullAtom(_,_, p0, inner2)` then return:
   - `mkPullAtom_full(composeMaps(p0, pId), inner2)`
4. Let `src = pull_obj(pId, srcInner)`.
5. Let `tgt = pull_obj(pId, tgtInner)`.
6. Return `PullAtom(src, tgt, pId, inner)`.

If `adoptPullAtomMor == false`, `mkPullAtom_full` MUST reject.

## 7. Normalization procedure (normative)

### 7.1 Inputs

Normalization takes:

- `kind ∈ {"obj","mor"}`
- `valueRef: Ref` (a verified NF reference)
- `envSig, uid`
- `policyDescriptor` (hence `policyDigest`)
- access to NF bytes via store/overlays

If required store entries are missing, normalization MUST fail deterministically.

### 7.2 Normalizing ObjNF

To normalize an ObjNF:

1. Load and parse ObjNF bytes; unknown tags MUST reject.
2. Recursively normalize subcomponents (factors, base objects, locals).
3. Rebuild using canonical constructors:
   - `Unit` and `Prim` are unchanged
   - `Tensor(factors)` becomes `mkTensorObj(normalizedFactors)`
   - `PullSpine(pId, base)` becomes `mkPullSpine(pId, normalizedBase)`
   - `PushSpine(fId, base)` becomes `mkPushSpine(fId, normalizedBase)`
   - `Glue(wSig, locals)` becomes `mkGlueObj(wSig, normalizedLocals)`
4. Encode the resulting ObjNF bytes in canonical encoding (`draft/NF`).
5. Compute `cmpRef` as:
   - `project_ref("kcir.obj_nf", envSig || uid || obj_nf_bytes_norm)`.

### 7.3 Normalizing MorNF

To normalize a MorNF:

1. Load and parse MorNF bytes; unknown tags MUST reject.
2. Enforce capability rules:
   - if MorNF tag `PullAtom` appears and `adoptPullAtomMor == false`, reject.
3. Recursively normalize subcomponents (inner morphisms, parts, locals).
4. Normalize referenced endpoint objects where present and rebuild endpoints from canonicalized objects.
5. Rebuild using canonical constructors:
   - `Id(src)` rebuilt with canonical `src`
   - `Comp(...)` rebuilt via `mkCompMor`
   - `TensorAtom(...)` MAY be canonicalized if `tensorFlattenMor` is enabled
   - `GlueAtom(...)` rebuilt via `mkGlueMor`
   - `PushAtom(...)` rebuilt via `mkPushAtom_full`
   - `PullAtom(...)` rebuilt via `mkPullAtom_full` (if enabled)
6. Encode normalized MorNF bytes (`draft/NF`) and compute:
   - `cmpRef = project_ref("kcir.mor_nf", envSig || uid || mor_nf_bytes_norm)`.

### 7.4 Input handling: reject vs normalize

If `rejectNonCanonicalInputs` is enabled:
- the normalizer MUST reject if the input NF bytes are not already canonical under the active policy.

If `rejectNonCanonicalInputs` is disabled:
- the normalizer MUST normalize any input NF that parses successfully.

## 8. Normalized equality (normative)

In `normalized` mode (see `draft/BIDIR-DESCENT`), two values are equal iff their
normalized comparison refs are equal:

```text
eq_normalized(kind, refA, refB, envSig, uid, policy) :=
  normalize_nf(kind, refA, envSig, uid, policy).cmpRef ==
  normalize_nf(kind, refB, envSig, uid, policy).cmpRef
```

No other equality notion is permitted in `normalized` mode.
