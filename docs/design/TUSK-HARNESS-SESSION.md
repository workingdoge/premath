# Tusk Harness Session Artifact

Status: draft
Scope: design-level, non-normative

## 1. Purpose

`HarnessSession` is the minimal handoff artifact for fresh-context restartability.

- it carries compact stop/boot continuity state,
- it references existing authority artifacts (issues/instructions/witnesses),
- it does not introduce parallel semantic authority.

Authoritative semantics remain in checker/discharge/witness surfaces.

## 2. Canonical Artifact

- default path: `.premath/harness_session.json`
- kind: `premath.harness.session.v1`
- schema: `1`

## 3. Schema Fields

Required:

- `schema: 1`
- `sessionKind: "premath.harness.session.v1"`
- `sessionId: string`
- `state: "active" | "stopped"`
- `startedAt: RFC3339`
- `updatedAt: RFC3339`

Optional:

- `issueId: string`
- `summary: string`
- `nextStep: string`
- `instructionRefs: string[]` (canonicalized: sorted + deduplicated)
- `witnessRefs: string[]` (canonicalized: sorted + deduplicated)
- `stoppedAt: RFC3339` (present when `state = stopped`)
- `issuesPath: string`
- `issuesSnapshotRef: string` (derived via `store_snapshot_ref`)

## 4. Command Surface

- `premath harness-session write --path <session.json> --state active|stopped ... --json`
- `premath harness-session read --path <session.json> --json`
- `premath harness-session bootstrap --path <session.json> --json`

`bootstrap` emits:

- kind: `premath.harness.bootstrap.v1`
- `mode`:
  - `resume` when session state is `stopped`
  - `attach` when session state is `active`

## 5. Determinism Rules

- Update-in-place preserves `sessionId` unless explicitly overridden.
- Update-in-place preserves `startedAt`; always refreshes `updatedAt`.
- `issuesSnapshotRef` is stable for unchanged issue-memory state.
- Empty/whitespace optional string inputs are normalized to absent values.

## 6. Related Docs

- `docs/design/TUSK-HARNESS-CONTRACT.md`
- `docs/design/TUSK-ARCHITECTURE.md`
