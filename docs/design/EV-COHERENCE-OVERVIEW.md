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

- algebraic closure epic `bd-121` is closed (tasks `bd-122`..`bd-127` closed),
- issue-note compaction `bd-129` is closed; note-size warnings reduced to zero,
- current open issue: `bd-67` (reviewer-pool governance follow-up).

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

If Stage 1 begins, keep scope minimal:

1. introduce typed-core projection identity surface,
2. enforce deterministic dual-projection parity checks,
3. promote only after fail-closed parity passes.
