# Instruction Envelopes

Instruction envelopes are JSON files consumed by `tools/ci/run_instruction.sh`.

Doctrine contract:

- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`

Recommended filename shape:

- `instructions/<ts>-<id>.json`
- example: `instructions/20260221T000000Z-bootstrap-gate.json`

Required fields:

- `intent` (string): why this instruction exists.
- `scope` (string/object): scope of change/evaluation.
- `policyDigest` (string): policy binding identifier.
- `requestedChecks` (string[]): gate checks to execute (for example `hk-check`).

Run an envelope:

```bash
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
```

This writes a witness artifact to `artifacts/ciwitness/<instruction-id>.json`.
