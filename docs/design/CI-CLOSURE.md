# CI Closure Gate

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define CI/pre-commit as a closure operator over change sets:

- every change set `Delta` maps to a required check set `G(Delta)`,
- the closure condition is that all required checks pass before merge,
- profile/representation changes must preserve kernel invariants.

## 2. Unifying invariant

For fixed semantic inputs and fixed policy bindings:

- kernel accept/reject verdict is invariant across evidence profiles,
- Gate failure classes are invariant across evidence profiles.

This is the load-bearing invariant for optional capabilities.

## 3. Gate entrypoints (current)

Operational source of truth:

- `.mise.toml` (`[tasks.baseline]`, `[tasks.ci-required*]`, `[tasks.doctrine-check]`)
- `tools/ci/pipeline_required.py`
- `tools/ci/pipeline_instruction.py`

Current full baseline gate (`mise run baseline`) includes:

1. setup + language hygiene (`py-setup`, `rust-setup`, `fmt`, `lint`)
2. build/test closure (`build`, `test`, `test-toy`, `test-kcir-toy`)
3. conformance/docs closure
   - `conformance-check`
   - `traceability-check`
   - `docs-coherence-check`
   - `doctrine-check` (site coherence + doctrine-inf vectors)
   - `conformance-run` (cached fixture suite runner)
4. CI/control-plane closure
   - `ci-command-surface-check`
   - `ci-hygiene-check`
   - `ci-pipeline-check`
   - `ci-pipeline-test`
   - `ci-observation-test`
   - `ci-observation-check`
   - `ci-wiring-check`
   - `ci-instruction-check`
   - `ci-instruction-smoke`

Local command:

```bash
mise run baseline
```

Projected required gate (canonical CI entrypoint):

```bash
mise run ci-required
```

`mise run ci-required` computes deterministic change projection
(`Delta -> requiredChecks`) and executes only projected checks.

`mise run ci-verify-required` verifies emitted `ci.required` witness artifacts
against deterministic projection semantics.

`mise run ci-required-verified` runs execution + witness verification.

`mise run ci-required-attested` is the authoritative local/CI chain
(execution + strict verification + decision + decision verification).

Underlying check execution still routes through `tools/ci/run_gate.sh`, so
executor substrate selection (`PREMATH_SQUEAK_SITE_PROFILE`, legacy
`PREMATH_EXECUTOR_PROFILE`) stays decoupled from gate semantics.

Optional infra-provisioned path:

```bash
mise run ci-check-tf
```

This resolves external runner binding from Terraform/OpenTofu output first, then
executes the same gate surface.

Instruction-envelope path:

```bash
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
```

This executes requested checks through the same gate surface and emits
`artifacts/ciwitness/<instruction-id>.json`.

Recommended pre-commit gate:

```bash
mise run precommit
```

Optional hook install:

```bash
git config core.hooksPath .githooks
```

## 4. Entry minimization by change projection

Use `Delta -> G(Delta)` to avoid running unnecessary checks while preserving invariants.

Suggested v0 projection:

- docs-only changes:
  - run conformance stub checker if `specs/premath/raw/` or `tests/conformance/` touched
- Rust crate changes:
  - run build + Rust tests
  - include toy + KCIR toy if `crates/premath-kernel` touched
- conformance fixture/schema/tooling changes:
  - run conformance checker + toy + KCIR toy
- capability/profile semantics changes:
  - run full baseline gate

Implemented in `tools/ci/change_projection.py` and executed via
`tools/ci/run_required_checks.py`.

Current deterministic projected check IDs include:

- `baseline`
- `build`
- `test`
- `test-toy`
- `test-kcir-toy`
- `conformance-check`
- `conformance-run`
- `doctrine-check`

## 5. Variants and capability projection

Variants should declare capability claims explicitly.
CI should verify only vectors for claimed capabilities, while always enforcing kernel-level invariants.

This allows operational variants to specialize without fragmenting semantics.
