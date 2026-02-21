---
slug: draft
shortname: WITNESS-ID
title: workingdoge.com/premath/WITNESS-ID
name: Deterministic Witness Identifiers
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - witnesses
  - determinism
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

This specification defines a deterministic method for computing `witnessId` values
for Gate and bidirectional/descent failure witnesses.

The goal is interoperability: two independent implementations given the same
semantic failure MUST produce identical witness IDs.

## 2. Canonical witness key

A witness object MAY contain many fields (human-readable messages, provenance,
implementation-local debug payloads). Only a stable subset contributes to `witnessId`.

The **canonical witness key** is the JSON object:

```json
{
  "schema": 1,
  "class": "...",
  "lawRef": "...",
  "tokenPath": "..." | null,
  "context": { ... } | null
}
```

Rules:

- `schema` MUST be the integer `1`.
- `class` MUST be the Gate failure class (e.g. `stability_failure`).
- `lawRef` MUST be the Gate law reference (e.g. `GATE-3.1`).
- `tokenPath` MUST be the witness token path string if present, else `null`.
- `context` MUST be the witness context object if present, else `null`.

Fields **excluded** from the witness key (do not affect `witnessId`):

- `message`
- `sources`
- `details`
- any implementation-local fields

This exclusion is intentional: implementations may vary wording and debug details
while remaining interoperable.

## 3. Canonical JSON encoding

The canonical witness key MUST be serialized using the JSON Canonicalization Scheme
(JCS), RFC 8785:

- UTF-8
- object keys sorted lexicographically
- no insignificant whitespace
- canonical number formatting

Let `keyBytes` denote the resulting UTF-8 byte string.

## 4. WitnessId computation

`witnessId` MUST be computed as:

```text
witnessId = "w1_" || base32hex_lower( SHA256(keyBytes) )
```

Where:

- `SHA256` is the SHA-256 hash function.
- `base32hex_lower` is RFC 4648 base32hex encoding using lowercase letters, without padding.

This function is used only to produce compact, deterministic identifiers; it is
not a security boundary.

## 5. Deterministic ordering tie-breaker

When witness arrays are required to be deterministically ordered and the ordering
keys are equal up to `context`, `witnessId` provides a canonical tie-breaker.

Implementations MUST compute `witnessId` before sorting.

## 6. Compatibility

- This spec is compatible with both set-level and higher-level Gate models.
- Implementations that cannot or do not wish to implement full RFC 8785 MAY
  implement an equivalent canonicalization procedure, but conformance suites
  MUST treat RFC 8785 behavior as the source of truth.
