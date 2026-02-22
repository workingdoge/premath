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
  `artifacts/ciwitness/*`, `artifacts/observation/latest.json`),
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
| `boot` memory roots | `premath mcp-serve`, `.premath/issues.jsonl`, `artifacts/observation/latest.json` | partial |
| `step` mutation authority | `instruction-linked` mutation policy in MCP + instruction witness checks | present |
| `step` deterministic verification | `ci-required-attested` (`run_required_checks` + verify/decide) | present |
| `stop` lease + handoff | `harness-session` artifact + `issue_claim` / `issue_lease_renew` / `issue_lease_release` | partial |
| trajectory projection | Observation Surface v0 (`observe-build` / `observe`) | partial |
| replayable work-memory | issue/event replay + witness artifacts | present |

## 8. Gaps (remaining)

1. No typed feature/progress ledger for strict one-feature session closure.
2. No harness-specific trajectory row schema (step-level, witness-linked).
3. No single documented retry-policy table bound to failure classes.

## 9. Implementation slice plan (no math generalization required)

1. Add `HarnessFeatureLedger` schema + check command (machine-verifiable
   progress state).
2. Add step-trajectory append log + projection query (witness-ref linked).
3. Add deterministic retry-policy table in control-plane docs + enforcement
   hook in pipeline wrappers.

Each slice should ship with:

- one deterministic JSON schema,
- one command-surface entry,
- one integration test path,
- one issue-backed acceptance checklist.

## 10. Relation to existing docs/specs

- Runtime shape: `docs/design/TUSK-ARCHITECTURE.md`
- Harness handoff artifact: `docs/design/TUSK-HARNESS-SESSION.md`
- Identity/refinement/witness details:
  - `docs/design/TUSK-IDENTITY.md`
  - `docs/design/TUSK-REFINEMENT.md`
  - `docs/design/TUSK-WITNESSING.md`
- Runtime normative candidate (raw): `specs/premath/raw/TUSK-CORE.md`
- Authority boundaries:
  - `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`
  - `specs/premath/draft/BIDIR-DESCENT.md`
  - `specs/premath/draft/GATE.md`
