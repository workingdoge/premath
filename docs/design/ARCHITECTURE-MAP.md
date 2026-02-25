# Architecture Map (One Page)

Status: draft
Scope: design-level, non-normative

## 0. Frontend Route (Newcomer View)

One canonical control path:

```text
frontend adapter -> host action -> transport envelope
-> site resolver decision (INF -> SITE -> WORLD)
-> world-route kernel check -> mutation/evidence artifacts
```

Adapter set:
- Steel/Scheme (`premath scheme-eval`)
- Rhai (`premath rhai-eval`)
- MCP (`premath mcp-serve`)
- optional BEAM/Rustler (`premath-transport` `dispatch(request_json)`)

Boundary command map:
- resolver boundary: `premath site-resolve`
- transport boundary: `premath transport-dispatch`, `premath transport-check`
- world-route boundary: `premath world-registry-check`, `premath world-gate-check`
- mutation/evidence boundary: `premath issue ...`, `premath instruction-*`,
  `premath required-*`

Generated newcomer index:

- `docs/design/generated/DOCTRINE-SITE-INVENTORY.md`
  (site -> operations -> route families -> world bindings -> command surfaces).

## 0.1 World Descent Authority Contract (WDAC-1)

This is the architecture-level contract for world descent. It is execution
policy, not a second semantic authority.

Lane ownership:

| Lane | Authority Surface | Ownership |
| --- | --- | --- |
| semantic lane | `PREMATH-KERNEL`, `GATE`, `BIDIR-DESCENT` | admissibility law and gate verdicts |
| constructor lane | `WORLD-REGISTRY`, `SITE-RESOLVE` | route/world/morphism selection and constructor binding |
| check-role lane | `PREMATH-COHERENCE` | parity/discharge over declared contract surfaces |
| wrapper lane | `tools/ci/*`, `tools/conformance/*`, frontend adapters | transport, orchestration, replay/parity only |

No-parallel-authority constraints:

1. Wrapper lanes MUST NOT synthesize semantic accept/reject classes for route
   admissibility.
2. Wrapper lanes MUST consume core route decisions from `premath site-resolve`
   and `premath world-registry-check` (or equivalent kernel-backed API).
3. Missing/ambiguous/unbound constructor or route material MUST fail closed
   through canonical authority classes; wrappers may only pass through those
   classes.

Execution-order contract (for non-trivial world-descent epics):

1. architecture contract,
2. spec/index + doctrine glue,
3. control-plane parity wiring,
4. implementation,
5. conformance vectors,
6. docs/traceability closure.

Spec chain anchors:

- `specs/premath/draft/SPEC-INDEX.md` §0.4 (world self-hosting boundary map),
- `specs/premath/draft/WORLD-REGISTRY.md` (constructor + route/world contract),
- `specs/premath/draft/SITE-RESOLVE.md` (deterministic resolver contract),
- `specs/premath/draft/PREMATH-COHERENCE.md` (check-role authority),
- `specs/premath/draft/UNIFICATION-DOCTRINE.md` §12 (descent operationalization).

## 1. Layer Stack

