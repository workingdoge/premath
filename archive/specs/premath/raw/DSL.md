---
slug: raw
shortname: DSL
title: workingdoge.com/premath/DSL
name: Dependency Pattern DSL
status: raw
category: Standards Track
tags:
  - premath
  - kernel
  - dsl
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

KCIR nodes reference dependency certificates by commitment reference (`Ref`).
Each dependency, once verified, yields a record including:

- `cid: Ref` (the dependency certificate reference)
- `sort: u8`
- `opcode: u8`
- `out: Ref`
- `meta: map` (opcode-defined metadata)

Many opcodes require dependencies that play **roles** (e.g. “the cover pull”,
“the mkTensor witness”, “the k-th local pull”).

This document defines a **dependency-pattern DSL** for matching and validating
such roles.

The DSL is used by opcode contracts (see `raw/OPCODES`).

## 2. Determinism requirement

Given:

- a list of verified dependency records `deps[]` in the order they are listed in
  the KCIR node,
- a pattern `specs[]`, and
- any required `bindings` produced while matching,

a conforming implementation MUST either:

- deterministically produce a unique set of bindings (including ordered lists
  for bag matches), or
- reject.

Implementations MUST NOT accept ambiguous matches.

## 3. Common definitions

### 3.1 Predicates

A predicate is a function `Pred(dep) -> bool`.

Predicates commonly check:

- `dep.sort`, `dep.opcode`
- fields inside `dep.meta`
- (optionally) `dep.out`

### 3.2 Keys

A `key_of(dep)` function maps a dep to a key (any comparable value).
Keys are used to match deps to expected roles.

### 3.3 Positions

A `pos` value is one of:

- `"anywhere"`
- `"first"`
- `"last"`
- `i` (an integer start index)
- `("suffix", i)` (alias for integer start index)

If `pos` is an integer or suffix form, it refers to an index in the current
**remaining** list (for UniqueSpec), or in the filtered list (for MultiBagSpec
pool slicing).

## 4. Specs

### 4.1 UniqueSpec

A UniqueSpec binds a single dependency to a role.

Fields:

- `name: string`
- `pred: Pred`
- `pos: pos` (default `"anywhere"`)
- `optional: bool` (default `false`)

Semantics:

- If `pos` is `"first"`, the first remaining dep MUST satisfy `pred`.
- If `pos` is `"last"`, the last remaining dep MUST satisfy `pred`.
- If `pos` is an integer (or suffix alias), the dep at that index MUST satisfy
  `pred`.
- If `pos` is `"anywhere"`, exactly one remaining dep MUST satisfy `pred`.

If `optional=false`, failure to find a match (or finding multiple matches in
`"anywhere"` mode) MUST be a rejection.

On success, the matched dep is removed from the remaining list.

### 4.2 BagSpec

A BagSpec binds a list of deps corresponding to a list of expected keys.

Fields:

- `name: string`
- `expected_keys: [Key]` OR `(bindings -> [Key])`
- `key_of: dep -> Key`
- `pred: Pred`
- `mode: "ordered" | "unordered"`
- `pos: pos` (default `"anywhere"`)

Semantics:

Let `E = expected_keys` evaluated under current bindings.
Let `k = len(E)`.

**Slice selection** (`pos` is `"first"`, `"last"`, integer, or suffix):

- Select a slice `S` of length `k` from the remaining list.
- Every dep in `S` MUST satisfy `pred`.
- If `mode="ordered"`, `key_of(S[i]) MUST equal E[i]` for all i.
- If `mode="unordered"`, the multiset of keys of `S` MUST equal the multiset
  of `E`, and the bound list MUST be returned in canonical order `E`.

**Pool selection** (`pos="anywhere"`):

- If `mode="unordered"`, select exactly the required multiplicities of deps
  matching keys in `E`, leaving all others as remaining.
- If `mode="ordered"`, select a subsequence matching `E` in order.

In all modes, multiplicities MUST be respected (duplicate keys are allowed).
Ambiguity MUST be rejected.

### 4.3 MultiBagSpec

A MultiBagSpec partitions a pool of dependencies into multiple bag roles
simultaneously, using an exact bipartite matching.

Fields:

- `name: string`
- `bags: [BagSpec]` (each bag MUST have `mode="unordered"` and `pos="anywhere"`)
- `pool_pred: Pred OR (bindings -> Pred)` (default: `true`)
- `pos: pos` (default `"anywhere"`)
- `pool_k: int | "all" | (bindings -> int|"all")` (optional)
- `consume_all: bool` (default `true`)
- `domain_pred: Pred OR (bindings -> Pred)` (optional)

Pool selection:

1. Compute `pool_pred` under bindings.
2. Filter remaining deps in order: `filtered = [d in remaining | pool_pred(d)]`.
3. If `pos="anywhere"`, then `pool = filtered`.
4. Else select a slice of `filtered`:
   - let `nSlots` = total expected key occurrences across all bags
   - let `k` = `pool_k` if provided else `nSlots`
   - if `pool_k="all"`, `k` is the length of the selected segment (entire list
     for `first/last`, or suffix from start index)
   - select `pool = filtered[pos : pos+k]` (with the appropriate meaning of
     `first/last`)

Matching:

- Each expected key occurrence across all bags is treated as a distinct **slot**.
- A dep may match a slot if:
  - it satisfies that bag's `pred`, and
  - `key_of(dep) == slotKey`.
- The implementation MUST find a matching that fills all slots.
- If no matching exists, reject.

Bindings:

- Each bag name binds a list of deps in the canonical expected-keys order for
  that bag.
- Only matched deps are removed from remaining.

Consume-all rule:

- If `consume_all=true`, then any unused dep in the selected pool that satisfies
  `domain_pred` MUST cause rejection.
- If `domain_pred` is omitted, it defaults to `OR(bag.pred)`.

## 5. Matching API

This spec assumes an API like:

- `match_pattern(deps, specs, allow_extra)` returning `(bindings, remaining)`

A conforming implementation MUST reject if `allow_extra=false` and the remaining
list is non-empty.
