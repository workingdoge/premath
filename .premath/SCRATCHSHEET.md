# Scratchsheet (Compaction Recovery)

Status: active working notes (non-authoritative)
Last updated: 2026-02-25

## North Star

Build one explicit Grothendieck constructor authority for control-plane worldization:

- one constructor lane in core,
- one route/world binding semantics path,
- one evidence factorization path (`eta_F -> Ev`),
- no parallel semantic authority in wrappers/fixtures.

## Active Issue Topology

Primary epic:

- `bd-364` `[EPIC] World Descent: Route Every Site Through Kernel` (open, pinned)

Active worker issue:

- none (handoff point reached; next ready issue is `bd-365`)

Execution chain (architecture-first):

1. `bd-369` architecture contract
2. `bd-365` spec/index + doctrine-inf/site glue
3. `bd-370` control-plane parity + gate wiring
4. `bd-366` implementation
5. `bd-367` conformance vectors
6. `bd-368` docs/traceability closure

Dependency wiring (current):

- `bd-365` blocks `bd-369`
- `bd-370` blocks `bd-365`
- `bd-366` blocks `bd-370`
- `bd-367` blocks `bd-366`
- `bd-368` blocks `bd-367`
- `bd-364` blocks `bd-369`, `bd-365`, `bd-370`, `bd-366`, `bd-367`, `bd-368`

Queue snapshot (2026-02-25):

- `ready`: 1 (`bd-365`)
- `in_progress`: 0
- `open`: 6
- `blocked`: 1 (`bd-67` governance/manual)

## Non-Negotiable Invariants

1. Kernel/Gate/BIDIR remain semantic admissibility authority.
2. Coherence/control-plane/wrappers are projection/parity lanes only.
3. World-route decisions are kernel-backed and deterministic.
4. Torsor/ext rows are overlay attachments only, never authority route targets.
5. All operational outputs consumed by control/runtime factor through `Ev`.

## Design Anchors In Use

Primary design docs currently grounding implementation choices:

- `docs/design/ARCHITECTURE-MAP.md` (layer map + Grothendieck operationalization section)
- `docs/design/DEVELOPMENT-META-LOOP.md` (work-order and lane discipline)
- `docs/design/README.md` (where-to-edit authority map)
- `docs/design/MEMORY-LANES-CONTRACT.md` (memory-lane write discipline)
- `docs/design/EV-COHERENCE-OVERVIEW.md` (evidence-lane posture and stage status)
- `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md` (worker/coordinator operational discipline)

Companion normative anchors used with the design docs:

