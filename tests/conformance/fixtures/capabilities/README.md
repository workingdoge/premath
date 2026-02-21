# Capability Fixtures

Each capability folder includes:

- `manifest.json`: planned vectors for the capability claim
- `golden/`: expected accept/success vectors
- `adversarial/`: deterministic reject vectors
- `invariance/`: paired profile vectors that must preserve kernel verdict and Gate class

Invariance pairs share a `semanticScenarioId` and differ only in evidence profile.

Execution status:

- `capabilities.normal_forms`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.kcir_witnesses`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.commitment_checkpoints`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.squeak_site`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.ci_witnesses`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.instruction_typing`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.change_projection`: executable via `tools/conformance/run_capability_vectors.py`
- `capabilities.ci_required_witness`: executable via `tools/conformance/run_capability_vectors.py`
- other capability folders: stub/informational until upgraded with executable payloads
