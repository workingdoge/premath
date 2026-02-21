# Conformance Fixture Layout

Suggested layout (from `draft/CONFORMANCE.md`):

- `interop-core/{golden,adversarial}`
- `gate/{golden,adversarial}`
- `capabilities/<capability-id>/{golden,adversarial,invariance}`

Capability fixtures include both:

- executable vectors (currently `capabilities.normal_forms`, `capabilities.kcir_witnesses`, `capabilities.commitment_checkpoints`, `capabilities.squeak_site`, `capabilities.ci_witnesses`, and `capabilities.instruction_typing`),
- stub/informational vectors for capabilities not yet wired to executable checks.
