# Ev Stage3 Execution Runbook

This runbook defines deterministic execution order and hygiene cadence for
`UNIFICATION-DOCTRINE` Stage 3 (`typed-first cleanup`).

Normative authority remains in:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md` (§10.6),
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`,
- `specs/premath/draft/PREMATH-KERNEL.md`,
- `specs/premath/draft/BIDIR-DESCENT.md`,
- `specs/premath/draft/GATE.md`.

This document is operational guidance for execution order only.

Status note:

- Stage 3 issue IDs listed below are historical execution references.
- Authoritative active work ordering is always `.premath/issues.jsonl` queried
  via `premath issue ready` / `premath issue list`.

## 1) Stage3 Task Order

Execute in this order to avoid drift:

1. `bd-148` typed-only authority reads in CI/CLI/MCP consumers.
2. `bd-152` required pipeline summary: remove alias-as-authority fallback.
3. `bd-153` decision verification reporting: remove alias fallback.
4. `bd-155` required-decision verify client: typed-authority fail-closed checks.
5. `bd-149` typed-first observation/projection query contract.
6. `bd-154` explicit alias compatibility mode for projection queries.
7. `bd-150` replace transitional kernel sentinel with direct bidir evidence path.
8. `bd-151` docs/traceability/decision closure.

## 2) Per-Task Gate Set

Run the smallest deterministic gate set per task:

- Consumer/runtime checks (`bd-148`, `bd-152`, `bd-153`, `bd-155`):
  - `mise run ci-pipeline-test`
  - `python3 tools/conformance/run_capability_vectors.py --capability capabilities.ci_witnesses`
  - `cargo test -p premath-coherence`
  - `cargo test -p premath-cli`
- Observation/query checks (`bd-149`, `bd-154`):
  - `mise run ci-observation-test`
  - `cargo test -p premath-surreal`
  - `cargo test -p premath-ux`
- Bidir-handoff checks (`bd-150`):
  - `mise run coherence-check`
  - `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract`
  - `python3 tools/ci/test_drift_budget.py`
- Docs closure (`bd-151`):
  - `mise run docs-coherence-check`
  - `mise run traceability-check`
  - `python3 tools/ci/check_issue_graph.py`

Before push, run full closure:

- `mise run ci-required-attested`

## 3) Commit And Push Cadence

Use small, deterministic slices:

1. One issue or one tightly related issue-set per commit.
2. Run issue-specific gate set before each commit.
3. Push after each green commit (pre-push baseline is the final guard).
4. Do not batch unrelated issue work into one commit.

## 4) Issue Update Cadence

For each issue in this lane:

1. set `status=in_progress` before edits,
2. append concise notes with:
   - changed surfaces,
   - deterministic failure classes (if changed),
   - exact verification commands run,
3. set `status=closed` only after gates pass.

Keep dependency ordering accurate (`dep add`) before opening downstream work.

## 5) Stop Conditions

Pause and open a follow-up issue when either occurs:

1. a required gate cannot be made green in the current slice without widening
   scope materially,
2. authority semantics become ambiguous (typed vs alias behavior cannot be
   stated as one deterministic rule).
