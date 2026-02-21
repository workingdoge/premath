---
slug: draft
shortname: ERROR-CODES
title: workingdoge.com/premath/ERROR-CODES
name: KCIR Error Code Registry
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - kcir
  - errors
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

## 1. Requirement

Verifier outputs MUST include a stable machine-readable `code` selected from
this registry.

## 2. Core

- `kcir_v2.parse_error`
- `kcir_v2.env_uid_mismatch`
- `kcir_v2.dep_cycle`
- `kcir_v2.unsupported_sort`
- `kcir_v2.unsupported_opcode`
- `kcir_v2.contract_violation`

## 3. Reference/Profile

- `kcir_v2.profile_mismatch`
- `kcir_v2.params_hash_mismatch`
- `kcir_v2.domain_mismatch`
- `kcir_v2.digest_mismatch`
- `kcir_v2.evidence_malformed`
- `kcir_v2.evidence_invalid`
- `kcir_v2.anchor_mismatch`
- `kcir_v2.anchor_missing`

## 4. Store and Availability

- `kcir_v2.store_missing_node`
- `kcir_v2.store_missing_obj_nf`
- `kcir_v2.store_missing_mor_nf`
- `kcir_v2.data_unavailable`

## 5. Canonicality

- `kcir_v2.obj_nf_noncanonical`
- `kcir_v2.mor_nf_noncanonical`
