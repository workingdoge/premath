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
- `normalizerId` (string): deterministic normalization binding identifier.
- `policyDigest` (string): canonical policy artifact digest
  (`pol1_<sha256(...)>`), resolved from `policies/instruction/*.json`.
- `requestedChecks` (string[]): gate checks to execute (for example `hk-check`);
  MUST be allowlisted for the active `policyDigest`.

Optional doctrine-typing fields:

- `instructionType` (string): explicit typed kind
  (for example `ci.gate.check`, `ci.gate.pre_commit`, `ci.gate.pre_push`).
- `typingPolicy` (object):
  - `allowUnknown` (boolean, default `false`): if `false`, classification
    `unknown(reason)` is rejected deterministically before check execution.
- `capabilityClaims` (string[]): optional deterministic capability claims carried
  into instruction witnesses (used by instruction-linked mutation policy
  enforcement for MCP issue/dep writes).
- `proposal` (object): optional inline LLM proposal payload (or legacy alias
  `llmProposal`) validated under
  `specs/premath/draft/LLM-PROPOSAL-CHECKING.md`.
  - include `proposalKind`, `targetCtxRef`, `targetJudgment`, and `binding`
    (`normalizerId`, `policyDigest`) at minimum.
  - proposal binding MUST match top-level instruction `normalizerId` and
    `policyDigest`.
  - optional `proposalDigest` MUST match canonical proposal digest when present.
  - proposal handling is checking-only: canonical proposal claims compile to
    deterministic `obligations[]` then `discharge` in normalized mode before
    check execution.
  - instruction witnesses include this under
    `proposalIngest.obligations` + `proposalIngest.discharge`.

Run an envelope:

```bash
mise run ci-instruction-check
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-pipeline-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
mise run ci-instruction-smoke
```

This writes a witness artifact to `artifacts/ciwitness/<instruction-id>.json`.
If envelope validation fails, a reject witness is still emitted with
`rejectStage: pre_execution` and deterministic `failureClasses`.

Provider workflow rule:

- CI workflows should invoke the provider-neutral wrapper
  `python3 tools/ci/pipeline_instruction.py --instruction "$INSTRUCTION_PATH"`
  and keep envelope orchestration out of inline YAML scripts.

Golden smoke fixture:

- `tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json`

Policy registry:

- `policies/instruction/README.md`
- `policies/instruction/*.json`
