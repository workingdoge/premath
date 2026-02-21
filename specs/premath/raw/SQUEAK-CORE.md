---
slug: raw
shortname: SQUEAK-CORE
title: workingdoge.com/premath/SQUEAK-CORE
name: Squeak Core Transport Contracts
status: raw
category: Standards Track
tags:
  - premath
  - squeak
  - sigpi
  - transport
  - composition
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

This specification defines `squeak-core`: transport and composition contracts
between Premath worlds.

Boundary:

- `raw/TUSK-CORE` governs execution inside one world.
- `raw/SQUEAK-CORE` governs world-to-world mapping and transport.
- `raw/SQUEAK-SITE` governs runtime-location site structure (`Loc`, covers, overlap/glue).

`Squeak` is the operational SigPi layer.

## 2. World descriptor

A transport endpoint MUST publish a deterministic world descriptor:

```text
WorldDescriptor {
  world_id
  kernel_profile
  capability_vector
  adapter_set
  witness_schema
  comparison_modes
}
```

Where `comparison_modes` includes supported `normalizer_id` and policy-binding
compatibility declarations.

## 3. Negotiation contract

Before transport, implementations MUST perform deterministic negotiation over:

- source/destination capability compatibility,
- witness schema compatibility,
- comparison mode compatibility (`normalizer_id`, policy bindings),
- adapter/interpreter compatibility when transporting adapter-scoped artifacts.

Negotiation mismatch MUST reject deterministically and emit transport failures.

## 4. Transport interfaces

Logical interface:

```text
map_context(src_world, dst_world, src_context_id, src_ctx_ref) -> (dst_context_id, dst_ctx_ref)
transport_summary(src_world, dst_world, summary) -> dst_summary
transport_obligation(src_world, dst_world, obligation) -> dst_obligation
transport_witness(src_world, dst_world, witness_ref) -> dst_witness_ref
```

Context mapping MUST preserve lineage attribution material sufficient for
destination-side diagnostics.

Transported artifacts MUST include source lineage pointers (`src_world_id`,
`src_run_id` or equivalent) for auditability.

### 4.1 Destination handoff (required)

After transport, Squeak MUST hand destination-scoped artifacts to destination
`tusk-core` admissibility checks.

Squeak MAY orchestrate this handoff directly or by delegation, but there is no
separate semantic “bridge” authority. Handoff is part of Squeak transport
responsibility.

### 4.2 Runtime unit contract (`Cheese`)

Implementations MAY expose Squeak runtime units called `Cheese` for execution
substrate placement/orchestration.

Minimal logical interface:

```text
resolve_cheese(world, runtime_profile) -> CheeseDescriptor
run_on_cheese(cheese, task_ref, env_ref) -> ExecutionResult
```

where:

```text
CheeseDescriptor {
  cheese_id
  runtime_profile
  capability_vector
  substrate_binding_ref
}
```

`Cheese` orchestration is operational only and MUST NOT redefine semantic
admissibility or Gate-class outcomes.

Location covers, overlap agreement, and site-level glue contracts for Cheese
runtime placement are specified in `raw/SQUEAK-SITE`.

## 5. Composition laws

Squeak transport composition SHOULD satisfy:

- identity coherence,
- composition coherence,
- witness lineage preservation.

If any composition law is not satisfied for an attempted transport chain,
implementation MUST reject with `transport_non_composable`.

## 6. Witness model

Squeak MUST emit transport-class witnesses distinct from Gate witnesses.

Recommended failure classes:

- `world_morphism_missing`
- `world_capability_mismatch`
- `world_policy_mismatch`
- `transport_context_unresolved`
- `transport_witness_unverifiable`
- `transport_non_composable`

Transport witnesses MAY reference Gate witness bundles, but MUST NOT relabel
transport failures as Gate failures.

## 7. Non-bypass rule

Squeak transport NEVER grants local admissibility.

Destination-world admissibility MUST still be checked by destination `tusk-core`
under destination policy bindings.

TransportWitness certifies transport compatibility only.

## 8. Squeak packet shape (v0)

A minimal v0 transport packet SHOULD include:

```text
SqueakPacket {
  transport_id
  src_world_id
  dst_world_id
  src_run_id
  src_context_id
  src_ctx_ref
  payload_kind: summary | obligation | witness
  payload_ref
  negotiation_digest
}
```

`transport_id` MUST be deterministic for fixed packet contents.

## 9. Determinism and retries

Transport replay MUST be deterministic for fixed source packet and destination
world descriptor.

Retries MUST be idempotent at the transport identity layer; duplicate delivery
MUST NOT create semantically distinct destination artifacts.

## 10. Security and robustness

Implementations MUST treat transported payloads and witness references as
untrusted.

Implementations SHOULD:

- enforce explicit allow-lists for accepted capability vectors,
- reject ambiguous context mappings,
- retain transport lineage logs for post-failure diagnosis.

## 11. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.transport.world`
- `dm.profile.evidence` (transport/evidence mapping must not change Gate class claims)

Not preserved:

- `dm.refine.context` (handled locally by world/kernel and `raw/TUSK-CORE`)
- `dm.refine.cover` (handled locally by world/kernel and `raw/TUSK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.presentation.projection` (handled by projection layer)
