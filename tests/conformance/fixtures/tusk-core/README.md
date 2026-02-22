# Tusk-Core Fixtures

Executable deterministic vectors for `raw/TUSK-CORE` runtime contracts via:

- `premath tusk-eval`

Coverage includes:

- accepted single-glue selection,
- locality failure mapping for missing locals,
- locality failure mapping for missing overlap compatibility in multi-local packs,
- descent failure mapping for missing glue proposals,
- descent failure mapping for missing mode binding,
- glue-non-contractible mapping for ambiguous multi-proposal glue selection.

Run with:

```bash
python3 tools/conformance/run_tusk_core_vectors.py
```
