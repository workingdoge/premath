---
slug: raw
shortname: WORLD-PROFILES-CONTROL
title: workingdoge.com/premath/WORLD-PROFILES-CONTROL
name: Control-Plane World Profiles (Lease, Instruction, CI Witness)
status: raw
category: Informational
tags:
  - premath
  - world
  - lease
  - instruction
  - ci
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

This raw profile document defines operational world-profile candidates for:

- `world.lease.v1`
- `world.fiber.v1`
- `world.instruction.v1`
- `world.ci_witness.v1`

This document does not introduce a new semantic authority chain. Kernel/Gate
authority remains in `draft/PREMATH-KERNEL`, `draft/BIDIR-DESCENT`, and
`draft/GATE`.

## 2. Shared framing

Each world profile is described by:

```text
W = (C_W, Def_W, Cov_W, ~=_W, Reindex_W)
```

where:

- `C_W` is a context category,
- `Def_W : C_W^op -> V` is a definables family,
- `Cov_W` is a cover system for concurrency decomposition,
- `~= _W` is profile sameness,
- `Reindex_W` is context morphism transport.

Control-plane routing may project into these worlds, but projection MUST NOT
replace semantic admissibility authority.

### 2.1 K0 migration boundary (core vs adapter)

This raw profile set assumes one kernel-backed world semantics lane:

- world row/morphism/binding validation lives in core Rust semantics paths,
- checker/CLI/CI consumers call those core semantics,
- Python/shell wrappers remain transport and orchestration adapters.

Operational ownership target:

- semantic validation owner: `crates/premath-kernel` (+ shared consumers),
- checker/CLI consumer lanes: `crates/premath-coherence`, `crates/premath-cli`,
- adapter lanes: `tools/conformance/*`, `tools/ci/*` (no independent authority).

Fail-closed migration rule:

- if wrapper-local world logic disagrees with core world logic, wrapper output
  is invalid and MUST be treated as drift.

## 3. Lease world (`world.lease.v1`)

### 3.1 Context family (`C_lease`)

Candidate context kinds:

- `lease.issue_scope` (set of issue IDs in one authority store snapshot),
- `lease.worker_partition` (worker-selected subcover of issue scope),
- `lease.snapshot` (time/snapshot keyed lease state view).

Candidate morphism kinds:

- `lease.identity`,
- `lease.refine.partition` (worker refinement),
- `lease.rebase.snapshot` (authority snapshot transition),
- `lease.policy_rollover` (policy digest/context roll).

### 3.2 Definables (`Def_lease`)

Definable objects are lease claims over issues:

- `Claim(issue_id, owner, lease_id, expires_at, status)`.

Local restriction projects claim state to a sub-scope (subset of issue IDs).

### 3.3 Covers and descent

Admissible covers are worker/worktree partitions of one issue scope.

Descent obligations:

- pairwise overlaps MUST agree on owner/lease identity for shared issue IDs,
- triple overlaps MUST satisfy cocycle agreement,
- glue MUST be contractible per issue ID (exactly one global lease claim class).

Failure-class projection (candidate):

- overlap restriction missing -> `locality_failure`,
- no admissible global lease glue -> `descent_failure`,
- multiple active-owner glues -> `glue_non_contractible`.

### 3.4 Existing route-family linkage

Mutation route families intended to bind this world:

- `issue.claim`
- `issue.lease_renew`
- `issue.lease_release`
- `issue.discover` (when lease-state mutation is implied)

## 3.5 Fiber world (`world.fiber.v1`)

### 3.5.1 Context family (`C_fiber`)

Candidate context kinds:

- `fiber.session_scope` (one parent concurrency scope),
- `fiber.child_partition` (child task cover/refinement),
- `fiber.snapshot` (typed lifecycle snapshot for join/cancel boundaries).

Candidate morphism kinds:

- `fiber.identity`,
- `fiber.refine.child`,
- `fiber.join.boundary`,
- `fiber.cancel.boundary`.

### 3.5.2 Definables (`Def_fiber`)

Definable objects are structured-concurrency lifecycle rows:

- `Fiber(fiber_id, task_ref, parent_fiber_id?, scope_ref?, state)`.

### 3.5.3 Covers and descent

Admissible covers are child-fiber decompositions for one parent execution
objective.

Descent obligations:

- overlap compatibility MUST preserve deterministic child identity for shared
  join domains,
- join glue MUST be deterministic (`joined` or explicit operational
  obstruction),
- cancellations MUST preserve lineage references and fail closed when target
  identity is unresolved.

### 3.5.4 Existing route-family linkage

Route families intended to bind this world:

- `fiber.spawn`
- `fiber.join`
- `fiber.cancel`

## 4. Instruction world (`world.instruction.v1`)

### 4.1 Context family (`C_instruction`)

Candidate context kinds:

- `instruction.envelope`,
- `instruction.policy_snapshot`,
- `instruction.repo_head`.

