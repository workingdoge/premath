---
slug: raw
shortname: TUSK-CORE
title: workingdoge.com/premath/TUSK-CORE
name: Tusk Core Runtime Contracts
status: raw
category: Standards Track
tags:
  - premath
  - tusk
  - runtime
  - descent
  - memory
  - control
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

This specification defines `tusk-core`: execution contracts for one Premath
world.

`tusk-core` operationalizes kernel law checks from:

- `draft/PREMATH-KERNEL`
- `draft/GATE`
- `draft/BIDIR-DESCENT`

This document does not replace kernel semantics and does not define inter-world
transport (`raw/SQUEAK-CORE`).

## 2. Boundary and authority split

Tusk implementations MUST preserve this split:

- `DomainAdapter` proposes projections, locals, compatibility evidence, and glue
  proposals.
- `PremathWorld` (or equivalent kernel runtime) chooses covers, materializes
  overlaps, decides admissibility, and emits Gate-class outcomes.

Adapters MUST NOT self-certify admissibility.

## 3. Context and reference model

Implementations MUST use explicit reference fields:

- `context_id`: stable key for context object `Gamma` in world category `C`.
- `ctx_ref`: lineage pointer used to materialize that context.
- `data_head_ref`: append-only EventStore head.

`head_ref` without a plane qualifier SHOULD be avoided.

Run base is:

```text
RunBase = (world_id, context_id, ctx_ref, data_head_ref)
```

`context_id` and `ctx_ref` MUST be treated as related but non-interchangeable.

## 4. Required interfaces

### 4.1 ContextProvider

```text
resolve_context_id(scope) -> context_id
resolve_ctx_ref(context_id, scope) -> ctx_ref
parents(ctx_ref) -> list<ctx_ref>
snapshot(ctx_ref) -> ContextSnapshot
diff(ctx_ref_a, ctx_ref_b) -> ContextDelta
```

### 4.2 EventStore

```text
append(events, at_data_head_ref) -> data_head_ref
read(range_or_filter, at_data_head_ref) -> event_stream
fold(event_stream, reducer_id) -> state_snapshot
checkpoint(data_head_ref) -> snapshot_ref
```

EventStore contract:

- `append` MUST be linearizable and induce a deterministic total event order.
- `at_data_head_ref` MUST be interpreted as a CAS-like precondition.
- CAS mismatch MUST be diagnostic/control-plane output, not a Gate failure.
- deterministic replay MUST be defined over `(data_head_ref, reducer_id)`.
- event identity MUST include an idempotency key to make retries safe.

### 4.3 DomainAdapter

```text
adapter_id() -> string
adapter_version() -> string
project(context_id, ctx_ref, data_head_ref, event_stream) -> DomainProjection
cover_strategy(projection, intent_spec) -> CoverStrategy
restrict(projection, cover_part_id) -> LocalState
compatibility(local_i, local_j, overlap_id) -> CompatWitness
propose_glue(descent_core) -> GlueProposalSet
encode_intent(domain_command) -> EventBatch
summarize(glue_result) -> Summary
obligations(glue_result) -> ObligationSet
```

Adapter rules:

- outputs MUST be deterministic for fixed inputs and policy bindings.
- adapters MUST NOT define world coverage doctrine.
- adapters MUST NOT select final glue semantics.
- `summarize` and `obligations` MUST consume world-selected `GlueResult`, not
  raw proposals.

### 4.4 PremathWorld / KernelRuntime

```text
choose_cover(context_id, cover_strategy) -> Cover
materialize_overlaps(cover, overlap_level) -> OverlapSet
check_descent_core(core, overlap_level) -> GateWitnessSet
select_glue(glue_proposals, mode) -> GlueResult | GlueSelectionFailure
```

World rules:

- `Cover`, `CoverPartId`, and `OverlapId` are world-owned.
- overlap obligations MUST be world-materialized, not adapter-supplied.
- glue selection under mode MUST be world-owned.

## 5. Descent artifacts

### 5.1 Core and pack

```text
DescentCore {
  cover: Cover
  locals: map<CoverPartId, LocalState>
  compat: list<CompatWitness>
  mode: {
    normalizer_id
    policy_digest
  }
}
```

```text
DescentPack = DescentCore + {
  glue_proposals: GlueProposalSet
}
```

### 5.2 Glue selection result

```text
GlueResult {
  selected: GlueProposalId
  contractibility_basis: {
    mode: (normalizer_id, policy_digest)
    method: "normal_form" | "equiv_witness" | "external_checker"
    evidence_refs: list<WitnessRef>
  }
  normal_form_ref?: Ref
}
```

