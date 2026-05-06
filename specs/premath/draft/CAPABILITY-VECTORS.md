---
slug: draft
shortname: CAPABILITY-VECTORS
title: workingdoge.com/premath/CAPABILITY-VECTORS
name: Optional Capability Checker Vectors
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - checker-claims
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

Active capability identifiers are exact claim tokens declared in
`draft/CAPABILITY-REGISTRY.json`.
Implementations MAY expose them as namespaced manifest keys (for example
`capabilities.normal_forms`), but checker checks them as exact identifiers.

Sections 2.1-2.3 are inactive extension notes retained to preserve pre-registry
design intent. They are not active capability claim tokens and MUST NOT be
asserted as checker claims unless a future `draft/CAPABILITY-REGISTRY.json`
entry promotes them.

### 2.1 Inactive extension note: `capabilities.pull_atom_mor`

Meaning:

- MorNF tag `0x16` (`PullAtom`) is accepted (see `draft/NF`).
- MOR pull classification may use `PullAtom` fusion (see `draft/NORMALIZER`).
- Former draft flag: `adoptPullAtomMor`.
- Current status: not an active checker-claims surface.

Future vectors if promoted and NOT claimed:

- adversarial: MorNF tag `0x16` rejected.
- adversarial: MOR pull steps that require PullAtom reject deterministically.

Future vectors if promoted and claimed:

- golden: MorNF tag `0x16` parses and binds.
- golden: normalization and pull-fusion behavior is deterministic.
- adversarial: malformed PullAtom payloads reject.

### 2.2 Inactive extension note: `hyperdescent`

Meaning:

- The optional hyperdescent strengthening specified in `raw/HYPERDESCENT`.
- Current status: raw proposal, not an active checker-claims surface.

Future vectors if promoted and NOT claimed:

- none beyond base kernel vectors (hypercovers are out-of-scope unless claimed).

Future vectors if promoted and claimed:

- golden: at least one case where a represented hypercover descent check succeeds.
- adversarial: at least one case where Čech descent holds but hyperdescent fails.
- determinism: witness IDs and ordering are stable across runs.

### 2.3 Inactive extension note: `universe`

Meaning:

- The optional universe/comprehension extension specified in `raw/UNIVERSE`.
- Current status: raw proposal, not an active checker-claims surface.

This repository bundle does not yet standardize an operational code format for universes.
Vectors for this capability are deferred until a code/cert format is specified.

### 2.4 `capabilities.normal_forms`

Meaning:

- The implementation supports an explicit normalized-comparison capability for
  witness/discharge flows.
- In normalized mode, outputs are bound to `normalizerId` and `policyDigest`
  per `draft/NORMALIZER` and `profile/interop/BIDIR-DESCENT`.

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
  (docs-only, kernel-touch, checker-touch, unknown-surface fallback,
  mixed known+unknown fail-closed baseline fallback).
- golden: provider env mapping (direct vs mapped GitHub env) yields equivalent
  projection/reference material.
- adversarial: requesting change-morphism projection checks without claim rejects
  deterministically.
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
- adversarial: missing glue proposals reject deterministically (`site_glue_missing`).
- adversarial: non-contractible glue proposals reject deterministically
  (`site_glue_non_contractible`).
- invariance: local and external runtime profiles preserve kernel verdict and
  Gate failure classes for the same semantic scenario.

### 2.9 `capabilities.instruction_typing`

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

### 2.10 `capabilities.adjoints_sites`

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
- when composed with `capabilities.squeak_site`, golden vectors MUST show
  cross-lane pullback/base-change routing through `span_square_commutation` and
  deterministic runtime location binding.
- when composed with `capabilities.squeak_site`, adversarial vectors MUST reject
  missing cross-lane route binding and transport reference mismatches
  deterministically.
- when composed with `capabilities.squeak_site`, invariance vectors MUST
  preserve kernel verdict and Gate failure classes across local/external
  profiles for the same composed semantic scenario.

## 3. Fixture naming guidance (informative)

Implementations SHOULD use stable fixture IDs that encode:

- mode (`nf`, `opcode`, `core-verify`, `gate-verify`)
- capability (`pull_atom`)
- expectation (`ok`, `reject`, etc.)
