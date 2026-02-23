---
slug: draft
shortname: HARNESS-RUNTIME
title: workingdoge.com/premath/HARNESS-RUNTIME
name: Harness Runtime Contract
status: draft
category: Standards Track
tags:
  - premath
  - harness
  - runtime
  - session
  - trajectory
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

## 1. Purpose and authority boundary

This specification defines one runtime contract for long-running harness
operation.

The harness is operational control only. It MUST NOT introduce semantic
authority. Authoritative admissibility remains in
`draft/BIDIR-DESCENT.md` and `draft/GATE.md`; instruction authority boundaries
remain in `draft/LLM-INSTRUCTION-DOCTRINE.md`.

## 2. Canonical artifacts and command anchors

A conforming harness runtime MUST use the following canonical projection
artifacts:

- `.premath/harness_session.json` (`premath.harness.session.v1`)
- `.premath/harness_feature_ledger.json` (`premath.harness.feature_ledger.v1`)
- `.premath/harness_trajectory.jsonl` (`premath.harness.step.v1`)

A conforming command surface MUST include:

- `premath harness-session read|write|bootstrap`
- `premath harness-feature read|write|check|next`
- `premath harness-trajectory append|query`
- `python3 tools/harness/multithread_loop.py worker|coordinator`
- `mise run harness-worker-loop`
- `mise run harness-coordinator-loop`

## 3. Hook contract (`boot` / `step` / `stop`)

### 3.1 `boot`

`boot` MUST:

- resolve canonical memory roots:
  `.premath/issues.jsonl`, `.premath/harness_session.json`,
  `artifacts/ciwitness/*`, `artifacts/observation/latest.json`;
- run dependency integrity preflight with `dep_diagnostics(graph_scope=active)`;
- load prior handoff summary when present;
- compute next actionable target from issue graph plus policy bindings;
- run startup verification for the current scope before mutation.

### 3.2 `step`

`step` MUST:

- claim one bounded work item (single-task by default),
- execute mutation through instruction-mediated surfaces,
- run deterministic verification before and after mutation,
- emit typed witness references for side effects.

### 3.3 `stop`

`stop` MUST:

- persist compact continuation handoff state,
- release or renew lease deterministically through canonical issue memory,
- append trajectory row(s) linking issue/mutation identity, verification
  outcome, witness references, and next-step recommendation.

## 4. Session artifact contract

`premath.harness.session.v1` MUST contain:

- `schema: 1`
- `sessionKind: "premath.harness.session.v1"`
- `sessionId`
- `state` (`active|stopped`)
- `startedAt` and `updatedAt` (RFC3339)

Optional fields MAY include `issueId`, `summary`, `nextStep`,
`instructionRefs[]`, `witnessRefs[]`, `lineageRefs[]`, `stoppedAt`, `issuesPath`,
`issuesSnapshotRef`.

Determinism requirements:

- update-in-place preserves `sessionId` unless explicitly overridden,
- update-in-place preserves `startedAt` and refreshes `updatedAt`,
- optional ref arrays are trimmed, sorted, and deduplicated,
- empty optional string values are normalized to absent values.

`bootstrap` output kind MUST be `premath.harness.bootstrap.v1` with
deterministic `mode`:

- `resume` when session state is `stopped`,
- `attach` when session state is `active`.

## 5. Feature ledger contract

`premath.harness.feature_ledger.v1` MUST support deterministic feature
projection:

- one row per feature (`featureId` unique),
- status domain:
  `pending|in_progress|blocked|completed`,
- at most one `in_progress` feature,
- `completed` rows require at least one verification reference.

`harness-feature next` MUST select:

1. lexicographically smallest `in_progress`,
2. else lexicographically smallest `pending`,
3. else `null`.

`harness-session bootstrap` SHOULD project
`nextFeatureId`, `featureClosureComplete`, and `featureCount` from this ledger.

## 6. Trajectory contract

Trajectory rows (`premath.harness.step.v1`) MUST be append-only and include:

- `schema: 1`
- `stepKind: "premath.harness.step.v1"`
- `stepId`
- `action`
- `resultClass`
- `finishedAt` (RFC3339)

Rows MAY include `issueId`, `instructionRefs[]`, `witnessRefs[]`,
`lineageRefs[]`, `startedAt`.

When provided, `lineageRefs[]` SHOULD encode deterministic site lineage for the
worker step (`ctx://...`, `cover://...`, `refinement://...`) so stop/handoff
session and trajectory projections can be joined under one operational
cover/refinement view.

Stop/handoff rows SHOULD include one deterministic lease witness reference:

- `lease://handoff/<issue-id>/<lease-state>/<digest>`

Deterministic projections (`premath.harness.trajectory.projection.v1`) MUST
support modes `latest|failed|retry-needed` sorted by:

1. descending `finishedAt`,
2. `stepId`,
3. `action`.

## 7. Multithread coordinator/worker contract

The canonical worker loop shape is:

- `issue_ready -> claim -> work -> verify -> release/update`

Coordinator requirements:

- dispatch worktrees in sorted path order,
- run `dep_diagnostics(graph_scope=active)` before each scheduling pass,
- re-evaluate `issue_ready` each round.

Worker requirements:

- claim with deterministic lease semantics,
- project `session` + `feature` state,
- encode deterministic site lineage refs in session/trajectory projections,
- execute work and verify commands,
- derive handoff action from canonical issue-memory lease state,
- append trajectory with deterministic lease witness reference,
- write stop-state handoff.

Projection artifacts MUST NOT be treated as mutation authority.

## 8. Doctrine-site routing note

This spec reuses existing operation routing in
`draft/DOCTRINE-OP-REGISTRY.json` and `draft/DOCTRINE-SITE.json`.
It MUST NOT introduce parallel operation IDs.

Runtime routes include:

- `op/harness.session_read`
- `op/harness.session_write`
- `op/harness.session_bootstrap`
- `op/mcp.issue_claim`
- `op/mcp.issue_lease_renew`
- `op/mcp.issue_lease_release`
- `op/mcp.issue_lease_projection`
- `op/mcp.dep_diagnostics`

## 9. Verification surfaces

Minimum deterministic verification includes:

- `mise run ci-hygiene-check`
- `python3 tools/ci/check_issue_graph.py`
- `mise run docs-coherence-check`
- `mise run doctrine-check`

## 10. Related surfaces

- design runbooks:
  - `docs/design/TUSK-HARNESS-CONTRACT.md`
  - `docs/design/TUSK-HARNESS-SESSION.md`
  - `docs/design/TUSK-HARNESS-FEATURE-LEDGER.md`
  - `docs/design/TUSK-HARNESS-TRAJECTORY.md`
  - `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- lane composition and authority:
  - `draft/SPEC-INDEX.md`
  - `draft/UNIFICATION-DOCTRINE.md`
