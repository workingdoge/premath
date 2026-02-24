# Tusk Harness Contract

Status: draft
Scope: design-level, non-normative

## 1. Why this document

Tusk already has strong runtime contracts (`TUSK-ARCHITECTURE`, identity,
descent packs, witnessing). This document narrows one specific question:

- where the long-running agent harness lives,
- what that harness must do,
- which existing surfaces already satisfy the contract,
- which gaps remain.

Boundary rule (unchanged):

- harness logic is operational control, not semantic authority.
- model output remains proposal material.
- checker/discharge/witness artifacts remain authoritative.

## 2. Harness shape (minimum encoding)

Harness = Tusk runtime control loop with three hooks:

- `boot`: initialize one working session from persisted state.
- `step`: execute one bounded unit of work with deterministic verification.
- `stop`: emit handoff artifacts for the next fresh-context session.

This intentionally avoids introducing a parallel semantic schema.

## 3. Hook contract

### 3.1 `boot`

Required effects:

- resolve canonical memory roots (`.premath/issues.jsonl`,
  `.premath/harness_session.json`, `artifacts/ciwitness/*`,
  `artifacts/observation/latest.json`),
- run dependency-integrity preflight (`dep_diagnostics` with `active` scope),
- load previous session handoff summary (if present),
- compute next actionable target from issue graph + policy bindings,
- run baseline startup verification for current working scope.

### 3.2 `step`

Required effects:

- claim one bounded work item (single-task discipline by default),
- execute mutation path through instruction-mediated surfaces,
- run deterministic verification before and after mutation,
- emit typed witness references for all side effects.

### 3.3 `stop`

Required effects:

- persist compact handoff state for continuation,
- release/renew lease deterministically,
- publish trajectory row(s) linking:
  - issue/mutation identity,
  - verification result,
  - witness refs,
  - site lineage refs (`ctx/cover/refinement`),
  - next-step recommendation.

## 4. Durability contract

Long-run durability requirements:

- fresh-context restartability: every session must be resumable from files and
  witness refs (not hidden prompt state),
- bounded context growth: compaction/offloading checkpoints at session
  boundaries,
- explicit sub-agent boundaries: parent/child work must be materialized in
  issue/memory surfaces, not implicit chat branches.

## 5. Verification and retry contract

Each step must have a fail-closed verify/retry policy:

- verification class:
  - semantic check failure,
  - operational wiring failure,
  - flaky/transient execution failure.
- retry policy:
  - deterministic max-attempts + backoff class,
  - typed escalation path (`issue_discover` / blocked state / stop).

No silent pass-through is allowed for failed required checks.

## 6. Trajectory/evidence contract

Trajectory capture should be minimal but replayable:

- one append record per step,
- references to existing witness artifacts (avoid duplicating payloads),
- enough typed metadata to support:
  - replay,
  - failure clustering,
  - policy refinement.

Trajectory records are an operational lane, not semantic authority.

## 7. Mapping to current repository surfaces

| Harness clause | Current surface | Status |
|---|---|---|
| `boot` memory roots | `premath mcp-serve`, `.premath/issues.jsonl`, `.premath/harness_session.json`, `artifacts/ciwitness/*`, `artifacts/observation/latest.json`, `dep_diagnostics(active)` | present |
| `boot` deterministic next feature | `harness-feature` ledger (`next`/`check`) + `harness-session bootstrap` projection | present |
| `step` mutation authority | `instruction-linked` mutation policy in MCP + instruction witness checks | present |
| `step` deterministic verification | `ci-required-attested` (`run_required_checks` + verify/decide) | present |
| `stop` lease + handoff | issue-memory-derived lease state + `harness-session`/`harness-trajectory` `lease://handoff/...` refs + `issue_claim` / `issue_lease_renew` / `issue_lease_release` | present |
| trajectory projection | `harness-trajectory` rows + deterministic `query` projection (`latest`/`failed`/`retry-needed`) | present |
| replayable work-memory | issue/event replay + witness artifacts | present |

## 8. Gaps (remaining)

No known remaining harness-v1 contract gaps are tracked in this document at the
current closure state.

Operational maintenance (continuous, not a contract gap):

1. Continue expanding failure-class coverage from observed CI/harness runs while
   preserving policy-digest discipline in the canonical retry policy surface.

## 9. Implementation slice plan (no math generalization required)

Current status:

1. Failure-class expansion + retry/escalation alignment is closed under
   historical issue `bd-190`.
2. Deterministic issue-context bootstrap via env/session fallback
   (`PREMATH_ACTIVE_ISSUE_ID` / `PREMATH_ISSUE_ID` /
   `PREMATH_HARNESS_SESSION_PATH` -> `.premath/harness_session.json`) is
   specified and implemented in `draft/HARNESS-RETRY-ESCALATION`.

