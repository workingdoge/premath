# Design Docs (Runtime + Control)

These docs are implementation-facing and non-normative.

Authority rule:

- normative contracts live under `specs/`,
- `docs/design/` explains implementation shape, boundaries, and operational
  composition.

## Lanes

### Tusk runtime (inside one world)

- `TUSK-ARCHITECTURE.md`: recursive unit contract and runtime surfaces.
- `TUSK-DOMAIN-ADAPTERS.md`: domain adapter model over generic substrate.
- `TUSK-DESCENT-PACKS.md`: local/overlap/glue package shape.
- `TUSK-REFINEMENT.md`: refinement taxonomy and activation rules.
- `TUSK-IDENTITY.md`: run identity and deterministic bindings.
- `TUSK-WITNESSING.md`: Gate vs transport witnessing split.
- `TUSK-HARNESS-CONTRACT.md`: long-running harness hooks (`boot/step/stop`),
  durability boundaries, and trajectory/evidence mapping.
- `TUSK-HARNESS-RETRY-POLICY.md`: canonical retry classification/escalation
  table for harness pipeline wrappers.
- `TUSK-HARNESS-SESSION.md`: compact handoff artifact + bootstrap payload
  contract for fresh-context restartability.
- `TUSK-HARNESS-FEATURE-LEDGER.md`: typed per-feature progress ledger, closure
  checks, and deterministic next-feature selection.
- `TUSK-HARNESS-TRAJECTORY.md`: append-only step trajectory rows with
  witness-linked deterministic query projections.

### Squeak/SigPi transport + runtime placement (between worlds)

- `SQUEAK-DESIGN.md`: canonical design guidance for transport/placement.
- `TUSK-SIGPI.md`: compatibility alias path that points to `SQUEAK-DESIGN.md`.

### Control/CI and architecture composition

- `ARCHITECTURE-MAP.md`: doctrine-to-operation map + active execution order.
- `CI-CLOSURE.md`: closure gate and change-projected entry minimization.
- `CI-PROVIDER-BINDINGS.md`: provider bindings to canonical CI contract.
- `LIFECYCLE-COHERENCE-FLOWS.md`: operator flow for schema lifecycle policy and
  coherence gate-chain enforcement.
- `ISSUE-GRAPH-CORE-CHECKING.md`: issue-memory authority split and plan to move
  issue-graph invariants from CI wrappers into `premath-bd` core.
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
