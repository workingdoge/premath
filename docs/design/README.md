# Design Docs (Runtime + Control)

These docs are implementation-facing and non-normative.

Authority rule:

- normative contracts live under `specs/`,
- `docs/design/` explains implementation shape, boundaries, and operational
  composition.

## Lanes

### Tusk runtime (inside one world)

- Promoted harness contract surfaces now live in:
  - `specs/premath/draft/HARNESS-RUNTIME.md`
  - `specs/premath/draft/HARNESS-RETRY-ESCALATION.md`
  Design docs below remain implementation-facing runbooks.

- `TUSK-ARCHITECTURE.md`: recursive unit contract and runtime surfaces.
- `TUSK-DOMAIN-ADAPTERS.md`: domain adapter model over generic substrate.
- `TUSK-DESCENT-PACKS.md`: local/overlap/glue package shape.
- `TUSK-REFINEMENT.md`: refinement taxonomy and activation rules.
- `TUSK-IDENTITY.md`: run identity and deterministic bindings.
- `TUSK-WITNESSING.md`: Gate vs transport witnessing split.
- `TUSK-HARNESS-CONTRACT.md`: long-running harness hooks (`boot/step/stop`),
  durability boundaries, trajectory/evidence mapping, and consolidated
  runbooks for session artifact (§11), trajectory rows (§12), and KPI benchmark
  (§13).
- `TUSK-HARNESS-RETRY-POLICY.md`: canonical retry classification/escalation
  table for harness pipeline wrappers.
- `TUSK-HARNESS-FEATURE-LEDGER.md`: typed per-feature progress ledger, closure
  checks, and deterministic next-feature selection.
- `TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`: deterministic coordinator/worker loop
  over `N` worktrees (`issue_ready -> claim -> work -> verify -> release/update`)
  with explicit heartbeat/recovery guidance.

### Squeak/SigPi transport + runtime placement (between worlds)

- `SQUEAK-DESIGN.md`: canonical design guidance for transport/placement.

### Control/CI and architecture composition

- `ARCHITECTURE-MAP.md`: doctrine-to-operation map + active execution order.
- `CI-CLOSURE.md`: closure gate and change-projected entry minimization.
- `CI-PROVIDER-BINDINGS.md`: provider bindings to canonical CI contract.
- `EV-COHERENCE-OVERVIEW.md`: compact status snapshot for Unified Evidence
  Plane contracts, coherence boundaries, issue posture, Stage 1 checklist, and
  Stage 3 execution runbook.
- `DEVELOPMENT-META-LOOP.md`: canonical development workflow contract
  (architecture-first ordering, multithread worker loop, and lane/gate
  discipline).
- `MULTITHREAD-LANE-SITE-ADJOINTS.md`: canonical concurrent-worker contract
  aligned with lane ownership, site refinement/covers, and optional SigPi/Squeak
  capability overlays.
- `LIFECYCLE-COHERENCE-FLOWS.md`: operator flow for schema lifecycle policy and
  coherence gate-chain enforcement.
- `ISSUE-GRAPH-CORE-CHECKING.md`: issue-memory authority split and plan to move
  issue-graph invariants from CI wrappers into `premath-bd` core.
- `MEMORY-LANES-CONTRACT.md`: canonical work-memory lane split (issues,
  operations, doctrine/decision) and write-discipline rules.
- `TOOL-CALLING-HARNESS-TYPESTATE.md`: typed tool-calling harness turn contract
  and fail-closed runtime-gate design notes.
- `RALPH-PLAYBOOK-PREMATH.md`: Ralph playbook execution-loop adaptation under
  premath issue/witness authority and fail-closed mutation gates.
- `STEEL-REPL-DESCENT-CONTROL.md`: Scheme/Steel REPL control-surface design,
  descent/sheaf execution shape, host API boundaries, and harness integration.
- `CONTROL-PLANE-THREAT-MODEL.md`: threat/hardening matrix for control-plane
  mutation, witness, and projection surfaces.
- `HIGHER-ORDER-CI-CD.md`: control-loop framing inside coding environment.
- `GLOSSARY.md`: shared terms across runtime/control docs.

## Relationship To Specs

Design docs do not replace normative specs.

- Semantic authority: `specs/premath/draft/PREMATH-KERNEL.md`,
  `specs/premath/draft/GATE.md`, `specs/premath/draft/BIDIR-DESCENT.md`.
- Runtime/transport normative candidates:
  - `specs/premath/raw/TUSK-CORE.md`
  - `specs/premath/raw/SQUEAK-CORE.md`
  - `specs/premath/raw/SQUEAK-SITE.md`
  - `specs/premath/raw/CI-TOPOS.md`

## Live Roadmap Source

Design docs may reference historical issue IDs, but active execution order is
always read from issue memory:

- `.premath/issues.jsonl`
- `premath issue ready`
- `premath issue list`
