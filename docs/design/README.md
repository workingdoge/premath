# Design Docs

These docs are implementation-facing and non-normative.

Authority rule:

- normative contracts live under `specs/`,
- `docs/design/` explains implementation shape, boundaries, and operational
  composition.

## Lanes

### Transport

- `transport/README.md`: lane entrypoint.
- `transport/SQUEAK-DESIGN.md`: canonical design guidance for
  transport/placement.

### Control Plane

- `control-plane/README.md`: lane entrypoint.
- `control-plane/ARCHITECTURE-MAP.md`: doctrine-to-operation map and active
  execution order.
- `control-plane/EV-COHERENCE-OVERVIEW.md`: compact evidence-plane status
  snapshot.
- `control-plane/DEVELOPMENT-META-LOOP.md`: canonical development workflow
  contract.
- `control-plane/LIFECYCLE-COHERENCE-FLOWS.md`: schema lifecycle and gate-chain
  flow.
- `control-plane/ATLAS.md`: site-of-sites staging note for cross-repo covers,
  seams, authority surfaces, projections, and descent conditions.
- `control-plane/TOPOLOGY-V2.md`: shape-first placement model for authority
  objects, projections, repo hosts, and simplex-native tracker routing.
- `control-plane/MEMORY-LANES-CONTRACT.md`: work-memory lane split and
  write-discipline rules.
- `control-plane/CONTROL-PLANE-THREAT-MODEL.md`: threat and hardening matrix.

### Operations

- `operations/README.md`: lane entrypoint.

### Shared

- `GLOSSARY.md`: shared terms across runtime/control docs.

## Relationship To Specs

Design docs do not replace normative specs.

- Semantic authority: `specs/premath/draft/PREMATH-KERNEL.md`,
  `specs/premath/draft/OBLIGATION-DISCHARGE.md`, `specs/premath/draft/GATE.md`.
- Transport normative candidates:
  - `specs/premath/raw/SQUEAK-CORE.md`
  - `specs/premath/raw/SQUEAK-SITE.md`

## Live Roadmap Source

Design docs may reference historical issue IDs, but active execution order is
owned outside Premath by Tusk or the downstream operator tracker.
