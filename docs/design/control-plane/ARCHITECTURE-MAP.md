# Architecture Map

Premath is the checker surface, not the runtime stack.

Current split:

- Premath Core: kernel law, Gate law, obligation discharge, witness IDs.
- Premath checker profiles: instruction typing/proposal checking, required
  projection/witness verification, coherence, traceability, and drift checks.
- Tusk/downstream runtime: tracker mutation, hook orchestration, provider workflow,
  gate execution, instruction execution, and runtime artifact publication.
- Kurma: executable carriage/realization for stable upstream method surfaces.

Operational surfaces retained in Premath:

- `premath coherence-check`
- `premath drift-budget-check`
- `premath traceability-check`
- `premath command-surface-check`
- `premath repo-hygiene-check`
- native checker commands under `premath-cli`

Removed from Premath ownership:

- provider workflow wrappers
- instruction execution runners
- required-gate execution runners
- hook-manager configuration
- tracker runtime/mutation commands

Boundary rule:

Premath may accept normalized runtime evidence and decide whether it is
admissible. It should not own the workflow that creates that evidence.
