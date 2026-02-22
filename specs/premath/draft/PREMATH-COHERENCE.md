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

Within the control-plane layer, this checker is the **check role**. Execution
and attestation transport roles are defined by `raw/PREMATH-CI` and
`raw/CI-TOPOS`.

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
6. `transport_functoriality`
7. `span_square_commutation`
8. `coverage_base_change`
9. `coverage_transitivity`
10. `glue_or_witness_contractibility`
11. `cwf_substitution_identity`
12. `cwf_substitution_composition`
13. `cwf_comprehension_beta`
14. `cwf_comprehension_eta`

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
  obligation vocabulary for the required set declared in the contract, sourced
  from kernel authority registry export (`premath obligation-registry --json`),
- this spec's required obligation list in §3 stays aligned with the executable
  required obligation set used by `premath coherence-check`.

The bidirectional checker alignment is a parity check over obligation kinds; it
does not authorize or alter discharge semantics.

### 4.2 `capability_parity`

MUST reject when executable capability IDs and documented capability IDs drift.

Minimum parity set includes:

- typed executable capability registry
  (`draft/CAPABILITY-REGISTRY.json`),
- capability manifest set,
- primary docs surfaces listed in the contract (including `README` and
  `SPEC-INDEX` capability section).

### 4.3 `gate_chain_parity`

MUST reject when documented gate chain surfaces drift from executable sources.

Minimum parity set includes:

- baseline task composition parity (`.mise.toml` vs CI closure docs),
- deterministic projected check ID parity (`draft/CONTROL-PLANE-CONTRACT.json`
  required-gate projection order vs CI closure docs),
- shared control-plane witness/policy identifiers are present and well-formed
  in `draft/CONTROL-PLANE-CONTRACT.json` (`requiredWitness`,
  `instructionWitness`).

### 4.4 `operation_reachability`

MUST reject when required operation paths are missing, unregistered in doctrine
site nodes, or unreachable from declared doctrine root through deterministic
cover/edge traversal.

### 4.5 `overlay_traceability`

MUST reject when declared profile overlays are missing from:

- profile file surface,
- `SPEC-INDEX` overlay section,
- profile README registry surface.

### 4.6 `transport_functoriality`

MUST reject when executable transport fixtures violate deterministic base/fibre
transport laws:

- identity preservation,
- composition preservation,
- naturality square equality.

### 4.7 `span_square_commutation`

MUST reject when span/square fixtures fail typed commutation constraints for the
pipeline/base-change witness layer:

- each square edge references declared spans,
- square witness digest is deterministic and matches its canonical fields,
- accepted squares have empty failure classes and commuting top/bottom span
  semantics,
- rejected squares carry non-empty failure classes.

This obligation keeps pipeline + base-change witness squares inside the same
deterministic coherence contract surface (no side-channel planner authority).

### 4.8 `coverage_base_change`

MUST reject when admissible cover pullback fixtures violate base-change
stability under refinement maps.

### 4.9 `coverage_transitivity`

MUST reject when composed-cover fixtures violate transitivity of covers under
refinement-of-cover composition.

### 4.10 `glue_or_witness_contractibility`

MUST reject when descent fixtures fail deterministic glue-or-obstruction shape:

- glue and obstruction both present, or
- glue and obstruction both absent, or
- declared glue/obstruction evidence is structurally invalid for the fixture
  contract.

### 4.11 `cwf_substitution_identity`

MUST reject when strict substitution identity equalities fail on fixture
rows:

- `A[id] = A` for type rows, and
- `t[id] = t` for term rows.

This obligation is a strict (definitional) presentation boundary for
substitution identity in the CwF operational lane.

### 4.12 `cwf_substitution_composition`

MUST reject when strict substitution composition equalities fail on fixture
rows:

- `A[f ∘ g] = A[f][g]` for type rows, and
- `t[f ∘ g] = t[f][g]` for term rows.