Next slices should be opened only for net-new gaps discovered in runtime
evidence, not for already-closed closure items.

Each slice should ship with:

- one deterministic JSON schema,
- one command-surface entry,
- one integration test path,
- one issue-backed acceptance checklist.

## 10. Relation to existing docs/specs

- Runtime shape: `docs/design/TUSK-ARCHITECTURE.md`
- Harness handoff artifact: this doc §11
- Harness feature ledger: `docs/design/TUSK-HARNESS-FEATURE-LEDGER.md`
- Harness trajectory rows: this doc §12
- Multithread runbook: `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- KPI benchmark: this doc §13
- Harness retry policy table: `docs/design/TUSK-HARNESS-RETRY-POLICY.md`
- Identity/refinement/witness details:
  - `docs/design/TUSK-IDENTITY.md`
  - `docs/design/TUSK-REFINEMENT.md`
  - `docs/design/TUSK-WITNESSING.md`
- Runtime normative candidate (raw): `specs/premath/raw/TUSK-CORE.md`
- Authority boundaries:
  - `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`
  - `specs/premath/draft/BIDIR-DESCENT.md`
  - `specs/premath/draft/GATE.md`

## 11. Harness Session Artifact (Consolidated)

`HarnessSession` is the minimal handoff artifact for fresh-context
restartability.

- it carries compact stop/boot continuity state,
- it references existing authority artifacts (issues/instructions/witnesses),
- it does not introduce parallel semantic authority.

Canonical artifact:

- default path: `.premath/harness_session.json`
- kind: `premath.harness.session.v1`
- schema: `1`

Required fields:

- `schema: 1`
- `sessionKind: "premath.harness.session.v1"`
- `sessionId: string`
- `state: "active" | "stopped"`
- `startedAt: RFC3339`
- `updatedAt: RFC3339`

Optional fields:

- `issueId: string`
- `summary: string`
- `nextStep: string`
- `instructionRefs: string[]` (sorted + deduplicated)
- `witnessRefs: string[]` (sorted + deduplicated; includes stop/handoff
  `lease://handoff/...` refs when lease recovery is required)
- `lineageRefs: string[]` (sorted + deduplicated; operational site lineage refs
  such as `ctx://...`, `cover://...`, `refinement://...`)
- `stoppedAt: RFC3339` (present when `state = stopped`)
- `issuesPath: string`
- `issuesSnapshotRef: string` (derived via `store_snapshot_ref`)

Command surface:

- `premath harness-session write --path <session.json> --state active|stopped ... --json`
- `premath harness-session read --path <session.json> --json`
- `premath harness-session bootstrap --path <session.json> --json`

`bootstrap` output:

- kind: `premath.harness.bootstrap.v1`
- `mode`:
  - `resume` when session state is `stopped`
  - `attach` when session state is `active`
- optional feature projection fields when feature ledger is available:
  - `nextFeatureId`
  - `featureClosureComplete`
  - `featureCount`

Determinism rules:

- update-in-place preserves `sessionId` unless explicitly overridden,
- update-in-place preserves `startedAt`; always refreshes `updatedAt`,
- `issuesSnapshotRef` is stable for unchanged issue-memory state,
- empty/whitespace optional string inputs are normalized to absent values.

## 12. Harness Trajectory (Consolidated)

`HarnessTrajectory` captures bounded harness step outcomes as append-only rows.

- one row per step,
- witness-linked references (no payload duplication),
- deterministic projection queries for operator/agent handoff.

Canonical artifact:

- default path: `.premath/harness_trajectory.jsonl`
- row kind: `premath.harness.step.v1`
- row schema: `1`

Required row fields:

- `schema: 1`
- `stepKind: "premath.harness.step.v1"`
- `stepId: string`
- `action: string`
- `resultClass: string`
- `finishedAt: RFC3339`

Optional row fields:

- `issueId: string`
- `instructionRefs: string[]`
- `witnessRefs: string[]`
- `lineageRefs: string[]`
- `startedAt: RFC3339`

Worker-loop convention:

- stop/handoff rows include deterministic lease witness references:
  `lease://handoff/<issue-id>/<lease-state>/<digest>`
- stop/handoff rows include deterministic site lineage refs:
  `ctx://...`, `cover://...`, `refinement://...`

Normalization rules:

- refs are trimmed, sorted, and deduplicated,
- empty optional values are dropped,
- malformed timestamps are rejected.

Deterministic projection:

- projection kind: `premath.harness.trajectory.projection.v1`
- modes: `latest`, `failed`, `retry-needed`
- ordering: descending `finishedAt`, then `stepId`, then `action`
- output includes `totalCount`, `failedCount`, `retryNeededCount`, and `items`

