# Tusk Descent Packs

Status: draft
Scope: design-level, non-normative

## 1. Purpose

This document defines a domain-generic shape for presheaf-like local/overlap/glue material.

Term choice: use `DescentPack` as the canonical term.

## 2. Hard boundary

Authority split:

- `DomainAdapter` proposes `DescentCore` structure/evidence and glue proposals.
- `PremathWorld` (or `KernelRuntime`) judges admissibility and emits Gate-class witnesses.

Adapters do not self-certify sheaf/stack behavior.

## 3. World-owned entities

World side owns:

- `context_id`
- `ctx_ref`
- `Cover`
- `CoverPartId`
- `OverlapId`
- admissibility checks

`OverlapId` must be world-defined so adapters cannot hide problematic overlaps.

## 4. Adapter-owned entities

Adapter side defines opaque domain payloads:

- `LocalState(part_id)`
- `CompatWitness(part_i, part_j, overlap_id)`
- `GlueProposal`

The world checks these, but does not prescribe domain internals.

## 5. Core and pack shapes

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

`DescentCore` is the admissibility-check input.
`DescentPack` is an assembled artifact that includes proposal outputs for traceability.

`glue_proposals` may be empty; this is semantically different from non-contractible multiple proposals.

Recommended phase order:

1. collect locals/compat -> `DescentCore`
2. world checks admissibility on `DescentCore`
3. adapter proposes `GlueProposalSet` from `DescentCore`
4. world selects `GlueResult` from `GlueProposalSet`
5. assemble `DescentPack` as trace artifact

## 6. Operational checks

World-side checks:

- locality: restrictions exist for required cover parts,
- descent existence: at least one valid glue proposal can be produced,
- contractibility: glue space is unique up to declared mode,
- refinement invariance: result stable under allowed refinements.

World-owned glue result:

- adapter proposes `GlueProposalSet`,
- world selects `GlueResult` under declared mode,
- if no valid proposal exists: `descent_failure`,
- if multiple inequivalent selections remain: `glue_non_contractible`.

## 7. GlueResult and selection failure shapes

```text
GlueResult {
  selected: GlueProposalId
  normal_form_ref?: NormalFormRef
  contractibility_basis: {
    mode: (normalizer_id, policy_digest)
    proof_refs: list<WitnessRef>
  }
}
```

```text
GlueSelectionFailure =
  | no_valid_proposal
  | non_contractible_selection
  | mode_comparison_unavailable
```

Suggested Gate-class mapping:

- `no_valid_proposal` -> `descent_failure`
- `non_contractible_selection` -> `glue_non_contractible`
- `mode_comparison_unavailable` -> `descent_failure` with `phase=normalize` and `responsible_component=world`

## 8. Failure mapping

Suggested mapping:

- missing required locals/restrictions -> `locality_failure`
- no glue proposal or no valid glue selection -> `descent_failure`
- multiple inequivalent glue results -> `glue_non_contractible`
- reindex/coherence mismatch in context transport -> `stability_failure`

## 9. Domain examples (shape only)

Task-graph adapter:

- locals: partitioned subgraphs + boundary stubs,
- overlaps: shared boundary task/dependency obligations,
- glue proposal: global graph assembly.

Accounting adapter:

- locals: partitioned posting/journal views,
- overlaps: shared transaction/balance obligations,
- glue proposal: global journal/ledger assembly.

Same `DescentPack` shape, different domain payload semantics.

## 10. Refinement behavior

Refinement acts on pack structure by one axis per step:

- cover refinement -> new world cover + recomputed locals/compat,
- context refinement -> new `ctx_ref` + recomputed pack,
- policy refinement -> new mode binding (`normalizer_id`, `policy_digest`),
- adapter refinement -> new adapter version (with migration evidence if needed).

Each refined pack should be linked to its parent run identity.

## 11. Overlap policy (v0)

This section fixes the minimum overlap-checking contract for v0 implementations.

### 11.1 Required and optional levels

- required level: `pairwise`
- optional level: `higher_cech`

`pairwise` means obligations over overlap IDs induced by `(i, j)` cover-part intersections.

`higher_cech` means additional obligations on higher intersections `(i, j, k, ...)` and cocycle coherence.

### 11.2 Capability and activation

- worlds must always support `pairwise`.
- worlds may advertise `higher_cech` capability.
- if policy requests `higher_cech`, run mode must bind that request in semantic policy material (`policy_digest`).

### 11.3 Deterministic overlap ordering

Overlap obligations must be deterministically ordered for checks and witness emission.

Recommended ordering key:

1. overlap arity (`2` for pairwise, then `3`, etc.)
2. lexicographic tuple of normalized part IDs
3. overlap ID as tie-breaker

### 11.4 Failure mapping

For v0, use the following mapping:

- required overlap level unavailable (`higher_cech` requested but unsupported) -> `descent_failure`
- required overlap obligation missing from world materialization -> `locality_failure`
- required compatibility witness missing for materialized overlap -> `locality_failure`
- compatibility witness present but fails coherence check -> `descent_failure`
- pairwise checks pass but higher-level cocycle fails (when enabled) -> `descent_failure`
- multiple inequivalent glue outcomes after all required overlap checks -> `glue_non_contractible`

### 11.5 Diagnostic attachment

Implementations should attach machine-readable diagnostics with overlap level and overlap ID context.

Examples:

- `overlap_level_requested`
- `overlap_level_supported`
- `overlap_id`
- `overlap_arity`
- `phase` (`restrict` | `compat` | `propose_glue` | `select_glue` | `normalize`)
- `responsible_component` (`world` | `adapter` | `context_provider` | `event_store`)
