---
slug: draft
shortname: PREMATH-COHERENCE
title: workingdoge.com/premath/PREMATH-COHERENCE
name: Premath Coherence Contract and Checker Witness
status: draft
category: Standards Track
tags:
  - premath
  - coherence
  - checker
  - witness
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

This specification defines a typed coherence artifact and checker surface for
repository-level control-plane consistency.

The checker is not a replacement for kernel authority (`draft/PREMATH-KERNEL`,
`draft/GATE`, `draft/BIDIR-DESCENT`). It is a deterministic adapter that:

1. compiles repository coherence claims into finite obligations,
2. discharges those obligations against declared operational/doc surfaces, and
3. emits a deterministic witness (`premath.coherence.v1`).

## 2. Coherence Contract Artifact

The machine artifact is `draft/COHERENCE-CONTRACT.json`.

A conforming artifact MUST contain at least:

- contract identity (`contractKind`, `contractId`, `schema`),
- binding material (`binding.normalizerId`, `binding.policyDigest`),
- declared obligations (ordered list),
- declared operational/doc surface references used by the checker.

Contract bindings are checker bindings only. They do not redefine kernel
normalization/equality semantics.

## 3. Obligation Set (normative)

A conforming checker MUST support at least the following obligation IDs:

1. `scope_noncontradiction`
2. `capability_parity`
3. `gate_chain_parity`
4. `operation_reachability`
5. `overlay_traceability`

Unknown obligation IDs in the contract MUST reject deterministically.
Missing required obligation IDs MUST reject deterministically.

## 4. Obligation Semantics

### 4.1 `scope_noncontradiction`

MUST reject when contractually declared normative/informative scope constraints
contradict the indexed spec surfaces.

Minimum checks:

- capability-scoped conditional normativity clauses in `draft/SPEC-INDEX`
  remain coherent,
- informative fallback clause remains present,
- bidirectional checker surface stays aligned with `draft/BIDIR-DESCENT`
  obligation vocabulary for the required set declared in the contract.

The bidirectional checker alignment is a parity check over obligation kinds; it
does not authorize or alter discharge semantics.

### 4.2 `capability_parity`

MUST reject when executable capability IDs and documented capability IDs drift.

Minimum parity set includes:

- executable source capability tuple,
- capability manifest set,
- primary docs surfaces listed in the contract (including `README` and
  `SPEC-INDEX` capability section).

### 4.3 `gate_chain_parity`

MUST reject when documented gate chain surfaces drift from executable sources.

Minimum parity set includes:

- baseline task composition parity (`.mise.toml` vs CI closure docs),
- deterministic projected check ID parity (`tools/ci/change_projection.py`
  vs CI closure docs).

### 4.4 `operation_reachability`

MUST reject when required operation paths are missing, unregistered in doctrine
site nodes, or unreachable from declared doctrine root through deterministic
cover/edge traversal.

### 4.5 `overlay_traceability`

MUST reject when declared profile overlays are missing from:

- profile file surface,
- `SPEC-INDEX` overlay section,
- profile README registry surface.

## 5. Deterministic Failure Classes

The checker MUST emit deterministic failure classes prefixed by:

- `coherence.contract.*` for contract-shape/obligation-set failures,
- `coherence.<obligation_id>.*` for obligation discharge failures.

For the same contract bytes and repository state, failure class output MUST be
stable.

## 6. Witness Schema (normative)

A successful checker execution MUST emit one witness object with at least:

- `schema`
- `witnessKind = "premath.coherence.v1"`
- `contractKind`
- `contractId`
- `contractRef`
- `contractDigest`
- `binding.{normalizerId,policyDigest}`
- `result` (`accepted|rejected`)
- `obligations[]`
  - `obligationId`
  - `result` (`accepted|rejected`)
  - `failureClasses[]`
  - `details` (deterministic JSON payload)
- `failureClasses[]` (deduplicated union of obligation failures)

Witness emission MUST be deterministic for fixed inputs.

## 7. Command Surface

The canonical checker command surface is:

- `premath coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root .`

Repository task wrapper:

- `mise run coherence-check`

## 8. Relationship to Kernel and BIDIR

This checker does not synthesize or discharge semantic Gate obligations over
Premath terms. It validates coherence of repository control-plane surfaces.

`draft/BIDIR-DESCENT` remains the authority for synthesis/checking/discharge of
semantic obligations.

The coherence checker may verify that bidirectional checker operation surfaces
remain vocabulary-aligned; this is consistency checking, not semantic
admissibility.

## 9. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.presentation.projection` (surface parity over projected contracts)
- `dm.commitment.attest` (witness binding/digest emission)

Not preserved:

- `dm.refine.context`
- `dm.refine.cover`
- `dm.transport.location`
- `dm.transport.world`
