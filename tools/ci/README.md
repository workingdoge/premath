# CI SqueakSite Shim

`tools/ci/run_required_checks.py` is the canonical closure gate entrypoint used
by `mise run ci-required`.

It computes deterministic change projection (`Delta -> requiredChecks`) and
executes only those checks through `tools/ci/run_gate.sh`.
For each executed check it requests a per-check gate envelope artifact under
`artifacts/ciwitness/gates/<projection-digest>/` and links it from
`ci.required.v1` via `gateWitnessRefs`.
`run_gate.sh` prefers a native runner/task artifact when present; otherwise it
emits a deterministic fallback envelope (`tools/ci/emit_gate_witness.py`).
Each gate ref includes `source: native|fallback` provenance.

`tools/ci/run_gate.sh` is the host-agnostic task executor shim used by both
`ci-required` and fixed-task flows like `mise run ci-check`.
When `PREMATH_GATE_WITNESS_OUT` is set (by `ci-required`), it also handles
native-or-fallback gate envelope emission for that check.

`mise run ci-check` remains as legacy compatibility for fixed full-gate routing.

`tools/ci/verify_required_witness.py` verifies `ci.required` artifacts against
deterministic projection semantics.
When `gateWitnessRefs` are present, verification also enforces linkage integrity
(check ordering, artifact digest, and payload/result consistency).
`--require-native-check <id>` can phase in native-only requirements for selected
checks.
By default it verifies `artifacts/ciwitness/latest-required.json`.

`tools/ci/decide_required.py` emits deterministic merge/promotion decisions from
verified witness semantics (`accept` or `reject`).
`mise run ci-decide-required` writes `artifacts/ciwitness/latest-decision.json`.

It separates:

- **semantic gate surface**: `hk` profiles/tasks (`hk-check`, `hk-pre-commit`, ...)
- **execution substrate**: local process vs external runner

`tools/ci/run_instruction.sh` is the instruction-envelope entrypoint:

- input: `instructions/<ts>-<id>.json`
- classifies instruction as `typed(kind)` or `unknown(reason)` (doctrine-level)
- rejects unroutable `unknown(reason)` unless `typingPolicy.allowUnknown=true`
- executes requested gate checks through `run_gate.sh`
- output: `artifacts/ciwitness/<ts>-<id>.json`

## SqueakSite Profiles

- `PREMATH_SQUEAK_SITE_PROFILE=local` (default)
  - runs `mise run <task>` in the current environment.
- `PREMATH_SQUEAK_SITE_PROFILE=external`
  - delegates to `PREMATH_SQUEAK_SITE_RUNNER` (an executable).
  - runner protocol: `<runner> <task>` and exit code passthrough.

Legacy aliases remain accepted:

- `PREMATH_EXECUTOR_PROFILE`
- `PREMATH_EXECUTOR_RUNNER`

This keeps policy/admissibility stable while allowing host-specific provisioning
(Darwin microVM, Linux VM host, remote worker, etc.) in runner scripts.
See `tools/ci/executors/README.md` for runner responsibilities.

## Required Check Mapping

Canonical CI decision surface is `mise run ci-decide-required`.

Provider-specific check naming/binding guidance lives in
`docs/design/CI-PROVIDER-BINDINGS.md`.

## Example

```bash
PREMATH_SQUEAK_SITE_PROFILE=local mise run ci-required

# external runner wrapper (user-provided)
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=./tools/ci/executors/my_runner.sh \
mise run ci-required

mise run ci-verify-required
mise run ci-required-verified
mise run ci-decide-required

# strict mode: compare witness changedPaths to detected delta
mise run ci-verify-required-strict

# strict mode + phase-in native-only requirement
mise run ci-verify-required-strict-native
```

Instruction envelope run:

```bash
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
```

Inspect projection plan without executing checks:

```bash
python3 tools/ci/project_checks.py
python3 tools/ci/project_checks.py --changed-file crates/premath-kernel/src/lib.rs
```

## Terraform/OpenTofu Shape

Optional wrapper:

```bash
mise run ci-check-tf
```

This runs `tools/infra/terraform/up.sh` to resolve `premath_cheese_runner`
from Terraform/OpenTofu output, then executes the gate through the external
runner profile.

Default Terraform runner profile is `local`.
Experimental runtime profile: `darwin_microvm_vfkit` (microvm.nix + `vfkit`).
Use:

```bash
# default (local profile)
mise run ci-check-tf
# explicit local
mise run ci-check-tf-local
# experimental
mise run ci-check-tf-microvm
```
