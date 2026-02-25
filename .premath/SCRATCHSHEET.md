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

- `bd-238` `[EPIC] Coherence Constructor Authority (typechecker-first, fixtures replay-only)` (closed)

Completed foundation/spec implementation chain:

- `bd-239` `[CC0] Constructor authority architecture contract` (closed)
- `bd-240` `[CC1] Spec/index/traceability alignment` (closed)
- `bd-241` `[CC2] Typed coherence constructor in core` (closed)
- `bd-242` `[CC3] Wrapper parity via constructor outputs` (closed)

1. `bd-247` `[GC0] Spec: Explicit Grothendieck Constructor Contract` (closed)
2. `bd-248` `[GC1] Spec: Doctrine-Site to Constructor Total-Binding Contract` (closed)
3. `bd-249` `[GC2] Spec: Torsor Overlay as Non-Authority Constructor Attachment` (closed)

Current execution chain for fixture demotion:

1. `bd-250` `[CC4a] Strip fixture-side semantic verdict logic from conformance runners` (closed)
2. `bd-251` `[CC4b] Add fail-closed sentinels for core-authority fixture execution` (closed)
3. `bd-252` `[CC4c] Cleanup dead helpers + replay-role docs` (closed)

Coordinator + downstream:

- `bd-243` `[CC4] Fixture-runner demotion to replay/parity surfaces` (closed)
- `bd-244` `[CC5] Constructor-anchored conformance vectors` (closed)
- `bd-245` `[CC6] Newcomer/docs rewrite around constructor-first flow` (closed)
- `bd-246` `[CC7] Topology cleanup + epic closure sentinels` (closed)

Current post-CC8 route-closure chain:

1. `bd-253` `[KR0] Host-action operationId closure inventory` (closed)
2. `bd-254` `[KR1] Bind unmapped host actions into doctrine op registry` (closed)
3. `bd-255` `[KR2] Route-qualify required/harness actions through site/world bindings` (closed)

Current NX transport/parity chain:

1. `bd-256` `[NX0] Transport route contract for frontend dispatch` (closed)
2. `bd-257` `[NX1] Kernel admission closure for route-bound host actions` (closed)
3. `bd-258` `[NX2] Parallel harness topology proof (leases + dependency-safe dispatch)` (closed)
4. `bd-259` `[NX3] Frontend parity vectors for Scheme and Rhai over one kernel route` (closed)

Current evaluator UX chain:

1. `bd-261` `[UX1] Evaluator reject outcomes return non-zero exit status` (closed)
2. `bd-263` `[UX2] Non-JSON evaluator failures print actionable diagnostics` (closed)
3. `bd-265` `[UX4] Align Scheme/Rhai input model and precedence rules` (closed)
4. `bd-264` `[UX3] Add evaluator scaffold command for first-run usage` (closed)
5. `bd-269` `[UX5] Output-mode parity: add --json support for init surface` (closed)
6. `bd-260` `[EPIC] Evaluator UX hardening (scheme/rhai command surface)` (closed)

Current queue state:

- `ready`: 0
- `in_progress`: 0
- `open`: 0

Dependency wiring:

- `bd-247` blocks `bd-240`
- `bd-248` blocks `bd-247`
- `bd-249` blocks `bd-248`
- `bd-241` blocks `bd-249`

Compactness note:

- Removed redundant `bd-241 -> bd-240` edge to keep issue-graph compactness clean.

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

After spec slices (`bd-239`, `bd-247`, `bd-248`, `bd-249`) close:

1. Add explicit constructor type in `crates/premath-kernel`.
2. Build constructor from:
   - `specs/premath/draft/DOCTRINE-SITE-INPUT.json`
   - `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`
3. Route `world-registry-check` + `site-resolve` + coherence consumers through constructor outputs.
4. Keep `tools/ci/*` and `tools/conformance/*` adapter-only (no semantic re-derivation).
5. Add torsor overlay validation (bound to `baseRef`, fail-closed misuse classes).
6. Add conformance vectors for constructor + overlay invariants.

## Progress Snapshot

Completed in current session:

- Added explicit constructor object contract to
  `specs/premath/draft/WORLD-REGISTRY.md` (§2.5).
- Added doctrine-site constructor total-binding contract to
  `specs/premath/draft/DOCTRINE-SITE.md` (§3.5).
- Tightened constructor-first checker authority language in
  `specs/premath/draft/PREMATH-COHERENCE.md`.
- Added explicit constructor binding requirement in
  `specs/premath/draft/UNIFICATION-DOCTRINE.md` (§12.7).
- Added overlay row-shape guidance in `specs/premath/raw/TORSOR-EXT.md` (§6.1).
- Synced index/traceability/docs:
  - `specs/premath/draft/SPEC-INDEX.md`
  - `specs/premath/draft/SPEC-TRACEABILITY.md`
  - `docs/design/ARCHITECTURE-MAP.md`
- Added lifecycle entry:
  `specs/process/decision-log.md` (Decision 0129).
- Added constructor witness emission/authority path in
  `crates/premath-coherence/src/lib.rs` and validated via
  `cargo test -p premath-coherence` and `mise run coherence-check`.
- Aligned world-route parity fixtures with current constructor requirements in
  `tools/conformance/test_runtime_orchestration.py`
  (fiber lifecycle + `issue.claim_next` route bindings).
- Closed `bd-250` with runner changes:
  - `tools/conformance/run_world_core_vectors.py` now core-authority only,
  - `tools/conformance/run_frontend_parity_vectors.py` now derives kernel authority from core `site-resolve`,
  - updated `tests/conformance/fixtures/frontend-parity/adversarial/failure_class_drift_reject/expect.json`.
