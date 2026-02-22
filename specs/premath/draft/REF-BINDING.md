---
slug: draft
shortname: REF-BINDING
title: workingdoge.com/premath/REF-BINDING
name: Reference Binding and Digest Projection
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - kcir
  - ref
  - profile
  - binding
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

Premath artifacts (KCIR nodes, ObjNF, MorNF, policy descriptors, etc.) are
serialized into canonical byte strings. A commitment backend (a verifier
profile) binds these byte strings into opaque references:

```text
Ref {
  scheme_id: string,
  params_hash: bytes32,
  domain: string,
  digest: bytes
}
```

This specification defines:

1. a profile-independent notion of **payload bytes** per domain,
2. a profile-provided pure function **digest projection** (`project_ref`),
3. a profile-provided **evidence verification** procedure (`verify_ref`).

The purpose is to keep the kernel backend-generic:
opcodes and normalizers compute expected outputs without hardcoding any
hash scheme.

## 2. Domain payload bytes (profile-independent)

A verifier operates in a run context with DAG invariants:

- `envSig: bytes32`
- `uid: bytes32`

Domains are strings. For each domain, this spec defines the canonical
`payload_bytes` that are bound by `Ref`.

### 2.1 KCIR node domain

For `domain = "kcir.node"`:

- `payload_bytes = node_bytes`

### 2.2 NF domains (context-relative)

For `domain = "kcir.obj_nf"`:

- `payload_bytes = envSig || uid || obj_nf_bytes`

For `domain = "kcir.mor_nf"`:

- `payload_bytes = envSig || uid || mor_nf_bytes`

### 2.3 Other domains

Additional domains MAY be defined by numbered specs or profile bundles.
Each additional domain MUST specify its payload byte construction rule.
If a domain is not defined, verifiers MUST reject.

## 3. Profile interface requirements

A verifier profile MUST provide two distinct layers.

### 3.1 Digest projection (required)

A pure function:

- `project_ref(domain: string, payload_bytes: bytes) -> Ref`

Rules:

- `Ref.scheme_id` MUST equal the profile’s `scheme_id`.
- `Ref.params_hash` MUST equal the profile’s pinned parameter hash.
- `Ref.domain` MUST equal `domain` (string equality).
- `Ref.digest` MUST be deterministically computed from
  `(domain, scheme_id, params_hash, payload_bytes)` according to the profile’s binding rule.

`project_ref` MUST NOT require evidence or anchors.
It is used by opcode contracts, normalization, and deterministic comparisons.

### 3.2 Evidence + anchor verification (required)

A function:

- `verify_ref(ref: Ref, payload_bytes: bytes, evidence: bytes, anchors: ...) -> Ok | Error`

`verify_ref` MUST check:

1. `ref.scheme_id` matches the active profile,
2. `ref.params_hash` matches the active profile parameters,
3. `ref.domain` is supported by the profile’s domain table,
4. `ref.digest == project_ref(ref.domain, payload_bytes).digest`,
5. evidence and anchors validate per the profile (if the profile requires them).

Profiles that do not use evidence MUST treat evidence as empty and MUST reject
non-empty evidence deterministically.

## 4. Binding equality (normative)

In `normalized` comparison mode (see `draft/BIDIR-DESCENT`), equality MUST be
computed as strict equality of projected refs:

```text
project_ref(domain, payloadA) == project_ref(domain, payloadB)
```

No other notion of equality is permitted in `normalized` mode.

## 5. Determinism requirements

Given the same:

- store content,
- profile parameters and anchors,
- `(envSig, uid)`,
- and canonical payload bytes,

`project_ref` and `verify_ref` outcomes MUST be identical across runs and
implementations.

## 6. Relationship to other specs (informative)

- `draft/KCIR-CORE` defines the `Ref` and store model.
- `draft/NF` defines canonical NF byte grammars.
- `raw/OPCODES` computes expected outputs by building canonical bytes and applying `project_ref`.
- `draft/NORMALIZER` computes canonical forms and uses `project_ref` to derive comparison keys.
