# Ev + Coherence Overview

Status: draft
Scope: design-level, non-normative
Snapshot date: 2026-02-22

## 1. Purpose

Provide one compact operator/agent snapshot for:

- Unified Evidence Plane (`Ev`) direction,
- coherence/checker role and boundaries,
- issue-graph execution posture.

Normative authority remains under `specs/`.

## 2. Canonical Contracts

Primary normative anchors:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md`:
  - §10 Unified Evidence Plane contract (`Ev : Ctx^op -> V`)
  - §10.5 fail-closed factorization boundary
  - §10.6 staged typed evidence internalization + rollback
  - §11 cross-layer obstruction algebra
- `specs/premath/draft/PREMATH-COHERENCE.md`:
  - deterministic control-plane checker obligation surface
- `specs/premath/draft/SPEC-INDEX.md`:
  - lane ownership, reading order, claim/profile boundaries
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`
- `specs/premath/draft/COHERENCE-CONTRACT.json`

## 3. Current State

Unified Evidence Plane:

- one evidence family route is explicit and fail closed (`eta_F : F => Ev`),
- factorization failure classes are explicit:
  - `unification.evidence_factorization.missing`
  - `unification.evidence_factorization.ambiguous`
  - `unification.evidence_factorization.unbound`
- typed migration is staged (`§10.6`) with deterministic rollback constraints.

Coherence role:

- coherence is control-plane check role, not kernel semantic authority,
- semantic admissibility remains kernel/Gate-owned,
- cross-lane composition claims route through typed span/square witnesses.

Issue-memory posture:

- historical closure work (algebraic closure + note compaction) is complete,
- active sequencing is dynamic and sourced from `.premath/issues.jsonl`,
- docs do not carry authoritative "current issue" pointers.

## 4. Operational Invariants

1. One authority artifact at each boundary; no parallel semantics.
2. Deterministic binding for equality/comparison (`normalizerId`, `policyDigest`).
3. Proposal/projection outputs never self-authorize admissibility.
4. Fail closed on unknown/unbound/ambiguous factorization paths.
5. Keep issue ordering dynamic from `.premath/issues.jsonl` (`issue_ready`), not
   hardcoded in docs.

## 5. Verification Surfaces

Core checks:

- `mise run coherence-check`
- `mise run docs-coherence-check`
- `mise run traceability-check`
- `python3 tools/ci/check_issue_graph.py`

Issue graph status:

- `cargo run --package premath-cli -- issue list --issues .premath/issues.jsonl --json`
- `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`

## 6. Next Execution Lane

Near-term work is governance/operations unless a new implementation epic is
opened for `§10.6` Stage 1 (typed-core dual projection).

Live roadmap source (authoritative):

- `cargo run --package premath-cli -- issue list --issues .premath/issues.jsonl --json`
- `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`

If Stage 1 begins, keep scope minimal:

1. introduce typed-core projection identity surface,
2. enforce deterministic dual-projection parity checks,
3. promote only after fail-closed parity passes.

## 7. Stage 1 Checklist (Consolidated)

Anchor: `specs/premath/draft/UNIFICATION-DOCTRINE.md` §10.6 (Stage 1).

Goal:

- preserve one authority artifact,
- enforce deterministic parity between payload authority and typed-core projection,
- fail closed on mismatch,
- keep deterministic rollback to Stage 0 when parity fails.

Stage 1 deliverables:

1. typed-core identity profile:
   - minimal typed-core projection payload shape,
   - deterministic typed-core identity/ref shape,
   - binding to `normalizerId` + `policyDigest`.
2. dual-projection parity contract:
   - deterministic projection `authority payload -> typed-core view`,
   - deterministic replay `typed-core view -> comparison surface`,
   - canonical parity result shape.
3. fail-closed classes:
   - explicit classes for missing projection, mismatch, unbound comparison context.
4. rollback contract:
   - deterministic rollback trigger criteria,
   - rollback preserves prior authority identities and rejects second authority artifacts.

