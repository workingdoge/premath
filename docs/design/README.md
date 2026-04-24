# Design Docs

These docs are implementation-facing and non-normative.

Authority rule:

- normative contracts live under `specs/`,
- `docs/design/` explains implementation shape, boundaries, and operational
  composition.

## Lanes

### Runtime

Promoted harness contract surfaces now live in:

- `specs/premath/draft/HARNESS-RUNTIME.md`
- `specs/premath/draft/HARNESS-RETRY-ESCALATION.md`

Design docs below remain implementation-facing runbooks.

- `runtime/README.md`: lane entrypoint.
- `runtime/TUSK-ARCHITECTURE.md`: recursive unit contract and runtime surfaces.
- `runtime/TUSK-DOMAIN-ADAPTERS.md`: domain adapter model over generic
  substrate.
- `runtime/TUSK-DESCENT-PACKS.md`: local/overlap/glue package shape.
- `runtime/TUSK-REFINEMENT.md`: refinement taxonomy and activation rules.
- `runtime/TUSK-IDENTITY.md`: run identity and deterministic bindings.
- `runtime/TUSK-WITNESSING.md`: Gate vs transport witnessing split.
- `runtime/TUSK-HARNESS-CONTRACT.md`: long-running harness hooks
  (`boot/step/stop`), durability boundaries, trajectory/evidence mapping, and
  consolidated runbooks.
- `runtime/TUSK-HARNESS-RETRY-POLICY.md`: retry classification/escalation table
  for harness pipeline wrappers.
- `runtime/TUSK-HARNESS-FEATURE-LEDGER.md`: per-feature progress ledger and
  deterministic next-feature selection.
- `runtime/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`: deterministic
  coordinator/worker loop over `N` worktrees.

### Transport

- `transport/README.md`: lane entrypoint.
- `transport/SQUEAK-DESIGN.md`: canonical design guidance for
  transport/placement.

### Control Plane

- `control-plane/README.md`: lane entrypoint.
- `control-plane/ARCHITECTURE-MAP.md`: doctrine-to-operation map and active
  execution order.
- `control-plane/CI-CLOSURE.md`: closure gate and change-projected entry
  minimization.
- `control-plane/CI-PROVIDER-BINDINGS.md`: provider bindings to canonical CI
  contract.
- `control-plane/EV-COHERENCE-OVERVIEW.md`: compact evidence-plane status
  snapshot.
- `control-plane/DEVELOPMENT-META-LOOP.md`: canonical development workflow
  contract.
- `control-plane/MULTITHREAD-LANE-SITE-ADJOINTS.md`: concurrent-worker and
  site-adjoint contract notes.
- `control-plane/LIFECYCLE-COHERENCE-FLOWS.md`: schema lifecycle and gate-chain
  flow.
- `control-plane/ISSUE-GRAPH-CORE-CHECKING.md`: issue-memory authority split
  and core checking plan.
- `control-plane/MEMORY-LANES-CONTRACT.md`: work-memory lane split and
  write-discipline rules.
- `control-plane/TOOL-CALLING-HARNESS-TYPESTATE.md`: tool-calling typestate
  design notes.
- `control-plane/STEEL-REPL-DESCENT-CONTROL.md`: Scheme/Steel REPL control
  surface.
- `control-plane/CONTROL-PLANE-THREAT-MODEL.md`: threat and hardening matrix.
- `control-plane/HIGHER-ORDER-CI-CD.md`: coding-environment control-loop
  framing.

### Operations

- `operations/README.md`: lane entrypoint.
- `operations/RALPH-PLAYBOOK-PREMATH.md`: Ralph execution-loop adaptation under
  Premath issue and witness authority.

### Shared

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
