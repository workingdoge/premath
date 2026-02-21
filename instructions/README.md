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

Optional doctrine-typing fields:

- `instructionType` (string): explicit typed kind
  (for example `ci.gate.check`, `ci.gate.pre_commit`, `ci.gate.pre_push`).
- `typingPolicy` (object):
  - `allowUnknown` (boolean, default `false`): if `false`, classification
    `unknown(reason)` is rejected deterministically before check execution.

Run an envelope:

```bash
mise run ci-instruction-check
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
mise run ci-instruction-smoke
```

This writes a witness artifact to `artifacts/ciwitness/<instruction-id>.json`.

Golden smoke fixture:

- `tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json`