`Doctrine` (what must be preserved):
- `specs/premath/draft/DOCTRINE-INF.md`
- `specs/premath/draft/DOCTRINE-SITE.md`
- `docs/design/generated/DOCTRINE-SITE-INVENTORY.md` (generated navigation index)
- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`
- `specs/premath/draft/LLM-PROPOSAL-CHECKING.md`
  - governance-flywheel doctrine profile: `DOCTRINE-INF` §9

`Kernel` (semantic authority):
- `specs/premath/draft/PREMATH-KERNEL.md`
- `specs/premath/draft/GATE.md`
- `specs/premath/draft/BIDIR-DESCENT.md`

`Worldization` (route/world/morphism binding contract):
- `specs/premath/draft/WORLD-REGISTRY.md`
- `specs/premath/raw/WORLD-PROFILES-CONTROL.md`
- `specs/premath/raw/TORSOR-EXT.md` (overlay interpretation only; non-authority)

### 1.2 K0 world-kernel contract (why this is Premath, not parallel architecture)

Worldization is a concrete Premath instantiation, not a second kernel:

- Premath laws stay in `PREMATH-KERNEL`/`GATE`/`BIDIR-DESCENT`,
- world profiles map control-plane domains into those laws,
- route binding checks execute through one kernel-backed world semantics path.

Crate placement target:

| Responsibility | Primary lane |
| --- | --- |
| World row/morphism/binding semantics | `crates/premath-kernel` |
| Checker/CLI consumption of world semantics | `crates/premath-coherence`, `crates/premath-cli` |
| Transport/orchestration wrappers only | `tools/ci/*`, `tools/conformance/*` |

Boundary rule:

- wrappers may format, route, or aggregate results, but MUST NOT become
  independent world admissibility authorities.

### 1.3 Canonical world-semantics placement map

| Concern | Canonical lane | Notes |
| --- | --- | --- |
| world-route semantics + failure classes | `crates/premath-kernel/src/world_registry.rs` | single semantic authority for route/world/morphism admissibility |
| executable world check surface | `crates/premath-cli/src/commands/world_registry_check.rs` | `premath world-registry-check` derives required world-route bindings from `CONTROL-PLANE-CONTRACT` |
| semantic conformance lane | `tools/conformance/run_world_core_vectors.py`, `tests/conformance/fixtures/world-core/` | validates core world semantics and fixture parity against core outputs |
| adapter/runtime route parity lane | `premath runtime-orchestration-check`, `tools/conformance/run_runtime_orchestration_vectors.py`, `tests/conformance/fixtures/runtime-orchestration/` | semantic authority is `runtime-orchestration-check`; vectors validate parity/invariance without adding wrapper-only semantic lanes |

### 1.4 Constructor-First Coherence Contract

Coherence/control-plane implementation follows one constructor-first rule:

- one constructor-authority lane in core checker/kernel semantics,
- wrappers (`tools/ci/*`) are orchestration adapters only,
- fixture runners (`tools/conformance/*` + `tests/conformance/fixtures/*`) are
  replay/parity surfaces only.

Forbidden:

- wrapper-local semantic verdict logic,
- fixture-runner semantic reconstruction that diverges from core constructor
  outputs,
- alternate route/world admissibility lanes outside kernel-backed surfaces.

`Runtime` (execution inside/between worlds):
- `specs/premath/raw/TUSK-CORE.md`
- `specs/premath/raw/SQUEAK-CORE.md`
- `specs/premath/raw/SQUEAK-SITE.md`
- `specs/premath/raw/FIBER-CONCURRENCY.md`
- harness overlay (design): `docs/design/TUSK-HARNESS-CONTRACT.md`

Runtime composition route (required boundary shape):

- `Harness(step)` -> `Squeak transport/placement` -> `destination Tusk/Gate`
  -> `Harness projection artifacts`.
- Harness/Squeak remain operational routing surfaces; admissibility remains
  destination checker/Gate-owned.
- Structured concurrency profile for agent/runtime orchestration is documented
  in `docs/design/FIBER-CONCURRENCY.md` and currently binds transport lifecycle
  actions through `route.fiber.lifecycle -> world.fiber.v1`.

### 1.1 Phase-3 target vs transition contract

Target state (`bd-287`/phase 3):

- one premath-native control surface for multi-step worker execution
  (`scheme_eval`-style evaluator over capability-scoped host functions),
- semantic authority remains kernel/Gate-only,
- wrappers remain transport/compatibility adapters only.

Transition state (current):

- Python/CLI wrappers still orchestrate parts of required/instruction flows,
- wrapper logic must stay adapter-only and must not become a second authority
  lane,
- migration is accepted only when witness lineage, failure classes, and policy
  digests stay deterministic.

REPL/Steel integration rule (agent-facing control, not authority):

- REPL program execution may plan and sequence host calls, but host calls are
  the only mutation path.
- Host API remains capability-scoped and instruction-linked for mutations.
- REPL runtime must default deny direct shell/network effects.
- Each host call must emit deterministic effect rows with at least:
  `action`, `argsDigest`, `resultClass`, `witnessRefs[]`, `policyDigest`
  (when mutation-capable).

Design companion:

- `docs/design/STEEL-REPL-DESCENT-CONTROL.md`

Host API v0 families (mapped to current premath surfaces):

- issue/dependency mutation: `issue.claim|update|discover`,
  `dep.add|remove|replace` -> `premath issue ...`, `premath dep ...`
- observation/query: `issue.ready|blocked|list|check`, `observe.latest|
  needs_attention|instruction|projection`, `dep.diagnostics`
- doctrine/control: `instruction.check|run`, `coherence.check`, `required.*`
- harness durability: `harness.session.read|write|bootstrap`,
  `harness.trajectory.append|query`, `harness.feature.*`

`CI/Control` (one layer, two roles):
- `specs/premath/raw/PREMATH-CI.md`
- `specs/premath/raw/CI-TOPOS.md`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `specs/premath/draft/COHERENCE-CONTRACT.json`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md` (lane split + Unified Evidence Plane §10)

Role split inside CI/Control:
- check role: `PREMATH-COHERENCE` (`premath coherence-check`)
- execute/attest role: `PREMATH-CI` + `CI-TOPOS` (`pipeline_*`, `run_*`, verify/decide)

`Operational surfaces` (scripts/tasks):
- `tools/ci/pipeline_required.py`
- `tools/ci/pipeline_instruction.py`
- `tools/ci/run_required_checks.py`
- `tools/ci/verify_required_witness.py`
- `tools/ci/run_instruction.py` / `tools/ci/run_instruction.sh`
- `tools/ci/run_gate.sh`
- `tools/conformance/check_doctrine_site.py`
- `premath runtime-orchestration-check` (canonical authority)
- `tools/conformance/check_doctrine_mcp_parity.py`
- `tools/conformance/run_doctrine_inf_vectors.py`
- `premath coherence-check` (`crates/premath-coherence` + `premath-cli`)
- `hk.pkl`, `.mise.toml`

## 2. Doctrine to Operation Path

```text
DOCTRINE-INF
  -> DOCTRINE-SITE (nodes/covers/edges)
  -> LLM-INSTRUCTION-DOCTRINE
  -> LLM-PROPOSAL-CHECKING
  -> Control Plane
     -> check role: PREMATH-COHERENCE / COHERENCE-CONTRACT
        -> premath coherence-check
     -> execute/attest role: PREMATH-CI / CI-TOPOS
        -> tools/ci/pipeline_required.py / tools/ci/pipeline_instruction.py
        -> tools/ci/run_required_checks.py
        -> tools/ci/verify_required_witness.py
        -> tools/ci/run_gate.sh
  -> tools/conformance/check_doctrine_site.py /
     runtime-orchestration-check /
     check_doctrine_mcp_parity.py / run_doctrine_inf_vectors.py
  -> tools/conformance/generate_doctrine_site_inventory.py
     -> docs/design/generated/DOCTRINE-SITE-INVENTORY.md
  -> hk/mise tasks (.mise baseline + ci-required-attested)
  -> CIWitness artifacts
  -> conformance + doctrine-site checks
```

Authority rule:
- semantic admissibility comes from kernel/gate contracts, not from runners or hooks.
- coherence and CI are control-plane roles, not semantic authority layers.
- control-plane artifact families should factor through one attested evidence
  surface (`Ev`) per `UNIFICATION-DOCTRINE` §10.
- runners/profiles (`local`, `external`, infra bindings) change execution substrate only.
- torsor/extension overlays remain proposal/evidence interpretation surfaces and
  must not become route-bound authority targets.

## 3. Instruction Runtime Loop

```text
InstructionEnvelope
  -> classify: typed(kind) | unknown(reason)
  -> apply typingPolicy.allowUnknown
  -> project requested checks
  -> execute checks via run_gate
  -> emit CI witness with:
     instructionDigest + instructionClassification + typingPolicy
```

Deterministic rejection path:
- `unknown(reason)` with `allowUnknown=false` rejects before check execution with
  `instruction_unknown_unroutable`.

## 4. Work-Memory Authority Loop

```text
WorkMemory (canonical JSONL substrate)
  -> InstructionMorphisms (typed, policy-bound mutation requests)
  -> Witnesses (instruction-bound + optional JJ snapshot linkage)
  -> QueryProjection (rebuildable read/index layer; non-authoritative)
```

Repository default profile:
- canonical memory: `.premath/issues.jsonl` (`premath-bd`)
- MCP mutation policy: `instruction-linked`
  - mutation authorization is policy-scoped and capability-scoped from accepted
    instruction witnesses (`policyDigest` + `capabilityClaims`)
- query backend default: `jsonl` (with optional `surreal` projection mode)

## 5. Refinement Loop

```text
world_registry_check(route.issue_claim_lease -> world.lease.v1)
  -> issue_ready -> issue_blocked -> issue_claim -> instruction_run -> witness
  -> issue_lease_renew (long task) or issue_lease_release (handoff)
  -> issue_discover (when new work is found) -> issue_ready
```

Loop intent:
- keep sessions short and restartable,
- prevent lost/discarded discovered work,
- keep mutation authority instruction-mediated with auditable witnesses,
- keep BEAM coordinator lease orchestration world-bound through canonical
  `premath world-registry-check` semantics (no wrapper-local world authority).

## 6. Conformance Closure

Baseline gate (`mise run baseline`) enforces:
- setup/lint/build/test/toy suites,
- conformance + traceability + coherence-check + docs-coherence + doctrine closure,
- doctrine closure includes doctrine-site roundtrip/reachability plus MCP
  doctrine-operation parity + runtime-route parity
  (`check_doctrine_site.py`, `runtime-orchestration-check`,
  `check_doctrine_mcp_parity.py`),
- world semantic route/binding closure runs in dedicated `world-core` vectors
  (`run_world_core_vectors.py`) with fixture parity against core outputs,
- CI/control-plane wiring, pipeline, observation, instruction, and drift-budget checks,
- executable fixture-suite closure (`mise run conformance-run`).

Operational source of truth for baseline composition is `.mise.toml`
(`[tasks.baseline]`).

Projected required gate (`mise run ci-required`) enforces:
- deterministic `Delta -> requiredChecks` projection,
- execution of projected checks only,
- CI closure witness emission (`artifacts/ciwitness/proj1_*.json`).

Authoritative verification (`mise run ci-verify-required`) enforces:
- projection/witness digest consistency,
- required/executed check-set consistency,
- verdict/failure-class consistency with check results.

Control-plane schema lifecycle discipline:
- coherence gate-chain parity enforces `schemaLifecycle` resolution over
  contract/projection/witness kind families.
- aliases are epoch-bounded under `activeEpoch`; expired aliases reject
  deterministically as
  `coherence.gate_chain_parity.schema_lifecycle_invalid`.
- operator runbook for lifecycle/coherence flow:
  `docs/design/LIFECYCLE-COHERENCE-FLOWS.md`.

Instruction doctrine is executable via:
- `capabilities.instruction_typing`
- `capabilities.ci_witnesses`
- `draft/LLM-PROPOSAL-CHECKING` proposal ingest/discharge path
- `capabilities.change_morphisms`

## 7. Unification Execution Order (Current)

Current sequencing is tracked in `.premath/issues.jsonl` with active unification
work resolved dynamically via `premath issue ready`.

Canonical ordering for active work is architecture-first:

1. lane/site/adjoint architecture slice,
2. spec-index + doctrine-site glue slice,
3. control-plane typed-contract/checker parity slice,
4. implementation slice,
5. conformance/closure slice.

Operational companion:
- `docs/design/MULTITHREAD-LANE-SITE-ADJOINTS.md`
- `docs/design/DEVELOPMENT-META-LOOP.md`
- `docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md`

Operational rule:
- treat this section as a pointer only; use `premath issue ready` /
  `premath issue list` for authoritative ordering.
- for a compact status snapshot of `Ev`/coherence/issue posture, see
  `docs/design/EV-COHERENCE-OVERVIEW.md`.

Execution notes:
- keep semantic authority in kernel/gate paths; do not move admissibility into
  checker/profile/CI wrappers.
- keep cross-lane pullback/base-change claims routed through typed
  span/square witnesses.
- keep capability composition explicit (`change_morphisms`,
  `adjoints_sites`, `squeak_site`) with no implicit authority escalation.

## 8. Grothendieck Operationalization (Applied)

Operational reading of current stack (normative anchor:
`draft/UNIFICATION-DOCTRINE` §12):

1. one semantic authority fibration (`p0 : E -> C`) plus one attested
   control-plane evidence family (`Ev : Ctx^op -> V`),
2. multithread worker decomposition is a cover on `Ctx`
   (`{Gamma_i -> Gamma}`), with each worker handling one refinement,
3. merge is a deterministic glue-or-obstruction step over overlap pullbacks,
4. all runtime/control outcomes are projections factoring through `Ev`,
5. checker/Gate discharge remains the only admissibility authority.

This keeps concurrency and operational acceleration as structured base-change
and descent, not as a second semantic path.

Operational stance:

- continuity is descent-first (bounded slices + typed handoffs + glue checks),
  not transcript-linear + compaction-first.
- compaction remains a bounded compatibility/fallback policy only.

## 9. External Crosswalk (Harness / Context / Ralph Loop)

This section maps recent external harness patterns to existing premath
surfaces and identifies residual implementation gaps.

Source anchors:

- `https://michaellivs.com/blog/agent-harness`
- `https://michaellivs.com/blog/context-engineering-open-call/`
- `https://michaellivs.com/blog/multi-agent-context-transfer/`
- `https://claytonfarr.github.io/ralph-playbook/`
- `https://ampcode.com/notes/how-to-build-an-agent`
- `https://ampcode.com/guides/context-management`

Direct mappings already present:

- injection points (`system`, `user`, `tool response`) map to per-turn typed
  call policy and reminder-bearing tool rows in
  `docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md`.
- "simple inner loop + explicit tool contract" maps to a deterministic
  `step` contract and typed closure gates in
  `specs/premath/draft/HARNESS-RUNTIME.md` §3.2.
- "conversation as event stream + views" maps to append-only harness trajectory
  plus deterministic projections (`latest|failed|retry-needed`) in
  `specs/premath/draft/HARNESS-RUNTIME.md`.
- context management operations (restore/edit/handoff) map to
  `harness-session` bootstrap/continuation plus typed context-scope/state-view
  policies in `docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md`.
- explicit stop and enforcement logic maps to `ToolUse -> JoinClosed ->
  MutationReady` fail-closed mutation gates in
  `specs/premath/draft/HARNESS-RUNTIME.md`.
- tool failures are modeled as typed runtime evidence (not free-form narration)
  and participate in closure/mutation admissibility checks.
- lossy multi-agent boundaries map to typed handoff contracts
  (`handoffContractDigest`, required artifacts, allowed targets, return path)
  and lease handoff refs (`lease://handoff/...`) in
  `docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md` +
  `specs/premath/draft/HARNESS-RUNTIME.md`.
- governance flywheel/profile gating maps to `DOCTRINE-INF` §9 +
  `CONFORMANCE` §2.4 + `CAPABILITY-REGISTRY.json` `profileOverlayClaims`, with
  drift/coherence/doctrine checks bound to the same claim surface.
- Ralph-style one-task/fresh-context/backpressure loops map to
  issue-authoritative worker loops (`issue_ready -> claim -> work -> verify ->
  update`) in `docs/design/DEVELOPMENT-META-LOOP.md`, with detailed adaptation
  notes in `docs/design/RALPH-PLAYBOOK-PREMATH.md`.

Residual integration status:

- decomposition guards for parallel fan-out ("loop vs split") are now covered by
  dedicated adversarial conformance vectors (`bd-232`).
- governance promotion/runtime wrapper enforcement is now wired end-to-end in
  CI wrapper paths (`pipeline_required.py`, `pipeline_instruction.py`) through
  canonical governance gate helpers (`bd-229`).
- this crosswalk currently has no open residual integration gaps; track new gaps
  in `.premath/issues.jsonl` and keep this section synchronized with issue state.

## 10. Active WIP Topology Ownership (Authority-Lane Grouped)

Purpose:

- keep one explicit map from dirty repository clusters to lane ownership and
  tracked issue scope,
- prevent "orphaned" WIP clusters from accumulating outside the active issue graph.

Canonical WIP snapshot location:

- `.premath/OPERATIONS.md` -> `Active WIP Topology Ownership Map (bd-280 snapshot)`

Active chain for current reconciliation:

- epic: `bd-279`
- lane slices: `bd-280` -> `bd-281` -> `bd-282` -> `bd-283` -> `bd-284` ->
  `bd-285` -> `bd-286`

Lane grouping used by the WIP snapshot:

- doctrine/decision lane: `specs/*` + `specs/process/*` (spec glue and
  lifecycle closure),
- control/checker lane: `tools/ci/*` + control policy surfaces,
- runtime implementation lane: `crates/*`,
- conformance lane: `tests/conformance/*` + vector/check runners,
- design/operations lane: `docs/design/*`, root operational docs,
  `.premath/*`, and issue-graph updates.

Boundary rule:

- cluster ownership in the WIP snapshot MUST point to active issue IDs in
  `.premath/issues.jsonl`,
- any dirty cluster without an owning active issue is a fail-closed planning
  error and MUST be resolved before commit slicing.