- `specs/premath/draft/WORLD-REGISTRY.md`
- `specs/premath/draft/DOCTRINE-SITE.md`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`

## Planned Implementation Sequence

Current execution order (`bd-364` chain):

1. `bd-369` architecture contract.
2. `bd-365` world descent spec/index + doctrine-inf/site glue.
3. `bd-370` control-plane parity + gate wiring.
4. `bd-366` implementation (kernel-first world-route constructor/check path).
5. `bd-367` conformance vectors (golden/adversarial/invariance).
6. `bd-368` docs/traceability closure.

## Progress Snapshot

Current session updates:

- Refined issue topology to architecture-first ordering by adding:
  - `bd-369` architecture contract task,
  - `bd-370` control-plane parity task.
- Rewired dependencies so execution order matches AGENTS meta-loop.
- Completed `bd-369` and moved queue handoff to `bd-365` (spec slice).
- Updated current docs/spec wording for world-descent posture:
  - `specs/premath/draft/SPEC-INDEX.md` active epic list,
  - `specs/premath/draft/WORLD-REGISTRY.md` constructor/registry normativity,
  - `specs/premath/draft/SITE-RESOLVE.md` world-route validation execution requirement,
  - `README.md` transport language (generic first, BEAM as adapter),
  - `docs/design/ARCHITECTURE-MAP.md` WDAC-1 authority and execution-order contract,
  - `docs/design/README.md` onboarding link to WDAC-1.

Historical chain details are preserved in:

- `.premath/issues.jsonl` (authoritative issue notes),
- `.premath/OPERATIONS.md` (evidence log),
- `specs/process/decision-log.md` (boundary decisions).

## Key Verification Gates

- `mise run docs-coherence-check`
- `mise run traceability-check`
- `mise run coherence-check`
- `mise run doctrine-check`
- `mise run conformance-run`
- `python3 tools/ci/check_issue_graph.py`

## Fast Resume Commands

- `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`
- `cargo run --package premath-cli -- dep diagnostics --issues .premath/issues.jsonl --graph-scope active --json`
- `rg -n '"id":"bd-36[4-9]"|"id":"bd-370"' .premath/issues.jsonl`

## Update Discipline For This File

Update this scratchsheet whenever one of these changes:

1. active issue claim/release,
2. dependency rewiring,
3. authority boundary decisions,
4. constructor schema decisions,
5. implementation sequencing decisions.

This file is context-recovery memory only; canonical authority remains issues/specs/contracts.

## 2026-02-25 Topology Follow-up (bd-348)

- Closed `bd-348` by reducing doctrine-site source edges and regenerating
  generated doctrine artifacts.
- Source edge removals anchored in
  `specs/premath/site-packages/premath.doctrine_operation_site.v0/SITE-PACKAGE.json`:
  `e.doctrine.llm_instruction`, `e.doctrine.llm_proposal`,
  `e.ci.llm_instruction`, `e.ci.llm_proposal`,
  `e.llm_instruction.llm_proposal`, `e.llm_proposal.bidir`,
  `e.llm_instruction.op.run_instruction`.
- Regenerated outputs:
  `specs/premath/draft/DOCTRINE-SITE-INPUT.json`,
  `specs/premath/draft/DOCTRINE-SITE.json`,
  `specs/premath/draft/DOCTRINE-SITE-GENERATION-DIGEST.json`,
  `docs/design/generated/DOCTRINE-SITE-INVENTORY.json`,
  `docs/design/generated/DOCTRINE-SITE-INVENTORY.md`.
- Decision anchor added:
  `specs/process/decision-log.md` (Decision 0131).
- Verification anchors (all pass):
  `mise run doctrine-check`,
  `mise run docs-coherence-check`,
  `mise run ci-drift-budget-check`
  (`doctrineSiteEdgeCount=65`, `warnAbove=65`).

## 2026-02-25 Topology Follow-up (bd-349)

- Closed `bd-349` by reducing draft/topology warning pressure without changing
  thresholds.
- Reclassified control docs:
  - `specs/premath/draft/SPEC-INDEX.md` -> `status: informational`
  - `specs/premath/draft/SPEC-TRACEABILITY.md` -> `status: informational`
- Removed self-referential matrix rows from
  `specs/premath/draft/SPEC-TRACEABILITY.md`:
  - `SPEC-INDEX.md`
  - `SPEC-TRACEABILITY.md`
- Decision anchor added:
  `specs/process/decision-log.md` (Decision 0132).
- Verification anchors (all pass):
  `mise run ci-drift-budget-check`
  (`warningCount=0`, `draftSpecNodes=34`, `specTraceabilityRows=34`),
  `mise run traceability-check` (`draftSpecs=34`, `matrixRows=34`),
  `mise run docs-coherence-check`.

## 2026-02-25 Docs Coherence Follow-up (bd-350)

- Closed `bd-350` by aligning `SPEC-INDEX` section 0.3 with issue-memory
  authority.
- `specs/premath/draft/SPEC-INDEX.md` now reflects:
  - `bd-294` and `bd-332` as recently closed,
  - active epic list as empty (`none currently open/in-progress`).
- Verification anchors:
  `mise run docs-coherence-check` (pass),
  open/in-progress epic scan over `.premath/issues.jsonl` (0).

## 2026-02-25 New Epic Scaffold (bd-351 chain)

- Opened new epic `bd-351`:
  `[EPIC] World Kernel Self-Hosting Consolidation v1`.
- Added dependency-ordered task spine:
  - `bd-352` WKS-1 architecture contract (claimed, in_progress),
  - `bd-353` WKS-2 spec and doctrine-site glue,
  - `bd-354` WKS-3 control-plane parity enforcement,
  - `bd-355` WKS-4 implementation migration,
  - `bd-356` WKS-5 conformance invariance vectors,
  - `bd-357` WKS-6 docs and traceability closure.
- Dependency wiring:
  - chain blocks: `352 -> 353 -> 354 -> 355 -> 356 -> 357`,
  - epic blocked by all tasks (`bd-351` depends on `bd-352..bd-357`).
- Validation anchors:
  - `premath dep diagnostics --graph-scope active` (no cycles),
  - `premath issue check` (accepted),
  - `premath issue ready` (0 after claiming `bd-352`).

## 2026-02-25 World Self-Hosting Progress (bd-351 chain)

- Closed tasks:
  - `bd-352` WKS-1 architecture contract,
  - `bd-353` WKS-2 doctrine-site/spec glue,
  - `bd-354` WKS-3 control-plane parity enforcement,
  - `bd-355` WKS-4 wrapper-lane implementation migration (kernel authority only),
  - `bd-356` WKS-5 conformance vector extension.
- Active task:
  - `bd-357` WKS-6 docs/traceability closure (claimed, in progress).
- Design/spec anchors used during this chain:
  - `specs/premath/draft/WORLD-REGISTRY.md`
  - `specs/premath/draft/DOCTRINE-SITE.md`
  - `specs/premath/draft/PREMATH-COHERENCE.md`
  - `specs/premath/draft/SPEC-INDEX.md`
  - `specs/premath/draft/SPEC-TRACEABILITY.md`
  - `tools/conformance/README.md`
- Implementation anchors:
  - `crates/premath-kernel/src/runtime_orchestration.rs`
  - `tools/conformance/core_command_client.py`
  - `tools/conformance/check_runtime_orchestration.py`
  - `tools/conformance/run_world_core_vectors.py`
  - `tools/conformance/run_frontend_parity_vectors.py`
- New vector coverage anchors:
  - `tests/conformance/fixtures/runtime-orchestration/golden/world_route_transport_dispatch_bound_accept/case.json`
  - `tests/conformance/fixtures/runtime-orchestration/adversarial/world_route_transport_dispatch_missing_reject/case.json`
- Verification anchors (pass):
  - `cargo test --workspace`
  - `mise run doctrine-check`
  - `mise run coherence-check`
  - `python3 tools/conformance/run_runtime_orchestration_vectors.py`
  - `python3 tools/conformance/run_world_core_vectors.py`
  - `python3 tools/conformance/run_frontend_parity_vectors.py`
  - `mise run conformance-run`
