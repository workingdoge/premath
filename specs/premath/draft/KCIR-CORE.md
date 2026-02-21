---
slug: draft
shortname: KCIR-CORE
title: workingdoge.com/premath/KCIR-CORE
name: KCIR Core (Profiled Reference Model)
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - kcir
  - commitments
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

This document defines the profile-independent KCIR core model:

- the reference type `Ref`,
- the certificate node type `KCIRNode`,
- the store contract,
- and the verifier profile interface.

Cryptographic binding, proofs, anchors, and inclusion mechanisms are delegated
to profiles via the profile interface defined here and in `draft/REF-BINDING`.

## 2. Core types

### 2.1 Ref

A reference is opaque to core semantics.

```text
Ref {
  scheme_id: string,
  params_hash: bytes32,
  domain: string,
  digest: bytes
}
```

Rules:

- `scheme_id` identifies the active backend/profile family.
- `params_hash` pins backend parameters.
- `domain` is a string used for domain separation (e.g. `kcir.node`, `kcir.obj_nf`, `kcir.mor_nf`).
- `digest` is profile-defined, variable-length.

### 2.2 KCIRNode

```text
KCIRNode {
  env_sig: bytes32,
  uid: bytes32,
  sort: u8,
  opcode: u8,
  out: Ref,
  args: bytes,
  deps: list<Ref>
}
```

Rules:

- Node parse MUST be deterministic.
- Dependency order is semantically meaningful where an opcode contract requires it.
- `env_sig` and `uid` are DAG invariants: verifiers MUST reject DAGs whose nodes disagree.

### 2.3 Store contract

A store MUST provide:

```text
get_node(ref: Ref) -> Option<(node_bytes, evidence)>
get_obj_nf(ref: Ref) -> Option<(obj_nf_bytes, evidence)>
get_mor_nf(ref: Ref) -> Option<(mor_nf_bytes, evidence)>
```

`evidence` is profile-defined.

Stores MAY be backed by overlays computed during verification (implementation-defined),
but overlays MUST take precedence over store entries.

### 2.4 Verifier profile interface (required)

A verifier profile MUST provide:

1. `project_ref(domain, payload_bytes) -> Ref` (pure digest projection)
2. `verify_ref(ref, payload_bytes, evidence, anchors, domain) -> Ok | Error`

as specified by `draft/REF-BINDING`.

### 2.5 Domain table (required)

Each profile MUST define a deterministic domain mapping table for the domains it supports,
including at minimum:

- `kcir.node`
- `kcir.obj_nf`
- `kcir.mor_nf`

If a referenced domain is not supported, verifiers MUST reject deterministically.

## 3. Determinism

### 3.1 Deterministic verification

Given the same:

- root ref,
- store content,
- profile params + anchors,

verification results MUST be identical.

### 3.2 Error determinism

Verifiers MUST return stable machine-readable error codes (see `draft/ERROR-CODES`).

## 4. Core verification procedure (informative outline)

1. Load root node by `Ref`.
2. Verify node evidence (profile `verify_ref`) using `payload_bytes` for `kcir.node`.
3. Parse node bytes, enforce wire constraints.
4. Recurse deps with cycle detection.
5. Resolve required NF payloads for opcode checks and verify evidence.
6. Enforce opcode contracts and canonicality rules.
7. Emit deterministic witnesses.

## 5. Payload bytes (normative)

Verifiers MUST construct `payload_bytes` for `project_ref` / `verify_ref` exactly as
defined in `draft/REF-BINDING` §2 for each domain.

In particular, for NF domains:

- `payload_bytes = envSig || uid || nf_bytes`.

This rule is profile-independent.
