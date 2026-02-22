# CI SqueakSite Shim

Role boundary:

- CI tools in this directory implement control-plane execution/attestation.
- `premath coherence-check` implements control-plane consistency checking.
- semantic admissibility authority remains kernel/Gate/BIDIR, not CI wrappers.

`tools/ci/run_required_checks.py` is the canonical closure gate entrypoint used
by `mise run ci-required`.

It computes deterministic change projection (`Delta -> requiredChecks`) and
executes only those checks through `tools/ci/run_gate.sh`.
Projection + delta detection semantics are core-owned
(`premath required-projection`, `premath required-delta`);
`tools/ci/change_projection.py` is a thin adapter over those command surfaces.
It writes `artifacts/ciwitness/latest-delta.json` as a single-source delta
snapshot for strict compare phases (`ci-verify-required-strict`,
`ci-decide-required`).
For each executed check it requests a per-check gate envelope artifact under
`artifacts/ciwitness/gates/<projection-digest>/` and links it from
`ci.required.v1` via `gateWitnessRefs`.
Gate-ref assembly and fallback gate payload synthesis are core-owned via
`premath required-gate-ref`.
It delegates final `ci.required.v1` witness assembly to core
`premath required-witness` (Python wrapper is transport only).
`run_gate.sh` prefers a native runner/task artifact when present; otherwise it
emits a deterministic fallback envelope (`tools/ci/emit_gate_witness.py`).
Each gate ref includes `source: native|fallback` provenance.
`ci.required.v1` witness summaries expose deterministic failure-lineage split:

- `operationalFailureClasses`: CI control-plane execution classes
  (for example `check_failed`),
- `semanticFailureClasses`: semantic classes derived from linked gate witness
  payloads where available,
- `failureClasses`: deterministic union of both surfaces (compatibility field).

`tools/ci/run_gate.sh` is the host-agnostic task executor shim used by both
`ci-required` and fixed-task flows like `mise run ci-check`.
When `PREMATH_GATE_WITNESS_OUT` is set (by `ci-required`), it also handles
native-or-fallback gate envelope emission for that check.

`mise run ci-check` remains as legacy compatibility for fixed full-gate routing.

`tools/ci/verify_required_witness.py` verifies `ci.required` artifacts against
deterministic projection semantics.
It delegates semantic verification to core
`premath required-witness-verify` via a thin adapter.
When `gateWitnessRefs` are present, verification also enforces linkage integrity
(check ordering, artifact digest, and payload/result consistency).
`--require-native-check <id>` can phase in native-only requirements for selected
checks.
By default it verifies `artifacts/ciwitness/latest-required.json`.

`tools/ci/decide_required.py` emits deterministic merge/promotion decisions from
verified witness semantics (`accept` or `reject`).
It delegates decision semantics to core
`premath required-witness-decide` via a thin adapter.
`mise run ci-decide-required` writes `artifacts/ciwitness/latest-decision.json`.

`tools/ci/verify_decision.py` verifies the decision attestation chain:

- decision references the current witness and delta snapshot,
- decision hash bindings (`witnessSha256`, `deltaSha256`) match artifact bytes,
- projection/required-check semantics align across decision, witness, and snapshot.

It delegates attestation-chain semantics to core
`premath required-decision-verify`; Python wrapper logic is path/artifact
transport only.

`tools/ci/check_ci_wiring.py` validates that CI workflow wiring uses the
canonical attested gate chain entrypoint and does not split the required gate
steps.

`tools/ci/check_command_surface.py` validates the repository command surface is
`mise`-only and rejects legacy task-runner command/file references
(`mise run ci-command-surface-check`).

`tools/ci/check_repo_hygiene.py` validates repository hygiene guardrails for
private/local-only surfaces (for example `.claude/`, `.serena/`,
`.premath/cache/`) and required ignore entries
(`mise run ci-hygiene-check`).

`tools/ci/check_issue_graph.py` validates issue-memory contract invariants
(`.premath/issues.jsonl`) for machine-actionable planning surfaces:

- `[EPIC]` title rows must use `issue_type=epic`,
- active issues (`open`/`in_progress`) must carry an `Acceptance:` section,
- active issues must include at least one verification command surface,
- oversized `notes` payloads are reported as warnings to limit JSONL churn.

`tools/ci/check_branch_policy.py` validates effective GitHub `main` branch
rules against tracked process policy (`specs/process/GITHUB-BRANCH-POLICY.json`)
with two modes:

- fixture/offline deterministic mode (`mise run ci-branch-policy-check`),
- live API mode (`mise run ci-branch-policy-check-live`) for server-side drift
  detection.

Live mode reads `GITHUB_TOKEN` (or an alternate token env via `--token-env`)
and checks the effective rules API surface:
`/repos/{owner}/{repo}/rules/branches/{branch}`.
For this repo policy, bypass actors are fail-closed.

`tools/ci/check_pipeline_wiring.py` validates provider-specific workflow files
remain thin wrappers around provider-neutral pipeline entrypoints
(`mise run ci-pipeline-check`).

`tools/ci/test_pipeline_required.py`,
`tools/ci/test_pipeline_instruction.py`, and
`tools/ci/test_drift_budget.py` are deterministic unit tests for
provider-neutral pipeline summary/digest logic and drift-budget sentinels
(`mise run ci-pipeline-test`).

Observation projection now routes through one core command surface:
`premath observe-build` (`mise run ci-observation-build`).
`tools/ci/observation_surface.py` remains as a thin compatibility wrapper for
tests/scripts, while `mise run ci-observation-query` uses `premath observe`.
The summary includes explicit coherence projections for:

- policy drift,
- unknown instruction classification rate,
- proposal reject classes,
- ready-vs-blocked partition integrity,
- stale/contended lease claims.
It writes:

- `artifacts/observation/latest.json` (deterministic read model),
- `artifacts/observation/events.jsonl` (projection/event feed suitable for
  downstream query stores, including Surreal adapters).

`tools/ci/test_observation_surface.py` validates deterministic reducer/query
behavior (`mise run ci-observation-test`).
`tools/ci/check_observation_semantics.py` enforces projection invariance:
observation output must match a fresh `premath observe-build` projection from
current CI witness and issue-memory artifacts (`mise run ci-observation-check`).
`tools/ci/check_drift_budget.py` enforces fail-closed drift-budget sentinels
across docs/contracts/checkers/cache-closure surfaces and emits deterministic
`driftClasses` summary output (`mise run ci-drift-budget-check`).

`premath observe-serve` (from `premath-cli`) exposes the same observation query
contract as a tiny HTTP read API for frontend clients:

- `GET /latest`
- `GET /needs-attention`
- `GET /instruction?id=<instruction_id>`
- `GET /projection?digest=<projection_digest>`

`tools/ci/pipeline_required.py` is the provider-neutral required-gate pipeline
entrypoint (`mise run ci-pipeline-required`): maps provider refs, runs the
attested required gate chain, and emits summary/sha artifacts.

`tools/ci/pipeline_instruction.py` is the provider-neutral instruction pipeline
entrypoint (`mise run ci-pipeline-instruction`): validates envelope shape, runs
instruction execution, and emits summary/sha artifacts.

Workflow authoring contract:

- `.github/workflows/baseline.yml` must call
  `python3 tools/ci/pipeline_required.py`.
- `.github/workflows/instruction.yml` must call
  `python3 tools/ci/pipeline_instruction.py --instruction "$INSTRUCTION_PATH"`.
- `.github/workflows/branch-policy.yml` runs live branch/ruleset verification
  against `specs/process/GITHUB-BRANCH-POLICY.json` and requires secret
  `PREMATH_BRANCH_POLICY_TOKEN` for admin-read API access.
- workflow files should not inline attestation/summary logic; keep pipeline
  orchestration in `tools/ci/pipeline_*.py`.
- validate with:
  - `mise run ci-pipeline-check`
  - `mise run ci-pipeline-test`
  - `mise run ci-wiring-check`

`tools/ci/check_instruction_envelope.py` validates instruction envelope
schema/shape before execution (`mise run ci-instruction-check`).