This obligation is a strict (definitional) presentation boundary for
substitution composition in the CwF operational lane.

### 4.13 `cwf_comprehension_beta`

MUST reject when strict comprehension beta equalities fail on fixture rows:

- `q[⟨id, a⟩] = a`.

### 4.14 `cwf_comprehension_eta`

MUST reject when strict comprehension eta equalities fail on fixture rows:

- `⟨π ∘ σ, q[σ]⟩ = σ`.

### 4.15 Site Vector Polarity Coverage

For obligations discharged from `coherence-site` fixtures, checker input MUST
include at least one matched `golden/` vector and at least one matched
`adversarial/` vector for each obligation id.

Checker input MUST also include semantic polarity coverage from `expect.result`:

- at least one matched vector with `expectedResult = accepted`,
- at least one matched vector with `expectedResult = rejected`.

Missing either path polarity or semantic-result polarity MUST reject
deterministically.

`coherence-site/manifest.json` MUST provide `obligationVectors` mapping each
site obligation id to the exact vector ids discharged by that obligation.
Checkers MUST scope vector parsing/evaluation to that map so malformed vectors
outside an obligation scope do not fail unrelated obligations.

For vectors with `invariance/` prefix, case payloads MUST include non-empty:

- `semanticScenarioId`
- `profile`

For each obligation id and each `semanticScenarioId`, checker input MUST include
exactly two invariance vectors with distinct `profile` values. Those two vectors
MUST evaluate to the same `actualResult` and the same
`actualFailureClasses` set.

Violations of this invariance-pair contract MUST reject deterministically.

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

## 10. Migration Profile: Python Adapters -> `premath-coherence` Core

This section defines the phased migration contract for moving control-plane
checker semantics out of Python surfaces and into the Rust
`premath-coherence` core.

### 10.1 Authority boundary

During and after migration:

- semantic admissibility authority MUST remain in
  `draft/PREMATH-KERNEL` + `draft/GATE` + `draft/BIDIR-DESCENT`,
- control-plane checker semantics MAY live in `premath-coherence` (or successor
  checker crates),
- CI execution/attestation semantics MUST remain in
  `raw/PREMATH-CI` + `raw/CI-TOPOS` and their operational command surfaces,
- Python surfaces under `tools/ci/` MUST remain orchestration adapters
  (argument/env binding, command dispatch, summary shaping),
- adapters MUST NOT define parallel canonicalization/typing/discharge logic.

### 10.2 Parity contract

For identical input envelope + repository state + bindings, dual-path execution
MUST preserve the same authoritative outcome class and checker lineage.

Minimum parity keys:

- `result` (`accepted|rejected`),
- `failureClasses` (set equality),
- checker binding fields (`normalizerId`, `policyDigest`),
- required check ID set + deterministic ordering policy,
- projection/check digests emitted by the control plane,
- proposal identity keys (`proposalDigest`, `proposalKcirRef`) when present.

### 10.3 Phased cutover

Implementations SHOULD use this cutover sequence:

1. `phase_0_inventory`: enumerate Python semantic/check logic and map each
   boundary to target core APIs.
2. `phase_1_shadow`: run core checker in parallel and emit parity comparison
   witnesses without changing gate verdicts.
3. `phase_2_gate_on_parity`: treat parity mismatch as deterministic reject.
4. `phase_3_primary_cutover`: make core checker the primary authority path;
   keep adapter fallback path available.
5. `phase_4_deprecate_legacy`: remove duplicated legacy semantic paths once
   parity has remained stable over sustained baseline runs.

### 10.4 Rollback safety

Rollback MUST preserve witness reproducibility:

- rollback switches execution path only; it MUST NOT mutate instruction payload,
  policy binding, or normalizer binding material,
- parity-mismatch failures MUST emit typed witnesses before fallback use,
- fallback mode MUST remain doctrine-gated and fail-closed.