`normal_form_ref` MAY be present only when `method="normal_form"`.

```text
GlueSelectionFailure =
  | no_valid_proposal
  | non_contractible_selection
  | mode_comparison_unavailable
```

Required mapping:

- `no_valid_proposal` -> `descent_failure`
- `non_contractible_selection` -> `glue_non_contractible`
- `mode_comparison_unavailable` -> `descent_failure` with normalize-phase
  diagnostics

## 6. Unit lifecycle

A conforming unit lifecycle MUST preserve this phase ordering:

```text
open(scope, intent_spec, policy) -> RunBase + identity fields
project(...) -> DomainProjection
choose_cover(...) -> Cover
spawn(Cover) -> ChildUnits
collect(ChildUnits) -> DescentCore
check_descent(DescentCore, overlap_level) -> GateWitnessSet
propose_glue(DescentCore) -> GlueProposalSet
assemble_descent_pack(DescentCore, GlueProposalSet) -> DescentPack
select_glue(GlueProposalSet, mode) -> GlueResult | GlueSelectionFailure
close(GlueResult, GateWitnessSet) -> (Summary, ObligationSet, WitnessBundle)
```

## 7. Identity and determinism

Run identity MUST include at least:

- `world_id`
- `unit_id`
- `parent_unit_id` (optional)
- `context_id`
- `intent_id`
- `cover_id`
- `ctx_ref`
- `data_head_ref`
- `adapter_id`
- `adapter_version`
- `normalizer_id`
- `policy_digest`

`intent_id` MUST be derived from canonical `IntentSpec`, not raw natural
language.

`cover_strategy_digest` SHOULD be diagnostic material by default and MAY become
identity material only under explicit hardening policy.

## 8. Failure classes and diagnostics

`check_descent_core` and `select_glue` outcomes MUST map to Gate classes:

- stability mismatch -> `stability_failure`
- missing required restrictions/overlaps -> `locality_failure`
- no valid glue path -> `descent_failure`
- non-contractible glue space -> `glue_non_contractible`

Implementations SHOULD attach machine-readable diagnostics:

- `phase`: `restrict | compat | propose_glue | select_glue | normalize`
- `responsible_component`: `world | adapter | context_provider | event_store`
- `overlap_level`, `overlap_id`, `overlap_arity` where relevant

## 9. Evidence profile layering

`tusk-core` defines meaning and interfaces, not one witness representation.

A conforming implementation MAY expose profile capabilities, including:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`

Kernel admissibility outcomes MUST remain invariant for fixed semantic inputs
and fixed policy bindings, regardless of chosen evidence profile.

## 10. Conformance mapping (`tusk-core` fixture suite)

Deterministic runtime-contract vectors live under:

- `tests/conformance/fixtures/tusk-core/`

and are executable through:

- `python3 tools/conformance/run_tusk_core_vectors.py`
- `mise run conformance-run` (cached suite id: `tusk-core`)

Boundary coverage mapping:

- accepted single-glue path (§5.2, §6):
  `golden/tusk_eval_single_glue_accept`.
- missing local restrictions (§5.1, §8):
  `adversarial/tusk_eval_no_locals_locality_failure_reject`
  -> `locality_failure` (`GATE-3.2`).
- missing overlap compatibility for multi-local packs (§4.3, §8):
  `adversarial/tusk_eval_multi_local_missing_compat_locality_failure_reject`
  -> `locality_failure` (`GATE-3.2`).
- no valid glue proposal (§5.2 required mapping):
  `adversarial/tusk_eval_no_glue_descent_failure_reject`
  -> `descent_failure` (`GATE-3.3`).
- missing mode comparison binding (§5.2 required mapping):
  `adversarial/tusk_eval_mode_missing_descent_failure_reject`
  -> `descent_failure` (`GATE-3.3`).
- non-contractible glue selection (§5.2 required mapping):
  `adversarial/tusk_eval_multi_glue_non_contractible_reject`
  -> `glue_non_contractible` (`GATE-3.4`).

## 11. Security and robustness

Implementations MUST treat context stores, event streams, and adapter payloads as
untrusted input.

Implementations SHOULD:

- bound recursion and overlap expansion,
- fail closed on malformed mode bindings,
- emit deterministic error diagnostics suitable for CI gating.

## 12. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.profile.execution` (for fixed semantic inputs + fixed policy bindings)
- `dm.profile.evidence` (for fixed semantic inputs + fixed policy bindings)
- `dm.policy.rebind` (new run boundary required)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.presentation.projection` (handled by projection layer)
