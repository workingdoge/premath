---
slug: draft
shortname: WIRE-FORMATS
title: workingdoge.com/premath/WIRE-FORMATS
name: KCIR Wire Format Registry
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - kcir
  - wire-format
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

This document defines a registry of `wireFormatId` values referenced by
`draft/KCIR-CORE`.

A `wireFormatId` entry MUST fully specify:

- KCIR node binary encoding rules,
- reference payload encoding rules,
- malformed-input rejection requirements.

Profiles MUST declare which `wireFormatId` they use.

## 2. Registered formats

### 2.1 `kcir.wire.lenprefixed-ref.v1` (recommended)

Status: experimental (greenfield default for this bundle).

KCIR node wire layout:

```
envSig:32 || uid:32 || sort:1 || opcode:1 ||
outLen:varint || outRef:outLen ||
argsLen:varint || args ||
depsCount:varint ||
deps:(varint(depLen) || depRef:depLen){depsCount}
```

`outRef` and `depRef` are opaque byte strings interpreted by the active wire codec and profile.

Encoding primitive:
- `varint`: unsigned LEB128.

Verifiers MUST reject:
- truncated varints
- declared lengths that exceed available bytes
- non-canonical varints if the implementation enforces canonical varint form (RECOMMENDED)

### 2.2 `kcir.wire.legacy-fixed32.v1` (optional transitional)

Status: transitional.

KCIR node wire layout:

```
envSig:32 || uid:32 || sort:1 || opcode:1 ||
out:32 ||
argsLen:varint || args ||
depsCount:varint ||
deps:(depsCount * 32-byte digest)
```

This encoding assumes in-node references are represented as fixed 32-byte digests.
Profiles that use this format MUST define how digest-only payloads are lifted to full `Ref`.

## 3. Adding new formats

New entries SHOULD include:
- a canonical binary grammar,
- malformed-input rejection requirements,
- conformance vectors.
