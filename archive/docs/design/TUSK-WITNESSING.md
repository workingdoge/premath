# Tusk Witnessing

Status: draft
Scope: design-level, non-normative

## 1. Purpose

This document separates local admissibility witnessing from cross-world transport witnessing.

Without this split, SigPi failures get conflated with local Gate failures.

## 2. Witness classes

### 2.1 GateWitness (local world)

Used for admissibility checks inside one Premath world.

Failure classes align with Gate/Bidir mappings:

- `stability_failure`
- `locality_failure`
- `descent_failure`
- `glue_non_contractible`
- `adjoint_triple_coherence_failure` (optional capability)

### 2.2 TransportWitness (SigPi)

Used for inter-world transport/composition checks.

Recommended failure classes:

- `world_morphism_missing`
- `world_capability_mismatch`
- `world_policy_mismatch`
- `transport_context_unresolved`
- `transport_witness_unverifiable`
- `transport_non_composable`

These are not Gate failure classes.

## 3. Envelope split

### 3.1 GateWitnessEnvelope

Minimum shape:

```json
{
  "witnessSchema": 1,
  "witnessKind": "gate",
  "runId": "...",
  "worldId": "...",
  "contextId": "...",
  "intentId": "...",
  "adapterId": "...",
  "adapterVersion": "...",
  "ctxRef": "...",
  "dataHeadRef": "...",
  "normalizerId": "...",
  "policyDigest": "...",
  "result": "accepted | rejected",
  "failures": []
}
```

### 3.2 TransportWitnessEnvelope

Minimum shape:

```json
{
  "witnessSchema": 1,
  "witnessKind": "transport",
  "transportId": "...",
  "srcWorldId": "...",
  "dstWorldId": "...",
  "srcRunId": "...",
  "dstRunId": "...",
  "result": "accepted | rejected",
  "failures": []
}
```

## 4. Causal linkage

Transport envelopes should keep causal pointers to local Gate witnesses where relevant.

Local envelopes may include transport references, but local admissibility remains world-local.

## 5. Control-plane diagnostics

Control errors (timeouts, missing resources, leases, execution failures) should be emitted as diagnostics.

They should become Gate failures only when they imply a semantic law failure under declared checks.

Diagnostics should include attribution fields when available:

- `phase` (`restrict` | `compat` | `propose_glue` | `select_glue` | `normalize` | `transport`)
- `responsible_component` (`world` | `adapter` | `context_provider` | `event_store` | `transport`)

## 6. Determinism rules

Witness identifiers and ordering should be deterministic for fixed:

- identity fields,
- policy bindings,
- failure sets.

Use the deterministic witness ID rules from normative Premath specs for Gate-compatible witnesses.

## 7. Evidence profiles (representation-agnostic)

Tusk specifies evidence interfaces and admissibility meaning.
Tusk does not require one witness representation.

KCIR is an optional evidence profile, not a kernel requirement.

Recommended profile levels:

- Profile A (`opaque_witness`, v0 default):
  - world/local procedures and explicit equivalence witnesses,
  - no requirement to canonicalize adapter-owned payloads.
- Profile B (`kcir_lite`, optional):
  - selective KCIR fragments for portability/verification,
  - normal-form refs may appear for selected checks.
- Profile C (`kcir_full`, commitment-grade):
  - semantically relevant artifacts map to canonical KCIR,
  - deterministic hashing and commitment checkpoints enabled.

Capability negotiation keys should include:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`

Deterministic ID requirements always apply to world-owned/run identity material.
Canonical serialization of adapter-owned payloads is optional and profile-gated.

## 8. Non-bypass rule

A transport witness never grants local admissibility.

Transport can carry artifacts into a destination world, but destination admissibility must be checked under destination policy bindings.

TransportWitness certifies transport compatibility only.
Referencing GateWitness bundles does not upgrade their validity across worlds.
