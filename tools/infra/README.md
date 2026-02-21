# Infra Tooling

This directory contains provisioning-plane helpers.

Goal:

- provision/select execution substrates,
- expose SqueakSite/`Cheese` runner bindings,
- keep gate semantics in `tools/ci/run_gate.sh` unchanged.

Current implementation:

- `terraform/` for Terraform/OpenTofu-compatible provisioning.
- default runner profile in that module is `local`.
- optional experimental profile: `darwin_microvm_vfkit` (microvm.nix + `vfkit`).
