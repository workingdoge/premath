# CI SqueakSite Shim

`tools/ci/run_gate.sh` is the host-agnostic gate entrypoint used by `mise run ci-check`.

It separates:

- **semantic gate surface**: `hk` profiles/tasks (`hk-check`, `hk-pre-commit`, ...)
- **execution substrate**: local process vs external runner

`tools/ci/run_instruction.sh` is the instruction-envelope entrypoint:

- input: `instructions/<ts>-<id>.json`
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

## Example

```bash
PREMATH_SQUEAK_SITE_PROFILE=local mise run ci-check

# external runner wrapper (user-provided)
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=./tools/ci/executors/my_runner.sh \
mise run ci-check
```

Instruction envelope run:

```bash
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
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
