---
slug: draft
shortname: EVIDENCE-SITE
title: workingdoge.com/premath/EVIDENCE-SITE
name: Unified Evidence Plane Site Instantiation
status: draft
category: Standards Track
tags:
  - premath
  - site
  - evidence
  - sigpi
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

This specification is the concrete site instantiation of `draft/EVIDENCE-INF`
for premath's evidence fibres.

Dependencies:

- `draft/EVIDENCE-INF` (abstract evidence discipline),
- `draft/SIGPI-INF` (parent adjoint triple),
- `draft/DOCTRINE-SITE` (site object conventions).

This document is normative when capability `capabilities.unified_evidence` is
claimed.

## 2. Concrete evidence shape

The concrete `Ev` family for premath's control plane comprises:

| Evidence fibre | Shape | Factoring route |
| --- | --- | --- |
| CI witnesses | `ci.required.v1` witness payloads | `eta_CI : CI => Ev` via projection digest binding |
| Checker witnesses | Coherence-check verdict payloads | `eta_Chk : Chk => Ev` via obligation discharge lineage |
| Coherence witnesses | Gate-chain parity payloads | `eta_Coh : Coh => Ev` via gate-chain witness binding |
| Instruction witnesses | Instruction-envelope check payloads | `eta_Ins : Ins => Ev` via instruction witness lineage |
| Observation projections | Observation surface read-model payloads | `eta_Obs : Obs => Ev` via observation factoring (`draft/OBSERVATION-INF` §7) |

Each factoring route `eta_F` MUST be deterministic for fixed canonical inputs
(contract bytes + repository state + deterministic binding context).

## 3. Concrete factoring routes

### 3.1 CI witness factoring (`eta_CI`)

For each `ci.required.v1` witness artifact:

1. Extract canonical projection digest + binding tuple
   (`normalizerId`, `policyDigest`).
2. Compute typed-core projection digest via deterministic hash
   (`ev1_` prefix).
3. Factor into `Ev` via the typed-core projection.

### 3.2 Checker witness factoring (`eta_Chk`)

For each coherence-check verdict:

1. Extract obligation discharge set + binding tuple.
2. Compute canonical obligation-set digest.
3. Factor into `Ev` via obligation-set binding.

### 3.3 Observation factoring (`eta_Obs`)

Per `draft/OBSERVATION-INF` §7: the observation surface factors through `Ev`
with uniqueness for fixed `(W, I)`.

## 4. Concrete obstruction constructor mapping

Instantiation of `draft/EVIDENCE-INF` §2.4 for premath failure classes:

| Source failure class | Constructor | Canonical class |
| --- | --- | --- |
| `stability_failure` | `semantic(stability)` | `stability_failure` |
| `locality_failure` | `semantic(locality)` | `locality_failure` |
| `descent_failure` | `semantic(descent)` | `descent_failure` |
| `glue_non_contractible` | `semantic(contractibility)` | `glue_non_contractible` |
| `adjoint_triple_coherence_failure` | `semantic(adjoint_triple)` | `adjoint_triple_coherence_failure` |
| `coherence.cwf_substitution_identity.violation` | `structural(cwf_substitution_identity)` | `coherence.cwf_substitution_identity.violation` |
| `coherence.cwf_substitution_composition.violation` | `structural(cwf_substitution_composition)` | `coherence.cwf_substitution_composition.violation` |
| `coherence.span_square_commutation.violation` | `commutation(span_square_commutation)` | `coherence.span_square_commutation.violation` |
| `decision_witness_sha_mismatch` | `lifecycle(decision_attestation)` | `decision_witness_sha_mismatch` |
| `decision_delta_sha_mismatch` | `lifecycle(decision_delta_attestation)` | `decision_delta_sha_mismatch` |
| `unification.evidence_factorization.missing` | `lifecycle(evidence_factorization_missing)` | `unification.evidence_factorization.missing` |
| `unification.evidence_factorization.ambiguous` | `lifecycle(evidence_factorization_ambiguous)` | `unification.evidence_factorization.ambiguous` |
| `unification.evidence_factorization.unbound` | `lifecycle(evidence_factorization_unbound)` | `unification.evidence_factorization.unbound` |

## 5. Worker orchestration instantiation

Instantiation of `draft/EVIDENCE-INF` §3.2 for premath worker orchestration:

- **Context**: repository-state context family `C_cp`.
- **Cover**: worker-loop cover `{rho_i : Gamma_i -> Gamma}` from issue-memory
  decomposition.
- **Refinement**: per-worker issue-fibre refinement.
- **Glue**: deterministic compose-or-obstruction via checker/Gate authority.

## 6. Checker contract

### 6.1 Evidence factoring coherence

The checker validates:

1. Every claimed artifact family `F` has exactly one typed factoring route
   `eta_F` into `Ev`.
2. Factoring route digests are deterministic for fixed canonical inputs.
3. No artifact family has multiple inequivalent factoring routes.

### 6.2 Failure classes

- `unification.evidence_factorization.missing` — no typed `eta_F` route.
- `unification.evidence_factorization.ambiguous` — multiple inequivalent routes.
- `unification.evidence_factorization.unbound` — missing deterministic binding.

## 7. Non-goals

This document does not prescribe:

- specific witness storage backend or format,
- specific CI vendor integration,
- event streaming infrastructure,
- UI or frontend rendering of evidence surfaces.

It prescribes only the concrete evidence shape, factoring routes, obstruction
mapping, and checker contract for the unified evidence plane.