Candidate morphism kinds:

- `instruction.identity`,
- `instruction.rebase`,
- `instruction.policy_rebind`.

### 4.2 Definables (`Def_instruction`)

Definable objects are typed instruction admissibility states:

- envelope identity,
- policy/normalizer binding,
- requested-check set with route admissibility.

### 4.3 Covers and descent

Covers may split envelope checking into route-specific obligations (for example,
instruction shape, policy binding, route allowlist).

Descent obligations:

- overlap compatibility across route splits MUST agree on instruction identity
  and policy binding,
- global instruction admissibility glue MUST be unique.

### 4.4 Existing route-family linkage

Route families intended to bind this world:

- `op/ci.run_instruction`
- `op/mcp.instruction_run`
- instruction check/witness verification surfaces.

## 5. CI witness world (`world.ci_witness.v1`)

### 5.1 Context family (`C_ciw`)

Candidate context kinds:

- `witness.projection`,
- `required.delta`,
- `required.check_set`,
- `decision.attestation`.

Candidate morphism kinds:

- `ciw.identity`,
- `ciw.rebase`,
- `ciw.projection_refine`.

### 5.2 Definables (`Def_ciw`)

Definables are required/decision witness chain states:

- required witness payload,
- verification derivations,
- decision attestations and linkage.

### 5.3 Covers and descent

Covers may split witness validation into:

- projection path,
- linked gate witness lineage,
- decision-attestation chain.

Global admissibility requires unique coherent glue over the split checks.

### 5.4 Existing route-family linkage

Route families intended to bind this world:

- `op/ci.verify_required_witness`
- `op/ci.decide_required`

## 6. Optional torsor/extension overlay (`overlay.torsor_ext.v1`)

This overlay is optional and profile-gated. It is an interpretation layer for
twist/extension classes across the control worlds above; it is not an
admissibility world.

Candidate interpretation points:

- lease twist class: alternate but equivalent lease cover glue representatives,
- instruction twist class: alternate envelope decomposition representatives,
- CI witness twist class: alternate attestation-chain representatives.

Candidate row shape:

```text
TorsorOverlayRow {
  overlayId: "overlay.torsor_ext.v1"
  baseWorldId: string      # one of world.lease.v1 | world.fiber.v1 | world.instruction.v1 | world.ci_witness.v1
  baseRef: string          # canonical witness/projection ref in existing authority lanes
  extClassRef: string      # extension/twist class descriptor
  transportClass: string   # transport-naturality class
}
```

Non-authority constraints:

- torsor/extension rows MUST be proposal/evidence-only attachments,
- torsor/extension rows MUST NOT be bound as `worldId` targets for mutation or
  attestation route families,
- final `accept|reject` outcomes MUST remain derived from existing world route
  checks and checker/Gate witnesses.

Misuse rejection posture (candidate classes):

- torsor row used as direct admissibility authority ->
  `torsor_overlay_authority_violation`,
- torsor row unbound from base witness/reference ->
  `torsor_overlay_unbound`,
- torsor transport mismatch under reindexing ->
  `torsor_overlay_transport_drift`.

Compatibility statement:

- This overlay follows the informational geometry in `raw/TORSOR-EXT` and is
  invalid unless transport-naturality + proposal/evidence-only posture from
  `raw/TORSOR-EXT` §6 are preserved.

## 7. Control-plane morphism table sketch

For `controlPlaneBundleProfile` (`C_cp`, `E_cp`) the intended morphism rows are:

- `wm.kernel.semantic.runtime_gate`:
  `world.kernel.semantic.v1 -> world.control_plane.bundle.v0`
  over `{dm.identity, dm.profile.execution, dm.transport.world, dm.transport.location}`.
- `wm.control.lease.mutation`:
  `world.control_plane.bundle.v0 -> world.lease.v1`
  over `{dm.identity, dm.profile.execution, dm.commitment.attest}`.
- `wm.control.fiber.lifecycle`:
  `world.control_plane.bundle.v0 -> world.fiber.v1`
  over `{dm.identity, dm.profile.execution, dm.transport.world}`.
- `wm.control.instruction.execution`:
  `world.control_plane.bundle.v0 -> world.instruction.v1`
  over `{dm.identity, dm.profile.execution, dm.commitment.attest}`.
- `wm.control.ci_witness.attest`:
  `world.control_plane.bundle.v0 -> world.ci_witness.v1`
  over `{dm.identity, dm.presentation.projection, dm.commitment.attest}`.
- `wm.control.bundle.projection`:
  intra-control projections over `{dm.identity, dm.presentation.projection}`.

## 8. Rejection posture

Implementations should reject fail closed when:

- route-family world binding is missing,
- route morphisms drift from declared row morphisms,
- world profile claim is asserted but required route family is unbound.

## 9. Authority boundary

This profile document is informational/raw.

When these world profiles are implemented, they MUST project through one
authority chain and MUST NOT introduce a second admissibility surface.
