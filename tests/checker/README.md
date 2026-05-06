# Checker Fixtures

This directory contains Premath checker fixtures. Premath no longer owns a
separate Python suite-runner surface.

Current status:

- `fixtures/interop-core/` is retained adapter/profile fixture material.
- `fixtures/gate/` is covered by `premath-kernel` tests and toy vectors.
- `fixtures/doctrine-inf/` is covered through native checker commands.
- `fixtures/capabilities/` contains capability-vector fixture material.
- `fixtures/work-tracker-checker/` is raw checker-boundary material for
  `WORK-TRACKER-CHECKER-PROFILE`.

Spec-to-checker traceability is tracked in:

- `specs/premath/draft/SPEC-TRACEABILITY.md`

Run native checker/test entrypoints with:

```bash
cargo test --workspace
cargo run --package premath-cli -- traceability-check
cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json
cargo run --package premath-cli -- drift-budget-check --json
```

Run toy/KCIR toy vectors with:

```bash
python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures
python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures
```
