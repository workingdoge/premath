---
slug: draft
shortname: CAPABILITY-VECTORS
title: workingdoge.com/premath/CAPABILITY-VECTORS
name: Optional Capability Conformance Vectors
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - conformance
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

This document defines a concrete vector checklist for optional capability claims.

Capabilities are intended to be **explicit**: unsupported optional branches MUST reject
deterministically (no silent fallback).

## 2. Capability matrix

Capability identifiers are exact claim tokens.
Implementations MAY expose them as namespaced manifest keys (for example
`capabilities.normal_forms`), but conformance checks them as exact identifiers.

### 2.1 `adoptPullAtomMor`

Meaning:

- MorNF tag `0x16` (`PullAtom`) is accepted (see `draft/NF`).
- MOR pull classification may use `PullAtom` fusion (see `draft/NORMALIZER`).

Required vectors when NOT claimed:

- adversarial: MorNF tag `0x16` rejected.
- adversarial: MOR pull steps that require PullAtom reject deterministically.

Required vectors when claimed:

- golden: MorNF tag `0x16` parses and binds.
- golden: normalization and pull-fusion behavior is deterministic.
- adversarial: malformed PullAtom payloads reject.

### 2.2 `hyperdescent`

Meaning:

- The implementation claims the optional hyperdescent strengthening specified in
  `raw/HYPERDESCENT`.

Required vectors when NOT claimed:

- none beyond base kernel vectors (hypercovers are out-of-scope unless claimed).

Required vectors when claimed:

- golden: at least one case where a represented hypercover descent check succeeds.
- adversarial: at least one case where Čech descent holds but hyperdescent fails.
- determinism: witness IDs and ordering are stable across runs.

### 2.3 `universe`

Meaning:

- The implementation claims the optional universe/comprehension extension specified in
  `raw/UNIVERSE`.

This repository bundle does not yet standardize an operational code format for universes.
Vectors for this capability are deferred until a code/cert format is specified.

### 2.4 `capabilities.normal_forms`

Meaning:

- The implementation supports an explicit normalized-comparison capability for
  witness/discharge flows.
- In normalized mode, outputs are bound to `normalizerId` and `policyDigest`
  per `draft/NORMALIZER` and `draft/BIDIR-DESCENT`.

Required vectors when NOT claimed:

- adversarial: explicit requests that require normalized mode reject deterministically.

Required vectors when claimed:

- golden: same semantic input yields stable `(cmpRef, normalizerId, policyDigest)` across runs.
- golden: normalized-mode equivalence checks accept known equivalent forms.
- adversarial: policy/normalizer binding mismatch rejects deterministically.

### 2.5 `capabilities.kcir_witnesses`

Meaning:

- The implementation can emit/consume KCIR-linked witness evidence for portability.
- This capability augments witness representation; it does not change Gate semantics.

Required vectors when NOT claimed:

- adversarial: explicit requests for KCIR-linked witness payloads reject deterministically.

Required vectors when claimed:

- golden: emitted KCIR-linked witness references resolve and verify per
  `draft/KCIR-CORE` and `draft/REF-BINDING`.
- adversarial: missing or tampered witness references reject deterministically.
- invariance: for the same semantic failure, Gate class is identical between
  opaque-witness output and KCIR-witness output.

### 2.6 `capabilities.commitment_checkpoints`

Meaning:

- The implementation supports checkpoint artifacts that bind run/witness material
  to commitment references for audit/transport.
- This capability adds audit transport structure only; it does not change Gate semantics.

Required vectors when NOT claimed:

- adversarial: explicit checkpoint create/verify requests reject deterministically.

Required vectors when claimed:

- golden: checkpoint creation and verification succeed for valid artifacts.
- adversarial: tampered checkpoint payload or reference mismatch rejects deterministically.
- invariance: kernel verdict and Gate failure classes are identical with and
  without checkpoint generation.

### 2.7 `capabilities.change_morphisms`

Meaning:

- The implementation supports deterministic change projection morphisms
  (`Delta -> requiredChecks`) with stable projection digest material.
- Provider-wrapper environments (local and mapped external env) preserve the
  same projection/references for the same semantic delta.
- This capability expresses operational change-morphism discipline for gate
  selection; it does not alter kernel admissibility semantics.

Required vectors when NOT claimed:

- adversarial: explicit requests for change-morphism projection checks reject
  deterministically.

Required vectors when claimed:

- golden: deterministic required-check projection for representative deltas
  (docs-only, kernel-touch, conformance-touch, unknown-surface fallback,
  mixed known+unknown fail-closed baseline fallback).
- golden: provider env mapping (direct vs mapped GitHub env) yields equivalent
  projection/reference material.
- golden: work-memory mutation morphisms preserve deterministic claim/discover
  transitions (`issue_claim` and `issue_discover` non-loss linkage), including
  lease binding (`lease_id`, owner, expiry) for multiagent claim discipline.
- golden: deterministic lease lifecycle mutations preserve coherent claim
  ownership transitions for `issue_lease_renew` and `issue_lease_release`
  operations.
- golden: deterministic lease projection separates stale leases from contended
  active leases.
- golden: CLI issue command-surface parity preserves coherent
  `issue_ready`/`issue_blocked` partition semantics over the same graph state.
- adversarial: requesting change-morphism projection checks without claim rejects
  deterministically.
- adversarial: active lease contention rejects deterministically
  (`lease_contention_active`).
