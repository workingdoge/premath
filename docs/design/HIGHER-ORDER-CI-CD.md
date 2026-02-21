# Higher-Order CI/CD (DevOps Control Loop)

Status: draft
Scope: design-level, non-normative

Spec counterpart: `specs/premath/raw/PREMATH-CI.md`.

## 0. Implementation Status (as of February 21, 2026)

Implemented in this repo:

- local fast/full/staged gate triggers via `jj gate-fast|gate-check|gate-pre-commit`
- CI gate path via `.github/workflows/baseline.yml` -> `mise run ci-required`
- witness verification path via `.github/workflows/baseline.yml` -> `mise run ci-verify-required-strict`
- decision gate path (local/CI) via `mise run ci-decide-required`
- CI witness artifact publication path via `.github/workflows/baseline.yml`
  (`latest-required.json`, `.sha256`, `proj1_*.json`, summary digest row)
- instruction-envelope gate path via
  `sh tools/ci/run_instruction.sh instructions/<ts>-<id>.json`
  emitting `artifacts/ciwitness/<instruction-id>.json`
- optional local orchestration via `pitchfork.toml` + `mise run pf-*` tasks
- optional infra provisioning scaffold via `mise run infra-up|infra-down|ci-check-tf`
- doctrine-to-operation site map and checker:
  `specs/premath/draft/DOCTRINE-SITE.{md,json}` +
  `python3 tools/conformance/check_doctrine_site.py`
- instruction typing/binding doctrine:
  `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`

Not yet implemented:

- promotion/acceptance decisions driven directly by verified CI witness bundles
- hardened cross-host microvm runtime profile (current microvm runner is experimental/prototype)

## 1. Intent

Treat CI/CD as an operational control loop over Premath semantics, not as a
vendor-specific YAML pipeline.

The core object is:

```text
Delta (change set) -> G(Delta) (required gate set)
```

where `G` is closure-like and preserves kernel-level invariants.

## 2. Plane Split

This repository already has the right split:

- semantic plane: `premath-kernel` (`Gate`, descent, witness classes)
- execution plane: `premath-tusk` (unit lifecycle, descent artifacts, run IDs)
- transport plane: `squeak`/SigPi contracts (`specs/premath/raw/SQUEAK-CORE.md`)
- runtime-location site plane: Squeak site contracts (`specs/premath/raw/SQUEAK-SITE.md`)
- context/lineage plane: `jj` (`premath-jj`, `ctx_ref`)
- data plane: `bd`/JSONL (+ optional query projection like surreal)
- gate execution plane: `hk` + `mise` + CI runner backend
- infra provisioning plane: Terraform/OpenTofu profile (`tools/infra/terraform`)

`hk`/CI runners execute checks; they do not define semantic admissibility.

## 3. Coding Environment As CI Runtime

Current shape:

- local fast loop: `jj gate-fast` (delegates to `hk fix` profile)
- local required closure: `jj gate-check` (delegates to `hk check` -> `mise run ci-required`)
- optional staged-flow gate: `jj gate-pre-commit` (Git index semantics)
- canonical projected gate entrypoint: `mise run ci-required` (`tools/ci/run_required_checks.py`)
  - computes `Delta -> requiredChecks` deterministically before execution
  - executes each required check through `tools/ci/run_gate.sh`
- canonical witness verifier: `mise run ci-verify-required`
  (`tools/ci/verify_required_witness.py`)
  - strict CI mode: `mise run ci-verify-required-strict` (`--compare-delta`)
  - phase-in native requirement:
    `mise run ci-verify-required-strict-native` (`--require-native-check ...`)
- canonical decision surface: `mise run ci-decide-required`
  (`tools/ci/decide_required.py`) -> deterministic `accept|reject`
- default profile: `PREMATH_SQUEAK_SITE_PROFILE=local`
  - optional external profile:
    `PREMATH_SQUEAK_SITE_PROFILE=external` + `PREMATH_SQUEAK_SITE_RUNNER=<path>`
  - legacy aliases still accepted:
    `PREMATH_EXECUTOR_PROFILE` + `PREMATH_EXECUTOR_RUNNER`
- CI gate: `.github/workflows/baseline.yml` runs `mise run ci-required` and `mise run ci-verify-required-strict`
- optional infra-provisioned gate: `mise run ci-check-tf`
  - default infra runner profile: `local`
  - experimental runtime profile: `darwin_microvm_vfkit` (microvm.nix + vfkit)
- optional local orchestration runtime: `pitchfork` (`pitchfork.toml`)
  - `docs-preview` on-demand (`mise run pf-start`)
  - optional closure loop (`mise run pf-gate-loop-start`, then every 30m)

This gives one gate surface (`hk`/`mise`) with multiple trigger surfaces
(JJ aliases, Git hooks, CI backend) and multiple execution substrates
(local host, external runner).
`pitchfork` can host scheduled/background execution without changing gate semantics.

External runners are where host-specific provisioning lives (for example Darwin
launching Linux microVM workers, Linux-hosted VMs, or remote workers).
In Squeak naming, these runtime units can be modeled as `Cheese` profiles.

## 4. Invariance Rules

The higher-order loop should preserve:

- Gate class invariance (`stability`, `locality`, `descent`, `glue_non_contractible`)
- profile invariance (same semantic input + policy bindings -> same verdict class)
- replay invariance (pinned tools/config produce deterministic gate outcomes)

No transport/backend layer may bypass destination admissibility.

## 5. DevOps Interpretation

In DevOps terms:

- **control policy**: what checks are required (`G(Delta)`)
- **executor**: where checks run (`local`, `CI`, `remote worker`)
- **evidence**: `ci.required.v1` CI witness artifacts now
  (`artifacts/ciwitness/*.json`), with optional `gateWitnessRefs` to
  `GateWitnessEnvelope` artifacts when available

This keeps us vendor-agnostic: GitHub Actions, local hooks, and future runners
can all host the same semantics.

## 6. CIWitness <-> GateWitnessEnvelope Contract (v0)

Current implementation contract:

```text
CIWitness {
  witnessKind = "ci.required.v1"
  projectionDigest
  changedPaths
  requiredChecks
  executedChecks
  results
  verdictClass
  failureClasses
  policyDigest
  gateWitnessRefs?   // optional list of refs to GateWitnessEnvelope artifacts
                    // each ref carries source=native|fallback provenance
}
```

Source-of-truth split:

- `GateWitnessEnvelope` remains the authority for kernel admissibility classes.
- `CIWitness` records CI projection/requiredness execution and attestation metadata.
- `gateWitnessRefs` can point to gate envelopes; it cannot override or upgrade
  gate verdict classes.

Current status in this repo:

- `ci.required.v1` witnesses are emitted and strictly verified.
- per-check gate envelope artifacts are emitted under
  `artifacts/ciwitness/gates/<projection-digest>/` via runner-level native
  handoff when available, with deterministic fallback emission.
- `gateWitnessRefs` is populated deterministically and strict verification
  checks link integrity (check binding, payload digest, payload/result consistency).