- Closed `bd-251` with sentinel coverage and pipeline wiring:
  - added `tools/conformance/test_world_core_vectors.py`,
  - added `tools/conformance/test_frontend_parity_vectors.py`,
  - wired both into `.mise.toml` task `ci-pipeline-test`.
- Closed `bd-252` after fixture-lane cleanup/docs authority alignment:
  - removed dead semantic-helper paths in runner code,
  - clarified replay-only authority posture in fixture/conformance docs,
  - verified via coherence-contract suite + full conformance + docs-coherence.
- Closed coordinator `bd-243`; execution moved to `bd-244` chain.
- Verified constructor-anchored vector suites under `bd-244`:
  - runtime-orchestration (14 vectors, invariance=2),
  - frontend-parity (6 vectors, invariance=1),
  - world-core (13 vectors, invariance=2),
  - `mise run coherence-check` accepted.
- Closed `bd-245` with newcomer-facing constructor-first doc pass:
  - clarified fixture parity wording in `README.md` + `ARCHITECTURE-MAP.md`,
  - added explicit onboarding order to `docs/design/README.md`,
  - validated `docs-coherence-check` + `traceability-check`.
- `bd-246` cleanup completed:
  - removed orphan runtime-orchestration vectors not listed in manifest,
  - added manifest-closure sentinels in `tools/conformance/test_run_fixture_suites.py`,
  - validated `ci-pipeline-test` and issue-graph compactness after dependency cleanup.
- Closed `bd-246` and closed epic `bd-238` after epic-level verification:
  - `mise run coherence-check`,
  - `mise run doctrine-check`,
  - `mise run conformance-run`,
  - `mise run docs-coherence-check`,
  - `python3 tools/ci/check_issue_graph.py`.
- Closed `bd-253` inventory pass:
  - added `docs/design/generated/HOST-ACTION-OPID-CLOSURE-INVENTORY.md`
    with deterministic mapping for 12 unmapped host actions.
- Closed `bd-254` operationId binding pass:
  - mapped all required host actions to `operationId` (missing count -> 0),
  - added 12 operation rows in doctrine operation-registry source
    (`specs/premath/site-packages/.../SITE-PACKAGE.json`),
  - regenerated doctrine artifacts + inventory outputs and updated
    `docs/design/STEEL-REPL-DESCENT-CONTROL.md` §5.1 parity table.
- Closed `bd-255` route-qualification pass:
  - bound required/harness route-qualified operations in doctrine-site package
    (`op/ci.verify_required_decision`, `op/harness.feature_read`,
    `op/harness.feature_check`, `op/harness.feature_next`,
    `op/harness.trajectory_query`) with explicit `routeEligibility`,
  - regenerated doctrine-site artifacts + inventory projections,
  - validated by `python3 tools/conformance/run_world_core_vectors.py`,
    `python3 tools/conformance/run_runtime_orchestration_vectors.py`,
    `mise run conformance-run`, `mise run doctrine-check`,
    `mise run docs-coherence-check`, and
    `python3 tools/ci/check_issue_graph.py`.
- Closed `bd-256` transport route-contract + doctrine-inf consolidation pass:
  - added core CLI command `premath doctrine-inf-check` for deterministic
    doctrine boundary/governance/route-consolidation evaluation,
  - demoted `tools/conformance/run_doctrine_inf_vectors.py` to fixture
    orchestration + expect comparison via core CLI output,
  - added doctrine-inf route-consolidation vectors (golden + adversarial) and
    wired traceability/doc references to kernel-backed route closure semantics.
- Closed `bd-257` kernel admission closure for route-bound frontend host actions:
  - route-bound calls now run kernel `site-resolve` preflight using contract
    operation binding + expected route family before dispatch,
  - frontends fail-closed on unbound route admission and on resolver witness
  drift between preflight binding and transport-dispatch witness,
  - added adversarial CLI coverage for route-unbound and binding-mismatch
  rejection paths, with full conformance gate verification.
- Closed `bd-258` parallel harness topology proof:
  - verified deterministic lease/claim contention behavior and dependency-safe
    dispatch invariants through harness multithread tests,
  - removed stale closed-block edges from active graph to keep compactness
    fail-closed and queue projections coherent.
- Closed `bd-259` scheme/rhai frontend parity closure:
  - validated parity vectors remain accepted over one shared kernel route
 - Closed evaluator UX hardening chain (`bd-261`, `bd-263`, `bd-265`, `bd-264`, `bd-269`) and epic `bd-260`:
   - fail-closed evaluator reject exits (`scheme-eval` and `rhai-eval` shared path),
   - actionable non-JSON diagnostics (failure class + stage + action + diagnostic + json hint),
   - aligned Scheme/Rhai metadata precedence (`call > CLI > program` scalars; capability union/dedupe),
   - added canonical scaffold command `premath evaluator-scaffold` with runnable outputs,
   - added `premath init --json` deterministic output-mode parity.
 - Verification closure for UX epic:
   - `cargo test -p premath-cli` (pass in isolated rerun),
   - `mise run conformance-run` (pass),
   - `mise run docs-coherence-check` (pass).
    admission path,
  - confirmed docs/conformance closures with no frontend semantic-authority
    fork.

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
- `rg -n '"id":"bd-23[8-9]"|"id":"bd-24[0-9]"|"id":"bd-25[0-5]"' .premath/issues.jsonl`

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