Stage 1 checklist:

- [x] add Stage 1 typed-core profile section under normative `Ev` path
- [x] add deterministic field-level parity input bindings
- [x] define canonical parity result payload shape
- [x] add checker parity obligation hook
- [x] emit deterministic fail-closed class for missing/mismatch/unbound
- [x] keep semantic authority unchanged (checker verifies, never authorizes)
- [x] add/update vectors for accepted/rejected Stage 1 parity paths
- [x] extend docs-coherence checks for Stage 1 marker language
- [x] map Stage 1 clauses to executable checks in traceability
- [x] define rollback preconditions/postconditions
- [x] define deterministic rollback witness minimum fields
- [x] verify rollback path preserves canonical authority identity

Validation commands:

- `mise run coherence-check`
- `mise run docs-coherence-check`
- `mise run traceability-check`
- `python3 tools/ci/check_issue_graph.py`

Execution note (2026-02-22):

- stage slices are implemented in repository surfaces:
  - `draft/UNIFICATION-DOCTRINE` §10.6.1/§10.6.2/§10.6.3,
  - `draft/CONTROL-PLANE-CONTRACT` Stage 1 parity + rollback objects,
  - `premath-coherence` `gate_chain_parity` fail-closed enforcement,
  - coherence-site `gate_chain_parity_stage1_*` vectors.

## 8. Stage 3 Execution Runbook (Consolidated)

Anchor: `UNIFICATION-DOCTRINE` Stage 3 (`typed-first cleanup`).

Normative authority remains in:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md` (§10.6),
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`,
- `specs/premath/draft/PREMATH-KERNEL.md`,
- `specs/premath/draft/BIDIR-DESCENT.md`,
- `specs/premath/draft/GATE.md`.

Historical note:

- stage-3 issue IDs below are historical execution references,
- active ordering always comes from `.premath/issues.jsonl` via
  `premath issue ready` / `premath issue list`.

Deterministic Stage 3 order:

1. `bd-148` typed-only authority reads in CI/CLI/MCP consumers
2. `bd-152` required pipeline summary removes alias-as-authority fallback
3. `bd-153` decision verification reporting removes alias fallback
4. `bd-155` required-decision verify client adds typed-authority fail-closed checks
5. `bd-149` typed-first observation/projection query contract
6. `bd-154` explicit alias compatibility mode for projection queries
7. `bd-150` replace transitional kernel sentinel with direct bidir evidence path
8. `bd-151` docs/traceability/decision closure

Per-task gate set:

- consumer/runtime checks (`bd-148`, `bd-152`, `bd-153`, `bd-155`):
  - `mise run ci-pipeline-test`
  - `python3 tools/conformance/run_capability_vectors.py --capability capabilities.ci_witnesses`
  - `cargo test -p premath-coherence`
  - `cargo test -p premath-cli`
- observation/query checks (`bd-149`, `bd-154`):
  - `mise run ci-observation-test`
  - `cargo test -p premath-surreal`
  - `cargo test -p premath-ux`
- bidir-handoff checks (`bd-150`):
  - `mise run coherence-check`
  - `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract`
  - `python3 tools/ci/test_drift_budget.py`
- docs closure (`bd-151`):
  - `mise run docs-coherence-check`
  - `mise run traceability-check`
  - `python3 tools/ci/check_issue_graph.py`

Before push:

- `mise run ci-required-attested`

Commit and issue cadence:

1. one issue (or tightly related issue-set) per commit,
2. run issue-specific gates before each commit,
3. push after each green commit,
4. avoid batching unrelated work.

For each issue:

1. set `status=in_progress` before edits,
2. append concise notes with changed surfaces, classes, and exact verification commands,
3. set `status=closed` only after gates pass.

Stop conditions:

1. required gate cannot be made green in current slice without material scope widening,
2. authority semantics become ambiguous (typed vs alias cannot be stated deterministically).
