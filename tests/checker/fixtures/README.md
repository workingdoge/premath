# Checker Fixture Layout

Legacy fixture layout:

- `interop-core/{golden,adversarial}`
- `gate/{golden,adversarial}`
- `witness-id/{golden,adversarial}`
- `kernel-profile/{golden,adversarial}`
- `doctrine-inf/{golden,adversarial}`
- `work-tracker-checker/{golden,adversarial}`
- `coherence-transport/{golden,adversarial,invariance}`
- `coherence-site/{golden,adversarial,invariance}`
- `capabilities/<capability-id>/{golden,adversarial,invariance}`

Native checker entrypoints include:

- `doctrine-inf`: `cargo run --package premath-cli -- drift-budget-check --json`
- `coherence-contract`: `cargo run --package premath-cli -- coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root . --json`
- `traceability`: `cargo run --package premath-cli -- traceability-check`

Capability fixtures include retained executable and informational vector
material. The former Python multi-suite runner is intentionally removed.
