# Tusk Harness Trajectory

Status: draft
Scope: design-level, non-normative

## 1. Purpose

`HarnessTrajectory` captures bounded harness step outcomes as append-only rows.

- one row per step,
- witness-linked references (no payload duplication),
- deterministic projection queries for operator/agent handoff.

Trajectory rows are operational memory and not semantic authority.

## 2. Canonical Artifact

- default path: `.premath/harness_trajectory.jsonl`
- row kind: `premath.harness.step.v1`
- row schema: `1`

## 3. Row Schema

Required fields:

- `schema: 1`
- `stepKind: "premath.harness.step.v1"`
- `stepId: string`
- `action: string`
- `resultClass: string`
- `finishedAt: RFC3339`

Optional fields:

- `issueId: string`
- `instructionRefs: string[]`
- `witnessRefs: string[]`
- `startedAt: RFC3339`

Normalization rules:

- refs are trimmed, sorted, and deduplicated,
- empty optional values are dropped,
- malformed timestamps are rejected.

## 4. Deterministic Projections

Projection kind: `premath.harness.trajectory.projection.v1`

Modes:

- `latest`
- `failed`
- `retry-needed`

Ordering:

- descending `finishedAt`,
- tie-break by `stepId`, then `action`.

The projection output reports aggregate counters (`totalCount`, `failedCount`,
`retryNeededCount`) plus deterministic `items`.

## 5. Command Surface

- `premath harness-trajectory append --path .premath/harness_trajectory.jsonl --step-id <id> --action <action> --result-class <class> --witness-ref <ref> --json`
- `premath harness-trajectory query --path .premath/harness_trajectory.jsonl --mode latest|failed|retry-needed --limit 20 --json`

## 6. Related Docs

- `docs/design/TUSK-HARNESS-CONTRACT.md`
- `docs/design/TUSK-HARNESS-SESSION.md`
- `docs/design/TUSK-HARNESS-FEATURE-LEDGER.md`