`tools/ci/test_instruction_smoke.py` runs a deterministic instruction witness
smoke check against a golden fixture (`mise run ci-instruction-smoke`).

It separates:

- **semantic gate surface**: `hk` profiles/tasks (`hk-check`, `hk-pre-commit`, ...)
- **execution substrate**: local process vs external runner

`tools/ci/run_instruction.sh` is the instruction-envelope entrypoint:

- input: `instructions/<ts>-<id>.json`
- delegates instruction typing/proposal ingestion to core
  `premath instruction-check` (typed `instructionClassification` +
  authoritative `executionDecision` + canonical `instructionDigest`)
- rejects unroutable `unknown(reason)` unless `typingPolicy.allowUnknown=true`
- carries optional `capabilityClaims` from envelope into witness artifacts for
  downstream mutation-policy gating surfaces
- executes requested gate checks through `run_gate.sh` only when
  `executionDecision.state=execute`
- delegates final witness verdict/failure/proposal-ingest assembly to core
  `premath instruction-witness`
- output: `artifacts/ciwitness/<ts>-<id>.json`
  - for proposal-carrying instructions, witness includes deterministic
    `proposalIngest.obligations[]` and normalized `proposalIngest.discharge`
    payloads from core checker semantics.
  - instruction witnesses expose the same lineage split:
    - `operationalFailureClasses` for control-plane classes,
    - `semanticFailureClasses` for proposal-discharge semantic classes when
      present,
    - `failureClasses` as deterministic union for compatibility.
  - envelope validation failures now emit a first-class reject witness
    (`verdictClass=rejected`, `rejectStage=pre_execution`, deterministic
    `failureClasses`) instead of only stderr/exit status.

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

Canonical CI decision surface is `mise run ci-required-attested`.
Provider-neutral workflow entrypoint is `python3 tools/ci/pipeline_required.py`.

Provider-specific check naming/binding guidance lives in
`docs/design/CI-PROVIDER-BINDINGS.md`.

## Provider-Neutral CI Ref Contract

Strict delta compare commands consume canonical refs from environment:

- `PREMATH_CI_BASE_REF` (optional; if unset, auto-detected fallback order is used)
- `PREMATH_CI_HEAD_REF` (optional; default `HEAD`)

Strict compare changed-path source order:

1. explicit `--changed-file` (verify only),
2. `latest-delta.json` snapshot,
3. fallback re-detection from refs.

Examples:

```bash
PREMATH_CI_BASE_REF=origin/main PREMATH_CI_HEAD_REF=HEAD mise run ci-verify-required-strict
PREMATH_CI_BASE_REF=origin/main PREMATH_CI_HEAD_REF=HEAD mise run ci-decide-required
```

GitHub adapter export:

```bash
python3 tools/ci/providers/export_github_env.py
# emits PREMATH_CI_* assignments derived from GITHUB_* env
```

## Example

```bash
PREMATH_SQUEAK_SITE_PROFILE=local mise run ci-required

# external runner wrapper (user-provided)
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=./tools/ci/executors/my_runner.sh \
mise run ci-required

mise run ci-wiring-check
mise run ci-command-surface-check
mise run ci-hygiene-check
mise run ci-branch-policy-check
mise run ci-pipeline-check
mise run ci-pipeline-test
mise run ci-observation-test
mise run ci-observation-build
mise run ci-observation-query
mise run ci-observation-serve
mise run ci-observation-check
mise run ci-verify-required
mise run ci-required-verified
mise run ci-required-attested
mise run ci-branch-policy-check-live
mise run ci-pipeline-required
mise run ci-decide-required
mise run ci-verify-decision

# strict mode: compare witness changedPaths to detected delta
mise run ci-verify-required-strict

# strict mode + phase-in native-only requirement
mise run ci-verify-required-strict-native
```

Instruction envelope run:

```bash
mise run ci-instruction-check
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-pipeline-instruction
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json mise run ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
mise run ci-instruction-smoke
```

GitHub manual dispatch workflow:

- `.github/workflows/instruction.yml`
- inputs: `instruction_path` and `allow_failure`
- validates envelope schema/shape, runs instruction, uploads witness artifact

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
