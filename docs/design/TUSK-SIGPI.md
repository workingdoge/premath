# Tusk SigPi

Status: draft
Scope: design-level, non-normative

## 1. Purpose

`tusk-core` covers execution inside one Premath world.
`tusk-sigpi` covers transport/composition between Premath worlds.
Runtime placement site contracts are specified in `raw/SQUEAK-SITE`.

## 2. World descriptor

A world descriptor should include at least:

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

## 3. Inter-world transport contract

Logical interface:

```text
map_context(src_world, dst_world, src_context_id, src_ctx_ref) -> (dst_context_id, dst_ctx_ref)
transport_summary(src_world, dst_world, summary) -> dst_summary
transport_obligation(src_world, dst_world, obligation) -> dst_obligation
transport_witness(src_world, dst_world, witness_ref) -> dst_witness_ref
```

Optional adjoint-flavored operations may exist where implemented.

## 4. Composition requirements

Transport should satisfy:

- identity coherence,
- composition coherence,
- witness lineage preservation,
- explicit capability compatibility.

## 5. Negotiation requirements

Before transport, worlds should negotiate:

- capability compatibility,
- comparison mode compatibility,
- `normalizer_id` + `policy_digest` compatibility,
- witness schema compatibility.

Mismatch should reject deterministically.

## 6. Witness class boundary

SigPi failures are transport failures, not Gate failures.

Recommended transport classes:

- `world_morphism_missing`
- `world_capability_mismatch`
- `world_policy_mismatch`
- `transport_context_unresolved`
- `transport_witness_unverifiable`
- `transport_non_composable`

## 7. Non-bypass rule

SigPi transport never creates local admissibility.

Transported artifacts must still pass destination-world admissibility checks under destination bindings.

TransportWitness can certify only transport compatibility.
It may reference GateWitness bundles, but it never upgrades their validity across worlds.

## 8. Relationship to Tusk core lifecycle

`tusk-core` emits local summaries, obligations, and Gate witnesses.
`tusk-sigpi` maps those artifacts across worlds and returns destination-scoped artifacts for local checking.

No separate semantic bridge layer is required: destination handoff is part of
`tusk-sigpi` responsibility.

## 9. v0 boundary

Reasonable v0:

- one-way summary/obligation transport,
- strict binding compatibility checks,
- transport witness emission with lineage,
- no implicit local admissibility carry-over.

## 10. Runtime unit naming

For runtime substrate orchestration in SigPi/Squeak, this repository uses:

- `Cheese` (or `SqueakCheese`) as the runtime unit term.

`Cheese` covers operational execution placement (local process, remote worker,
microvm profile, etc.) and remains outside semantic admissibility authority.
