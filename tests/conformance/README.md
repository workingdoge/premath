# Conformance Fixtures

This directory contains Premath conformance fixtures.

Current status:

- `fixtures/interop-core/` is executable.
- `fixtures/gate/` is executable.
- `fixtures/doctrine-inf/` is executable.
- `fixtures/capabilities/` contains capability vectors.
  - `capabilities.normal_forms` is executable.
  - `capabilities.kcir_witnesses` is executable.
  - `capabilities.commitment_checkpoints` is executable.
  - `capabilities.squeak_site` is executable.
  - `capabilities.ci_witnesses` is executable.
  - `capabilities.instruction_typing` is executable.
  - `capabilities.adjoints_sites` is executable.
  - `capabilities.change_morphisms` is executable.
  - other capability tracks are currently stub/informational.

Spec-to-suite traceability is tracked in:

- `specs/premath/draft/SPEC-TRACEABILITY.md`

Validate fixture integrity and invariance pairing with:

```bash
cargo run --package premath-cli -- capability-stub-invariance-check --fixtures tests/conformance/fixtures/capabilities --json
```

Run executable interop-core vectors with:

```bash
python3 tools/conformance/run_interop_core_vectors.py
```

Run executable gate vectors with:

```bash
python3 tools/conformance/run_gate_vectors.py
```

Run executable doctrine-inf vectors with:

```bash
python3 tools/conformance/run_doctrine_inf_vectors.py
```

`run_doctrine_inf_vectors.py` delegates semantic evaluation to
`premath doctrine-inf-check` and only performs fixture orchestration/expect
comparison.

Run executable capability vectors with:

```bash
python3 tools/conformance/run_capability_vectors.py
```