Command surface:

- `premath harness-trajectory append --path .premath/harness_trajectory.jsonl --step-id <id> --action <action> --result-class <class> --witness-ref <ref> --json`
- `premath harness-trajectory query --path .premath/harness_trajectory.jsonl --mode latest|failed|retry-needed --limit 20 --json`

## 13. Multithread KPI Benchmark (Consolidated)

Canonical KPI:

- KPI kind: `premath.multithread.throughput.v1`
- formula: `kpi = throughput_per_worker_per_day * gate_pass_rate`

where:

- `throughput_per_worker_per_day = completed_rows_per_day / max(active_workers, 1)`
- `completed_rows_per_day = completed_rows * (24 / window_hours)`
- `gate_pass_rate = completed_rows / window_rows`

Row source: `.premath/harness_trajectory.jsonl` windowed by `finishedAt`.

Deterministic benchmark procedure:

1. choose deterministic window (`window_hours`, default `24`),
2. load trajectory rows and sort descending by `(finishedAt, stepId, action)`,
3. classify success rows with canonical success classes,
4. compute counts/ratios/KPI,
5. evaluate thresholds and emit one decision state.

Command surface:

- `python3 tools/harness/benchmark_kpi.py --json`
- `mise run harness-kpi-report`

Thresholds and rollback trigger:

- target KPI: `0.8`
- rollback KPI: `0.4`
- minimum sample rows: `3`

Decision states:

- `pass`: KPI >= target
- `watch`: rollback <= KPI < target
- `rollback`: KPI < rollback
- `insufficient_data`: window rows < minimum sample rows

Rollback trigger:

- `rollback` means deterministic multithread regression and pauses expansion
  until remediation is recorded.

Evidence boundary:

- benchmark output is control-plane operational evidence only,
- it does not alter semantic authority, checker verdicts, or issue-memory
  mutation authority.

## 14. REPL Host API v0 (Phase-3 Transition)

This section defines a compact `scheme_eval`-style control surface for agent
workers. It does not replace harness supervision.

Boundary rules:

- REPL is orchestration/planning only.
- Host functions are the only effect/mutation path.
- Mutation host calls remain instruction-linked and fail closed when authority
  evidence is missing.
- Direct shell/network access is denied by default for the evaluator runtime.

### 14.1 Host call envelope

Each host call should return one typed envelope:

- `schema`: `premath.host_effect.v0`
- `action`: canonical action id (for example `issue.claim`)
- `argsDigest`: deterministic digest over canonicalized args
- `resultClass`: deterministic success/failure class
- `payload`: action-specific typed JSON payload
- `witnessRefs`: ordered refs emitted/consumed by this effect
- `policyDigest`: required for mutation-capable actions
- `instructionRef`: required for mutation-capable actions

### 14.2 Function families and current command mappings

Read/query (no mutation authority):

- `issue.list|ready|blocked|check` ->
  `premath issue list|ready|blocked|check ... --json`
- `dep.diagnostics` ->
  `premath dep diagnostics ... --json`
- `observe.latest|needs_attention|instruction|projection` ->
  `premath observe ... --json`
- `instruction.check` ->
  `premath instruction-check ... --json`
- `coherence.check` ->
  `premath coherence-check ... --json`
- `required.projection|delta|gate_ref|witness.verify|decision.verify` ->
  `premath required-* ... --json`

Mutation (instruction-linked authority required):

- `issue.claim|update|discover|lease_renew|lease_release` ->
  `premath issue ... --json`
- `dep.add|remove|replace` ->
  `premath dep ... --json`
- `instruction.run` ->
  `sh tools/ci/run_instruction.sh ...` or `mise run ci-instruction`
- `harness.session.write` ->
  `premath harness-session write ... --json`
- `harness.feature.write` ->
  `premath harness-feature write ... --json`
- `harness.trajectory.append` ->
  `premath harness-trajectory append ... --json`

### 14.3 Harness integration shape

Keep coordinator/worker loop unchanged:

1. worker claims issue,
2. worker executes one bounded REPL program via `scheme_eval`,
3. program emits host-effect rows and optional harness trajectory refs,
4. existing verify/close/escalate logic remains authoritative in harness.

This preserves the current durability lanes:

- `.premath/issues.jsonl`
- `.premath/harness_session.json`
- `.premath/harness_feature_ledger.json`
- `.premath/harness_trajectory.jsonl`

### 14.4 Migration posture

Recommended sequence:

1. implement read-only `scheme_eval`,
2. prove deterministic replay/effect parity,
3. add mutation host calls with instruction-linked gating,
4. switch worker `work-cmd` to `scheme_eval`,
5. keep verification gates unchanged until parity is proven.
