# Tusk Refinement

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Refinement in Tusk means changing execution granularity while preserving meaning under declared equivalence mode.

Refinement is used when coarse runs fail to discharge obligations or fail contractible glue checks.

## 2. Refinement relation

Write `R' <= R` when run `R'` is a refinement of run `R`.

A refinement step is a morphism between run identities that changes exactly one axis.

Required metadata per refinement step:

- `parent_run_id`
- `refinement_axis`

## 3. Identity prerequisites

Run identity should include:

- `world_id`
- `context_id`
- `intent_id`
- `ctx_ref`
- `data_head_ref`
- `cover_id`
- `adapter_id` + `adapter_version`
- `normalizer_id` + `policy_digest`

Refinement comparability depends on this identity discipline.

`cover_strategy_digest` is audit material by default and only enters comparability when explicit hardening policy includes it.

## 4. Refinement axes

### 4.1 Cover refinement

Change `cover_id` only.

Use for:

- locality failures,
- descent existence gaps,
- coarse overlap ambiguity.

`intent_id` changes are usually new-run boundaries, not cover refinements.

### 4.2 Context refinement

Change `ctx_ref` only.

Use for:

- stability failures under context change,
- missing context transport structure.

### 4.3 Policy refinement

Change `normalizer_id` and/or `policy_digest`.

Use for:

- explicit semantic mode upgrades,
- capability-activated checks,
- comparison hardening.
- overlap-level upgrades (for example `pairwise` to `higher_cech`).

This is usually a new-run boundary.

### 4.4 Adapter refinement

Change `adapter_version` (or adapter binding).

Use for:

- schema evolution,
- improved domain interpretation.

Non-trivial changes should include migration evidence.

### 4.5 Evidence enrichment

Keep identity axes fixed and strengthen compatibility evidence payloads.

Use for:

- persistent ambiguity under same cover and mode.

## 5. One-axis rule

Exactly one axis per step is load-bearing.

It ensures:

- diagnosable witness deltas,
- comparable refinement ladders,
- clearer refinement invariance reasoning.

## 6. Witnessing under refinement

Refinement outputs should separate:

- `GateWitness` (local admissibility),
- `TransportWitness` (if cross-world transport is involved).

See `TUSK-WITNESSING.md` for envelope split.

## 7. Failure mapping

Keep Gate mapping aligned with normative specs:

- `stability` -> `stability_failure`
- `locality` -> `locality_failure`
- `descent_exists` / `ext_gap` -> `descent_failure`
- `descent_contractible` / `ext_ambiguous` -> `glue_non_contractible`
- `adjoint_triple` -> `adjoint_triple_coherence_failure` (when advertised)

## 8. Deterministic refinement ladder

```text
run R0
if accepted: close

for step in ordered_refinement_plan:
  create Ri+1 by one-axis change
  re-run checks
  if accepted: close

if exhausted:
  reject with deterministic witness ordering
```

`ordered_refinement_plan` should be deterministic for fixed input state/policy.

## 9. Trigger guidance

Suggested trigger order:

1. cover refinement for locality/descent failures,
2. context refinement for stability failures,
3. evidence enrichment for persistent ambiguity,
4. policy refinement for explicit semantic mode changes.

## 10. Control-policy boundary

Control policy decides when/what to refine.

Control policy does not redefine admissibility or equivalence semantics.
