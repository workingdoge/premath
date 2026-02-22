# Conformance Fixtures

This directory contains Premath conformance fixtures.

Current status:

- `fixtures/interop-core/` is executable.
- `fixtures/gate/` is still a layout placeholder.
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
python3 tools/conformance/check_stub_invariance.py
```

Run executable interop-core vectors with:

```bash
python3 tools/conformance/run_interop_core_vectors.py
```

Run executable capability vectors with:

```bash
python3 tools/conformance/run_capability_vectors.py
```