- adversarial: stale renew or mismatched lease-owner/lease-id release requests
  reject deterministically (`lease_stale`, `lease_owner_mismatch`,
  `lease_id_mismatch`).
- adversarial: work-memory discover morphism rejects when parent lineage is
  missing.
- adversarial: incoherent stale/contended lease projection expectations reject
  deterministically (`lease_stale_set_mismatch`, `lease_contended_set_mismatch`).
- adversarial: incoherent `issue_ready`/`issue_blocked` partition expectations
  reject with deterministic failure classification (`issue_ready_set_mismatch`
  and `issue_blocked_set_mismatch`).
- invariance: paired profile outputs for the same semantic scenario preserve
  kernel verdict and Gate failure classes (local/external and provider-wrapper
  invariance).

### 2.8 `capabilities.squeak_site`

Meaning:

- The implementation supports runtime-location site checks for Squeak/SigPi
  placement and overlap/glue contracts as described in `raw/SQUEAK-SITE`.
- This capability validates site-level runtime evidence consistency; it does not
  redefine local Gate admissibility semantics.

Required vectors when NOT claimed:

- adversarial: explicit requests for SqueakSite-linked runtime evidence reject
  deterministically.

Required vectors when claimed:

- golden: equivalent location descriptors yield deterministic `loc_id` material.
- golden: overlap agreement checks accept when required checks and bindings align.
- adversarial: overlap mismatches reject deterministically (`site_overlap_mismatch`).
- adversarial: non-contractible glue proposals reject deterministically
  (`site_glue_non_contractible`).
- invariance: local and external runtime profiles preserve kernel verdict and
  Gate failure classes for the same semantic scenario.

### 2.9 `capabilities.ci_witnesses`

Meaning:

- The implementation supports instruction-envelope CI witness artifacts in the
  higher-order CI loop (`raw/PREMATH-CI`).
- This capability checks:
  - deterministic instruction-witness binding, and
  - deterministic required-gate witness verification/decision attestation over
    projected checks.
- It does not alter kernel admissibility semantics.

Required vectors when NOT claimed:

- adversarial: explicit requests for CI witness determinism/verification checks
  reject deterministically.

Required vectors when claimed:

- golden: same instruction envelope yields stable verdict class and stable
  required/executed check sets.
- golden: required-gate witness verification succeeds for matching projection,
  gate witness refs, and native required-check bindings.
- golden: strict-delta compare and decision-attestation chain are stable for
  fixed inputs.
- adversarial: mismatched verdict class or required/executed check sets for the
  same instruction envelope reject deterministically.
- adversarial: required-gate witness digest/source/projection mismatches reject
  deterministically.
- invariance: local/external execution profiles preserve kernel verdict and Gate
  failure classes for paired instruction and required-gate scenarios.

### 2.10 `capabilities.instruction_typing`

Meaning:

- The implementation supports doctrine-level instruction typing for control-loop
  inputs (`typed(kind)` vs `unknown(reason)`) as defined by
  `draft/LLM-INSTRUCTION-DOCTRINE` and typed proposal ingestion/checking
  discipline as defined by `draft/LLM-PROPOSAL-CHECKING`.
- This capability validates typed/unknown classification determinism and
  explicit unknown routing policy; it does not alter kernel admissibility
  semantics.

Required vectors when NOT claimed:

- adversarial: explicit requests for instruction typing checks reject
  deterministically.

Required vectors when claimed:

- golden: fixed instruction envelope and fixed policy produce deterministic
  `typed(kind)` classification.
- golden: fixed typed LLM proposal payload with fixed binding material produces
  deterministic proposal canonicalization/checking outcomes.
- adversarial: `unknown(reason)` without explicit policy route rejects
  deterministically.
- adversarial: proposals missing `normalizerId`/`policyDigest` binding reject
  deterministically.
- adversarial: derivation proposals with invalid/unreplayable steps reject
  deterministically.
- adversarial: proposal digest/canonicalization nondeterminism rejects
  deterministically.
- invariance: local/external instruction-typing execution profiles preserve
  kernel verdict and Gate failure classes for paired scenarios.

### 2.11 `capabilities.adjoints_sites`

Meaning:

- The implementation supports the claimed `profile/ADJOINTS-AND-SITES` overlay
  obligation surface in executable form, bound to `(normalizerId, policyDigest)`.
- This capability validates deterministic obligation compilation/discharge for:
  `adjoint_triangle`, `beck_chevalley_sigma`, `beck_chevalley_pi`,
  and `refinement_invariance`.

Required vectors when NOT claimed:

- adversarial: explicit requests for adjoints/sites obligation checks reject
  deterministically.

Required vectors when claimed:

- golden: fixed refinement-plan proposal material compiles/discharges the
  required adjoint/site obligation set deterministically.
- adversarial: missing `adjoint_triangle` evidence rejects deterministically.
- adversarial: missing `beck_chevalley_sigma` evidence rejects deterministically.
- adversarial: missing `beck_chevalley_pi` evidence rejects deterministically.
- adversarial: missing `refinement_invariance` evidence rejects deterministically.
- invariance: local/external adjoints-sites execution profiles preserve kernel
  verdict and Gate failure classes for paired scenarios.

## 3. Fixture naming guidance (informative)

Implementations SHOULD use stable fixture IDs that encode:

- mode (`nf`, `opcode`, `core-verify`, `gate-verify`)
- capability (`pull_atom`)
- expectation (`ok`, `reject`, etc.)
