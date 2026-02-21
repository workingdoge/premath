# Terraform/OpenTofu Infra Profile

Status: scaffold

This profile resolves an external runner via Terraform-compatible outputs.

## Contract

The module in `infra/terraform/` MUST output:

- `premath_cheese_runner`: absolute path to an executable runner script.
- `premath_cheese_profile`: active named profile.

Legacy aliases remain available:

- `premath_executor_runner`
- `premath_runner_profile`

`tools/infra/terraform/up.sh` does:

1. `init`
2. `apply`
3. `output -raw premath_cheese_runner`

`tools/ci/run_gate_terraform.sh` consumes that output and executes:

```bash
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=<output> \
sh tools/ci/run_gate.sh <task>
```

## Binary Selection

- `PREMATH_TF_BIN` overrides binary choice.
- Otherwise the scripts prefer `tofu`, then `terraform`.

## Runner Profiles

- default profile: `local`
  - `tools/ci/executors/local_runner.sh`
- optional experimental profile: `darwin_microvm_vfkit`
  - `tools/ci/executors/darwin_microvm_vfkit_runner.sh`
  - prototype only; not part of required CI closure

Terraform variable controls:

- `TF_VAR_cheese_profile` (`local` | `darwin_microvm_vfkit`)
- `TF_VAR_cheese_relpath_override` (path override, takes precedence)
- legacy aliases: `TF_VAR_runner_profile`, `TF_VAR_runner_relpath_override`
- optional checked-in example: `infra/terraform/terraform.tfvars.example`

## Commands

```bash
mise run infra-up
mise run ci-check-tf
mise run infra-down
```

Convenience local-profile run:

```bash
mise run ci-check-tf-local
```

Experimental microvm run:

```bash
mise run ci-check-tf-microvm
```

Darwin VM runner settings are controlled through env at execution time
(`PREMATH_MICROVM_CPUS`, `PREMATH_MICROVM_MEMORY_MB`, `PREMATH_MICROVM_HYPERVISOR`,
`PREMATH_MICROVM_DRY_RUN`).
