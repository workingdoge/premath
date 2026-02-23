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

### 1.1 Shared harness surface map (authoritative)

The harness surface is split into three normative contracts with one shared
authority map:

| Concern | Normative surface | Shared bindings and routes | Primary executable anchors |
| --- | --- | --- | --- |
| Runtime loop/session/trajectory | `draft/HARNESS-RUNTIME` | canonical artifacts (`.premath/harness_session.json`, `.premath/harness_feature_ledger.json`, `.premath/harness_trajectory.jsonl`) and routed operation IDs (`op/harness.session_*`, issue lease routes) | `premath harness-session ...`, `premath harness-feature ...`, `premath harness-trajectory ...`, `mise run harness-worker-loop`, `mise run harness-coordinator-loop` |
| Tool-calling closure/mutation gate | `draft/HARNESS-TYPESTATE` | typestate binding digests (`mutationPolicyDigest`, `governancePolicyDigest`, context/decomposition digests), fail-closed join/mutation classes, claim-gated governance provenance classes | `premath harness-join-check --input <json> --json`, `python3 tools/conformance/run_harness_typestate_vectors.py` |
| Retry/escalation wrappers | `draft/HARNESS-RETRY-ESCALATION` | canonical retry policy artifact + digest binding (`policies/control/harness-retry-policy-v1.json`), deterministic escalation action mapping and issue-context resolution order | `python3 tools/ci/test_harness_retry_policy.py`, `python3 tools/ci/test_harness_escalation.py`, `mise run ci-pipeline-test` |

`draft/HARNESS-TYPESTATE` and `draft/HARNESS-RETRY-ESCALATION` MUST reference
this section for shared harness surface partitioning and MUST NOT introduce
parallel authority routes.

### 1.2 Harness-Squeak composition boundary (required)

When runtime orchestration crosses world/location boundaries, Harness MUST route
through Squeak transport and destination Tusk checks in this order:

1. Harness computes deterministic work context and witness lineage refs.
2. Squeak performs transport/runtime-placement mapping and emits transport-class
   witness outcomes.
3. Destination Tusk/Gate performs destination-local admissibility checks and
   emits Gate-class outcomes.
4. Harness records the resulting references in session/trajectory projections.

Harness and Squeak are operational projection surfaces only. Neither may
introduce semantic admissibility authority; acceptance/rejection remains
checker/Gate-owned.

## 2. Canonical runtime artifacts and command anchors

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
- compute deterministic `ToolUse` projection state from normalized tool
  results, including typed consumed/observed/discarded dispositions as defined
  in `draft/HARNESS-TYPESTATE`,
- project deterministic state views over the append-only event stream and apply
  queue reductions required by active stop/enforcement policy before the next
  model/tool iteration,
- compute deterministic closure gate state (`JoinClosed`) from normalized
  tool/protocol/handoff evidence (including `ToolUse`) before any
  issue/dependency mutation attempt, following `draft/HARNESS-TYPESTATE`,
- when `profile.doctrine_inf_governance.v0` is claimed in
  `draft/CAPABILITY-REGISTRY.json` `profileOverlayClaims`, include bound
  `policyProvenance` evidence (`pinned`, `packageRef`, `expectedDigest`,
  `boundDigest`) in closure/mutation inputs and fail closed on unpinned or
  mismatched provenance before mutation,
- when `executionPattern` implies fan-out/fan-in, enforce decomposition policy
  admissibility before parallel worker dispatch,
- compute deterministic mutation-admissibility gate state (`MutationReady`) and
  permit issue/dependency mutations only when this gate is satisfied and
  required `ToolUse` evidence is present (fail-closed class semantics in
  `draft/HARNESS-TYPESTATE`),
- execute mutation through instruction-mediated surfaces,
- when work requires inter-world execution or runtime-location relocation,
  produce deterministic handoff inputs to Squeak transport and require
  destination Tusk/Gate evidence before treating the step as verified,
- run deterministic verification before and after mutation,
- emit typed witness references for side effects.

If closure or mutation-admissibility gates are unsatisfied, `step` MUST fail
closed by recording failure classes + witness references and skipping mutation;
projection artifacts MAY still be emitted.

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

Inter-world/runtime-placement transport remains in Squeak surfaces
(`raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`) and MUST compose with Harness routes as
one operational chain without introducing a parallel authority path.

## 9. Verification surfaces

Minimum deterministic verification includes:

- `mise run ci-hygiene-check`
- `python3 tools/ci/check_issue_graph.py`
- `mise run docs-coherence-check`
- `mise run doctrine-check`

## 10. Related surfaces

- design runbooks:
  - `docs/design/TUSK-HARNESS-CONTRACT.md`
  - `docs/design/TUSK-HARNESS-FEATURE-LEDGER.md`
  - `docs/design/TUSK-HARNESS-CONTRACT.md` (§11 session artifact, §12 trajectory, §13 KPI benchmark)
  - `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- lane composition and authority:
  - `draft/SPEC-INDEX.md`
- `draft/UNIFICATION-DOCTRINE.md`
- `draft/HARNESS-TYPESTATE.md`
