# Development Meta Loop

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Capture one canonical development workflow so we do not repeatedly re-derive
process shape while building.

This document is the operational meta contract for:

- issue sequencing,
- multithread worker discipline,
- lane/authority boundaries,
- and gate cadence.

## 2. First Principles

1. Minimum encoding, maximum expressiveness.
2. One authority artifact per boundary (no parallel semantics).
3. Architecture/spec glue before implementation.
4. Implementation before checker fixtures.
5. Checker fixtures before docs/traceability closure.
6. Context is treated as typed bounded state, not transcript carryover.

Authority references:

- `specs/premath/draft/SPEC-INDEX.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `docs/design/control-plane/MEMORY-LANES-CONTRACT.md`
- Tusk/downstream tracker docs for execution-loop policy

## 3. Canonical Work Order (per epic)

Default order for any non-trivial epic:

1. Architecture contract slice
2. Spec/index/doctrine-site glue slice
3. Control-plane typed contract + parity slice
4. Core implementation slice
5. Checker fixture slice
6. Downstream projection slice (if needed)
7. Docs/traceability closure slice

If an epic skips a layer, record why in issue notes and keep dependency edges
explicit.

## 4. Multithread Operating Model

### 4.1 Roles

- Coordinator: owns prioritization and dependency updates in the owning tracker.
- Worker: executes one bounded tracker item at a time.

### 4.2 Current write discipline

Tracker write discipline is owned outside Premath. Premath only checks
normalized work claims at the boundary.

Worker mutation authority remains instruction-linked by default.

### 4.3 Worker loop

1. Select target in the owning Tusk/downstream tracker.
2. Check tracker-provided normalized work claim with `premath work-tracker-check`
   when a Premath boundary witness is needed.
3. claim target
4. reconstruct bounded working context from typed state views/handoff refs
   before mutation-capable steps
5. execute bounded change
6. run required verification commands
7. if new work discovered: record it in the owning Tusk/downstream tracker
8. write concise notes + refs in the owning tracker, then close/release there

Never run multi-item implicit sessions.

Canonical Premath surface:

- `premath work-tracker-check`

Tracker scheduling and dependency diagnostics are outside Premath.

### 4.4 Dependency compactness discipline

- Chain-shaped epics should bind to terminal blockers only.
- Active `blocks` edges that point to `closed` issues are drift and should be removed.
- Active transitive-redundant `blocks` edges are drift and should be removed.

Operational hygiene is checked with `premath repo-hygiene-check`.

## 5. Lane Discipline

- Tracker lane: task state, dependencies, acceptance, and verification commands
  owned outside Premath.
- Operations lane (`.premath/OPERATIONS.md`): runbooks and rollout evidence.
- Doctrine/decision lane (`specs/*`, `decision-log.md`): contract authority and
  lifecycle decisions.

Do not move semantic authority into operations or issue notes.

## 6. Gate Cadence

Minimum gate cadence by change class:

- Docs/spec glue: `premath traceability-check` + `premath drift-budget-check`
- Control-plane/checker: `premath coherence-check` + `premath drift-budget-check`
- Checker/core: `cargo test --workspace`
- Checker fixtures: `cargo test --workspace`

Always finish with:

- `premath repo-hygiene-check`

## 7. Definition of Done (issue-level)

An issue is done when:

1. acceptance criteria are satisfied,
2. verification commands have been run successfully,
3. issue notes are concise and reference artifacts/commits/decisions,
4. dependency graph is updated for discovered follow-up work.

## 8. Anti-Patterns

Avoid:

- architecture changes without issue dependency updates,
- adding new operational surfaces without doctrine-site/spec-index mapping,
- parallel mutation semantics outside instruction-linked routes,
- long-lived sessions with unrecorded discovered work.

## 9. WIP Topology Inventory Protocol

Use this protocol whenever the worktree is materially dirty across multiple
lanes.

1. Enumerate dirty paths (`git status --porcelain`).
2. Group paths into WIP clusters by authority lane and surface family
   (crates/tools/specs/docs/fixtures/operations).
3. Assign each cluster to one active issue ID (primary owner, optional
   secondary).
4. Record the mapping in
   `.premath/OPERATIONS.md` under `Active WIP Topology Ownership Map`.
5. Ensure no dirty cluster is left unowned relative to active issue scope.
6. Keep dependency chain shape aligned with lane order in §3.

Consistency constraints:

- lane semantics must remain consistent with `docs/design/control-plane/ARCHITECTURE-MAP.md`
  §10,
- topology budget thresholds remain contract-driven in
  `specs/process/TOPOLOGY-BUDGET.json`,
- tracker updates remain authoritative in the owning Tusk/downstream tracker.
