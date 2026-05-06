# Capability Fixtures

Each capability folder includes:

- `manifest.json`: planned vectors for the capability claim
- `golden/`: expected accept/success vectors
- `adversarial/`: deterministic reject vectors
- `invariance/`: paired profile vectors that must preserve kernel verdict and Gate class

Invariance pairs share a `semanticScenarioId` and differ only in evidence profile.

Execution status:

- `capabilities.normal_forms`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.kcir_witnesses`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.commitment_checkpoints`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.squeak_site`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.instruction_typing`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.adjoints_sites`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- `capabilities.change_morphisms`: executable via `crates/premath-cli/src/commands/coherence_check.rs`
- other capability folders: stub/informational until upgraded with executable payloads
