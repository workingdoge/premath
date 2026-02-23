# Conformance Fixture Layout

Suggested layout (from `draft/CONFORMANCE.md`):

- `interop-core/{golden,adversarial}`
- `gate/{golden,adversarial}`
- `witness-id/{golden,adversarial}`
- `kernel-profile/{golden,adversarial}`
- `doctrine-inf/{golden,adversarial}`
- `harness-typestate/{golden,adversarial}`
- `runtime-orchestration/{golden,adversarial,invariance}`
- `coherence-transport/{golden,adversarial,invariance}`
- `coherence-site/{golden,adversarial,invariance}`
- `capabilities/<capability-id>/{golden,adversarial,invariance}`

Executable suite entrypoints include:

- `doctrine-inf`: `python3 tools/conformance/run_doctrine_inf_vectors.py`
- `harness-typestate`: `python3 tools/conformance/run_harness_typestate_vectors.py`
  (`expect.json.expectedFailureClasses` must collectively cover all emitted
  harness join-check fail-closed classes and must not include unreferenced
  classes)
- `runtime-orchestration`: `python3 tools/conformance/run_runtime_orchestration_vectors.py`
  (route/morphism/path-boundary checks + optional KCIR mapping row checks +
  invariance scenario parity)
- `coherence-contract`: `cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json`
- cached multi-suite surface: `python3 tools/conformance/run_fixture_suites.py`

Capability fixtures include both:

- executable vectors (currently `capabilities.normal_forms`, `capabilities.kcir_witnesses`, `capabilities.commitment_checkpoints`, `capabilities.squeak_site`, `capabilities.ci_witnesses`, `capabilities.instruction_typing`, `capabilities.adjoints_sites`, and `capabilities.change_morphisms`),
- stub/informational vectors for capabilities not yet wired to executable checks.
