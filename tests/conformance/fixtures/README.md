# Conformance Fixture Layout

Suggested layout (from `draft/CONFORMANCE.md`):

- `interop-core/{golden,adversarial}`
- `gate/{golden,adversarial}`
- `witness-id/{golden,adversarial}`
- `kernel-profile/{golden,adversarial}`
- `doctrine-inf/{golden,adversarial}`
- `capabilities/<capability-id>/{golden,adversarial,invariance}`

Executable suite entrypoints include:

- `doctrine-inf`: `python3 tools/conformance/run_doctrine_inf_vectors.py`
- cached multi-suite surface: `python3 tools/conformance/run_fixture_suites.py`

Capability fixtures include both:

- executable vectors (currently `capabilities.normal_forms`, `capabilities.kcir_witnesses`, `capabilities.commitment_checkpoints`, `capabilities.squeak_site`, `capabilities.ci_witnesses`, `capabilities.instruction_typing`, `capabilities.adjoints_sites`, and `capabilities.change_morphisms`),
- stub/informational vectors for capabilities not yet wired to executable checks.
