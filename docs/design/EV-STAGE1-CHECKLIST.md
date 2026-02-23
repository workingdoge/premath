# Ev Stage 1 Checklist

Status: draft
Scope: design-level, non-normative
Anchor: `specs/premath/draft/UNIFICATION-DOCTRINE.md` §10.6 (Stage 1)

## 1. Goal

Execute Stage 1 (`typed-core dual projection`) with:

- one authority artifact preserved,
- deterministic parity between payload authority and typed-core projection,
- fail-closed mismatch behavior,
- deterministic rollback to Stage 0 when parity fails.

## 2. Stage 1 Deliverables

1. Typed-core identity profile:
   - define minimal typed-core projection payload shape,
   - define deterministic typed-core identity/ref shape,
   - bind to `normalizerId` + `policyDigest`.
2. Dual-projection parity contract:
   - define deterministic projection from authority payload -> typed-core view,
   - define deterministic replay path typed-core view -> comparison surface,
   - define parity check result shape.
3. Fail-closed class surface:
   - explicit classes for missing projection, mismatch, and unbound comparison
     context.
4. Rollback contract:
   - deterministic rollback trigger criteria,
   - rollback preserves prior authority identities and refuses second authority
     artifacts.

## 3. Checklist

## 3.1 Contract surfaces

- [x] Add Stage 1 typed-core profile section under normative `Ev` contract path.
- [x] Add deterministic field-level binding requirements for parity inputs.
- [x] Define canonical parity result payload shape.

## 3.2 Checker wiring

- [x] Add checker-parity obligation hook for Stage 1 dual projections.
- [x] Emit deterministic fail-closed class on missing/mismatch/unbound cases.
- [x] Keep semantic authority unchanged (checker verifies, does not authorize).

## 3.3 Conformance and docs coherence

- [x] Add/update vectors for accepted/rejected Stage 1 parity paths.
- [x] Extend docs-coherence checks to require Stage 1 marker language.
- [x] Ensure traceability row maps Stage 1 clauses to executable checks.

## 3.4 Rollback readiness

- [x] Define rollback preconditions and postconditions.
- [x] Define deterministic rollback witness payload minimum fields.
- [x] Verify rollback path does not alter canonical authority identity.

## 4. Validation Commands

- `mise run coherence-check`
- `mise run docs-coherence-check`
- `mise run traceability-check`
- `python3 tools/ci/check_issue_graph.py`

## 5. Work Slice Plan

1. Contract slice:
   - add minimal normative Stage 1 profile text + class names.
2. Checker slice:
   - implement parity hook + fail-closed output surface.
3. Vector slice:
   - add accepted/rejected Stage 1 vectors.
4. Rollback slice:
   - add deterministic rollback witness + docs/spec glue.

Execution note (2026-02-22):
- Slices 1-4 are implemented in repository surfaces:
  - `draft/UNIFICATION-DOCTRINE` §10.6.1/§10.6.2/§10.6.3,
  - `draft/CONTROL-PLANE-CONTRACT` Stage 1 parity + rollback objects,
  - `premath-coherence` `gate_chain_parity` fail-closed enforcement,
  - coherence-site `gate_chain_parity_stage1_*` vectors.
