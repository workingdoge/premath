# Tusk Design Docs

These documents are implementation-facing and non-normative.

Tusk is the recursive unit model for building Premath-first systems where:

- the kernel provides semantic laws,
- a generic memory substrate provides replayable state,
- domain adapters (for example Beads, accounting) project domain meaning,
- control policies orchestrate execution without mutating projections directly.

Boundary:

- Tusk realizes execution inside a Premath world.
- SigPi (separate layer) handles composition/transport between worlds.

## Documents

- `TUSK-ARCHITECTURE.md`: Premath-first architecture and recursive unit contract.
- `TUSK-DOMAIN-ADAPTERS.md`: domain adapter model over the generic substrate.
- `TUSK-DESCENT-PACKS.md`: presheaf-like local/overlap/glue package shape across domains.
- `TUSK-REFINEMENT.md`: refinement taxonomy, witnesses, and activation rules.
- `TUSK-IDENTITY.md`: run identity, deterministic IDs, and semantic policy bindings.
- `TUSK-WITNESSING.md`: Gate vs transport witnessing model and failure mappings.
- `TUSK-SIGPI.md`: inter-world transport/composition boundary and contracts.
- `GLOSSARY.md`: shared terms for architecture and implementation docs.
- `ARCHITECTURE-MAP.md`: one-page doctrine-to-operation architecture map.
- `CI-CLOSURE.md`: CI/pre-commit closure gate and change-projected entry minimization.
- `HIGHER-ORDER-CI-CD.md`: DevOps/control-loop framing for CI/CD inside the coding environment.

## Relationship To Specs

These docs do not replace the normative raw specs.

- Kernel laws remain in `specs/premath/draft/PREMATH-KERNEL.md` and `specs/premath/draft/GATE.md`.
- Operational obligation framing remains in `specs/premath/draft/BIDIR-DESCENT.md`.
- Raw operational spec candidates live in:
  - `specs/premath/raw/TUSK-CORE.md`
  - `specs/premath/raw/SQUEAK-CORE.md`
  - `specs/premath/raw/SQUEAK-SITE.md`
  - `specs/premath/raw/CI-TOPOS.md`
