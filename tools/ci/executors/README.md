# External Runner Notes

External runners are invoked by `tools/ci/run_gate.sh` as:

```bash
<runner> <task>
```

where `<task>` is typically `hk-check` or `hk-pre-commit`.

Built-in local stub:

- `tools/ci/executors/local_runner.sh` (runs `mise run <task>` locally)
- useful as an override for non-Darwin hosts

Darwin microvm.nix + vfkit runner:

- `tools/ci/executors/darwin_microvm_vfkit_runner.sh`
- builds an ephemeral microvm.nix flake, runs the declared `vfkit` runner, mounts workspace/output/control shares via virtiofs, runs `mise run <task>` in Linux guest, writes exit code back to host
- experimental profile (`darwin_microvm_vfkit`), not baseline default
- host prerequisites: `nix` with flakes on Darwin
- prototype status: no stability/latency guarantee yet; keep out of required CI paths

Darwin runner tunables (env):

- `PREMATH_MICROVM_CPUS` (default `4`)
- `PREMATH_MICROVM_MEMORY_MB` (default `4096`)
- `PREMATH_MICROVM_HYPERVISOR` (default `vfkit`)
- `PREMATH_MICROVM_DRY_RUN` (`1` prints generated flake and exits)

Runner responsibilities:

- provision/select the target execution substrate (local VM, remote host, etc.),
- run `mise run <task>` in the target workspace,
- return a nonzero exit code when the gate fails.
- optional native witness handoff:
  - when `PREMATH_GATE_WITNESS_OUT` is set, a runner/task may write a
    GateWitnessEnvelope JSON artifact at that path.
  - if omitted, `run_gate.sh` emits a deterministic fallback envelope.

Example invocation:

```bash
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=./tools/ci/executors/my_runner.sh \
mise run ci-required-verified
```

This keeps gate semantics stable while allowing host-specific implementations
(for example Darwin-initiated Linux microVM workers).
