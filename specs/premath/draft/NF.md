---
slug: draft
shortname: NF
title: workingdoge.com/premath/NF
name: Normal Forms (ObjNF / MorNF)
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - normal-forms
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

This specification defines two canonical normal-form byte languages:

- **ObjNF**: normal forms for objects
- **MorNF**: normal forms for morphisms

NF bytes are *canonical payloads* that are bound to references by a commitment backend
(profile) via `draft/REF-BINDING`.

NF bytes MAY contain embedded references to other NF values. To keep NF encoding minimal,
this spec encodes embedded references as their raw `digest` bytes (length-prefixed).

Lifting an embedded digest to a full `Ref` is profile- and domain-context dependent and
is performed by verifiers using the active profile’s `(scheme_id, params_hash)` and
the appropriate NF domain (`kcir.obj_nf` or `kcir.mor_nf`).

## 2. Encoding primitives (normative)

- `varint`: unsigned LEB128.

- `encDigest(d)` where `d: bytes`:
  - `encDigest(d) = varint(len(d)) || d`

- `encListDigest([d1..dn])`:
  - `encListDigest(xs) = varint(n) || encDigest(d1) || ... || encDigest(dn)`

`Bytes32` values are fixed-width 32-byte strings.

Implementations MUST reject malformed/truncated encodings deterministically.

## 3. ObjNF grammar (normative)

Each ObjNF value is encoded as:

- `tag:u8 || payload`

Tags:

| Tag  | Constructor | Payload |
|------|-------------|---------|
| 0x01 | `Unit`      | *(none)* |
| 0x02 | `Prim`      | `primId:Bytes32` |
| 0x03 | `Tensor`    | `encListDigest(factors:[objDigest])` |
| 0x04 | `PullSpine` | `pId:Bytes32 || base:encDigest(objDigest)` |
| 0x05 | `PushSpine` | `fId:Bytes32 || base:encDigest(objDigest)` |
| 0x06 | `Glue`      | `wSig:Bytes32 || encListDigest(locals:[objDigest])` |

Here `objDigest` denotes the `Ref.digest` bytes for `kcir.obj_nf` under the active profile context.

## 4. MorNF grammar (normative)

Each MorNF value is encoded as:

- `tag:u8 || payload`

Tags:

| Tag  | Constructor | Payload |
|------|-------------|---------|
| 0x11 | `Id`        | `src:encDigest(objDigest)` |
| 0x13 | `Comp`      | `src:encDigest(objDigest) || tgt:encDigest(objDigest) || encListDigest(parts:[morDigest])` |
| 0x17 | `PushAtom`  | `src:encDigest(objDigest) || tgt:encDigest(objDigest) || fId:Bytes32 || inner:encDigest(morDigest)` |
| 0x18 | `TensorAtom`| `src:encDigest(objDigest) || tgt:encDigest(objDigest) || encListDigest(parts:[morDigest])` |
| 0x19 | `GlueAtom`  | `src:encDigest(objDigest) || tgt:encDigest(objDigest) || wSig:Bytes32 || encListDigest(locals:[morDigest])` |

Optional extension tag (enabled only if the implementation claims `adoptPullAtomMor`):

| Tag  | Constructor | Payload |
|------|-------------|---------|
| 0x16 | `PullAtom`  | `src:encDigest(objDigest) || tgt:encDigest(objDigest) || pId:Bytes32 || inner:encDigest(morDigest)` |

Here `morDigest` denotes the `Ref.digest` bytes for `kcir.mor_nf` under the active profile context.

If `PullAtom` is not adopted, implementations MUST reject tag `0x16`.

## 5. Canonicality notes (informative)

This spec defines the byte-level grammar. Canonicality policies (fusion, identity elimination,
flattening) are defined by opcode constructors and by `raw/NORMALIZER`.

Implementations MAY enforce canonicality by rejecting noncanonical NF bytes in hardened modes.
