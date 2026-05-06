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

Tracker posture:

- active sequencing is dynamic and sourced from the owning tracker,
- docs do not carry authoritative "current tracker item" pointers.

## 4. Operational Invariants

1. One authority artifact at each boundary; no parallel semantics.
2. Deterministic binding for equality/comparison (`normalizerId`, `policyDigest`).
3. Proposal/projection outputs never self-authorize admissibility.
4. Fail closed on unknown/unbound/ambiguous factorization paths.
5. Keep tracker ordering dynamic in the owning tracker, not hardcoded in docs.

## 5. Verification Surfaces

Core checks:

- `premath coherence-check`
- `premath traceability-check`
- `premath drift-budget-check`
- `premath repo-hygiene-check`

Tracker status:

- external to Premath; query the owning Tusk/downstream tracker.

## 6. Next Execution Lane

Near-term work is governance/operations unless a new implementation epic is
opened for `§10.6` Stage 1 (typed-core dual projection).

Live roadmap source (authoritative):

- owning Tusk/downstream tracker

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
- [x] preserve Stage 1 marker language in control-plane specs
- [x] map Stage 1 clauses to executable checks in traceability
- [x] define rollback preconditions/postconditions
- [x] define deterministic rollback witness minimum fields
- [x] verify rollback path preserves canonical authority identity

Validation commands:

- `premath coherence-check`
- `premath traceability-check`
- `premath drift-budget-check`
- `premath repo-hygiene-check`

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
- `specs/premath/draft/OBLIGATION-DISCHARGE.md`,
- `specs/premath/draft/GATE.md`.

Historical note:

- stage-3 issue IDs below are historical execution references,
- active ordering always comes from the owning tracker.

Deterministic Stage 3 order:

1. `bd-148` typed-only authority reads in checker consumers
2. `bd-152` required local-check projection removes alias-as-authority fallback
3. `bd-153` checker reporting removes alias fallback
4. `bd-155` local-check verification adds typed-authority fail-closed checks
5. `bd-150` replace transitional kernel sentinel with direct Core-obligation evidence path
6. `bd-151` docs/traceability/decision closure

Per-task gate set:

- consumer/runtime checks (`bd-148`, `bd-152`, `bd-153`, `bd-155`):
  - `premath coherence-check`
  - `cargo test -p premath-coherence`
  - `cargo test -p premath-cli`
- Core-obligation handoff checks (`bd-150`):
  - `premath coherence-check`
  - `premath drift-budget-check`
- docs closure (`bd-151`):
  - `premath traceability-check`
  - `premath drift-budget-check`
  - `premath repo-hygiene-check`

Before handoff, run the direct checker sequence in `README.md`.

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
