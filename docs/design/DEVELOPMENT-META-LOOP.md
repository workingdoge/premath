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
4. Implementation before conformance vectors.
5. Conformance before docs/traceability closure.
6. Context is treated as typed bounded state, not transcript carryover.

Authority references:

- `specs/premath/draft/SPEC-INDEX.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `docs/design/MEMORY-LANES-CONTRACT.md`
- `docs/design/RALPH-PLAYBOOK-PREMATH.md` (external loop adaptation)
- `docs/design/STEEL-REPL-DESCENT-CONTROL.md` (REPL/descent control surface)

## 3. Canonical Work Order (per epic)

Default order for any non-trivial epic:

1. Architecture contract slice
2. Spec/index/doctrine-site glue slice
3. Control-plane typed contract + parity slice
4. Core implementation slice
5. Conformance vector slice
6. Observation/UX projection slice (if needed)
7. Docs/traceability closure slice

If an epic skips a layer, record why in issue notes and keep dependency edges
explicit.

## 4. Multithread Operating Model

### 4.1 Roles

- Coordinator: owns prioritization and dependency updates in issue memory.
- Worker: executes one bounded issue at a time.

### 4.2 Current write discipline

Until lock-safe distributed claim primitives are fully shipped, prefer one
coordinator as the effective writer for issue-graph sequencing updates.

Worker mutation authority remains instruction-linked by default.

### 4.3 Worker loop (single-issue discipline)

1. `dep_diagnostics(graph_scope=active)` preflight (fail closed on cycles)
2. `issue_ready` (select target via dependency/priority order)
3. claim/lease target
4. reconstruct bounded working context from typed state views/handoff refs
   before mutation-capable steps
5. execute bounded change
6. run required verification commands
7. if new work discovered: `issue_discover` + dependency edge
8. write concise notes + refs, then close/release

Never run multi-issue implicit sessions.

Canonical script surface:

- `python3 tools/harness/multithread_loop.py worker`
- `python3 tools/harness/multithread_loop.py coordinator`

Diagnostic convention:

- use `active` scope to gate scheduling (`ready` integrity),
- use `full` scope for historical/forensic cycle review.

### 4.4 Dependency compactness discipline

- Chain-shaped epics should bind to terminal blockers only.
- Active `blocks` edges that point to `closed` issues are drift and should be removed.
- Active transitive-redundant `blocks` edges are drift and should be removed.

Operational surfaces:

- `python3 tools/ci/check_issue_graph.py` (gate-level compactness enforcement)
- `python3 tools/ci/compact_issue_graph.py --mode check|apply` (deterministic remediation)

## 5. Lane Discipline

- Issue lane (`.premath/issues.jsonl`): task state, dependencies, acceptance,
  verification commands.
- Operations lane (`.premath/OPERATIONS.md`): runbooks and rollout evidence.
- Doctrine/decision lane (`specs/*`, `decision-log.md`): contract authority and
  lifecycle decisions.

Do not move semantic authority into operations or issue notes.

## 6. Gate Cadence

Minimum gate cadence by change class:

- Docs/spec glue: `mise run docs-coherence-check` + `mise run traceability-check`
- Control-plane/checker: `mise run coherence-check` + `mise run ci-pipeline-test`
- Mutation/concurrency/core: `cargo test -p premath-bd` + `cargo test -p premath-cli`
- Capability/conformance: `mise run conformance-run`

Always finish with:

- `python3 tools/ci/check_issue_graph.py`

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

- lane semantics must remain consistent with `docs/design/ARCHITECTURE-MAP.md`
  §10,
- topology budget thresholds remain contract-driven in
  `specs/process/TOPOLOGY-BUDGET.json`,
- issue-graph updates remain authoritative in `.premath/issues.jsonl`.
