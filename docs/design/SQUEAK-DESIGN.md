# Squeak Design

Status: draft
Scope: design-level, non-normative

## 1. Purpose

This document is the canonical design companion for runtime placement and
inter-world transport concerns in this repository.

Boundary:

- Tusk covers execution inside one Premath world.
- Squeak covers runtime placement and transport/composition between worlds.
- Kernel/Gate remain semantic admissibility authority.

Normative surfaces remain under `specs/`:

- `specs/premath/raw/SQUEAK-CORE.md`
- `specs/premath/raw/SQUEAK-SITE.md`
- `specs/premath/raw/CI-TOPOS.md` (execution substrate mapping at CI layer)

## 2. Relationship to SigPi

In prose, use `SigPi` for the adjoint/transport framing and `squeak` for
runtime placement/orchestration surfaces.

Operationally:

- SigPi/Squeak transport never creates local admissibility.
- Transported artifacts must pass destination-world admissibility checks under
  destination bindings.
- Transport witnesses can certify compatibility/lineage only.

## 3. World descriptor (minimum)

```text
WorldDescriptor {
  world_id
  kernel_profile
  capability_vector
  adapter_set
  normalizer_id
  policy_digest
  witness_schema
}
```

## 4. Transport contract (design surface)

```text
map_context(src_world, dst_world, src_context_id, src_ctx_ref) -> (dst_context_id, dst_ctx_ref)
transport_summary(src_world, dst_world, summary) -> dst_summary
transport_obligation(src_world, dst_world, obligation) -> dst_obligation
transport_witness(src_world, dst_world, witness_ref) -> dst_witness_ref
```

Recommended transport failure classes:

- `world_morphism_missing`
- `world_capability_mismatch`
- `world_policy_mismatch`
- `transport_context_unresolved`
- `transport_witness_unverifiable`
- `transport_non_composable`

## 5. Runtime unit naming

For runtime substrate orchestration in Squeak/SigPi paths, this repository
uses:

- `Cheese` (or `SqueakCheese`) as the runtime unit term.

`Cheese` covers operational execution placement (local process, remote worker,
microvm profile, etc.) and remains outside semantic admissibility authority.

## 6. Design coherence guard

Keep design docs coherent by routing all runtime transport/placement guidance
through this file and linking it from:

- `docs/design/README.md`
- `docs/design/ARCHITECTURE-MAP.md`

Use `docs/design/TUSK-SIGPI.md` as a compatibility alias only.
