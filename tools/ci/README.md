# CI SqueakSite Shim

Role boundary:

- CI tools in this directory implement control-plane execution/attestation.
- `premath coherence-check` implements control-plane consistency checking.
- semantic admissibility authority remains kernel/Gate/Core obligations, not CI wrappers.

`tools/ci/run_required_checks.py` is the canonical closure gate entrypoint used
by `sh tools/ci/run_task.sh ci-required`.

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
emits a deterministic fallback envelope through `premath required-gate-ref`.
Each gate ref includes `source: native|fallback` provenance.
`ci.required.v1` witness summaries expose deterministic failure-lineage split:

- `operationalFailureClasses`: CI control-plane execution classes
  (for example `check_failed`),
- `semanticFailureClasses`: semantic classes derived from linked gate witness
  payloads where available,
- `failureClasses`: deterministic union of both surfaces (compatibility field).

`tools/ci/run_gate.sh` is the host-agnostic task executor shim used by both
`ci-required` and fixed-task flows like `sh tools/ci/run_task.sh ci-check`.
When `PREMATH_GATE_WITNESS_OUT` is set (by `ci-required`), it also handles
native-or-fallback gate envelope emission for that check.

`sh tools/ci/run_task.sh ci-check` remains as legacy compatibility for fixed full-gate routing.

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
`sh tools/ci/run_task.sh ci-decide-required` writes `artifacts/ciwitness/latest-decision.json`.

`tools/ci/verify_decision.py` verifies the decision attestation chain:

- decision references the current witness and delta snapshot,
- decision hash bindings (`witnessSha256`, `deltaSha256`) match artifact bytes,
- projection/required-check semantics align across decision, witness, and snapshot.

It delegates attestation-chain semantics to core
`premath required-decision-verify`; Python wrapper logic is path/artifact
transport only.

`premath command-surface-check` validates the repository command surface is
direct scripts/Nix and rejects legacy task-runner command/file references
(`sh tools/ci/run_task.sh ci-command-surface-check`).

`premath repo-hygiene-check` validates repository hygiene guardrails for
private/local-only surfaces (for example `.claude/`, `.serena/`,
`.premath/cache/`) and required ignore entries.

`premath issue check` validates core issue-memory semantics from `premath-bd`
for machine-actionable planning surfaces:

- `[EPIC]` title rows must use `issue_type=epic`,
- active issues (`open`/`in_progress`) must carry an `Acceptance:` section,
- active issues must include at least one verification command surface,
- oversized `notes` payloads are reported as warnings to limit JSONL churn,
- active `blocks` edges that target `closed` issues fail as compactness drift,
- transitive-redundant active `blocks` edges fail as compactness drift.

`sh tools/ci/run_task.sh ci-hygiene-check` runs both native checks. Remove
compactness drift with explicit `premath dep remove` commands so issue-memory
mutation stays auditable.

`premath pipeline-wiring-check` validates provider-specific workflow files
remain thin wrappers around provider-neutral pipeline entrypoints
(`sh tools/ci/run_task.sh ci-pipeline-check`).

`tools/ci/test_pipeline_required.py`,
`tools/ci/test_pipeline_instruction.py`, and
`tools/ci/test_drift_budget.py` are deterministic unit tests for
provider-neutral pipeline summary/digest logic and drift-budget sentinels
(`sh tools/ci/run_task.sh ci-pipeline-test`).

Observation projection now routes through one core command surface:
`premath observe-build` (`sh tools/ci/run_task.sh ci-observation-build`).
`sh tools/ci/run_task.sh ci-observation-query` uses `premath observe`.
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

`sh tools/ci/run_task.sh ci-observation-test` validates deterministic
reducer/query behavior through the native `premath-surreal`, `premath-ux`, and
`premath-cli` tests.
`premath observe-check` enforces projection invariance: observation output must
match a fresh `premath observe-build` projection from current CI witness and
issue-memory artifacts (`sh tools/ci/run_task.sh ci-observation-check`).
`tools/ci/check_drift_budget.py` enforces fail-closed drift-budget sentinels
across docs/contracts/checkers/cache-closure surfaces, includes deterministic
topology-budget metrics from `specs/process/TOPOLOGY-BUDGET.json`, and emits
deterministic `driftClasses` + `warningClasses` summary output
(`sh tools/ci/run_task.sh ci-drift-budget-check`).

`premath observe-serve` (from `premath-cli`) exposes the same observation query
contract as a tiny HTTP read API for frontend clients:

- `GET /latest`
- `GET /needs-attention`
- `GET /instruction?id=<instruction_id>`
- `GET /projection?digest=<projection_digest>[&match=typed|compatibility_alias]`

Projection lookup defaults to `match=typed` (canonical typed authority digest
only). Alias lookups are compatibility-scoped and require
`match=compatibility_alias`.

`tools/ci/pipeline_required.py` is the provider-neutral required-gate pipeline
entrypoint (`sh tools/ci/run_task.sh ci-pipeline-required`): maps provider refs, runs the
attested required gate chain, enforces governance/KCIR mapping gates, and emits
summary/sha artifacts.

`tools/ci/pipeline_instruction.py` is the provider-neutral instruction pipeline
entrypoint (`sh tools/ci/run_task.sh ci-pipeline-instruction`): validates envelope shape, runs
instruction execution, enforces governance/KCIR mapping gates, and emits
summary/sha artifacts.

Workflow authoring contract:

- `.github/workflows/baseline.yml` must call
  `python3 tools/ci/pipeline_required.py`.
- `.github/workflows/instruction.yml` must call
  `python3 tools/ci/pipeline_instruction.py --instruction "$INSTRUCTION_PATH"`.
- workflow files should not inline attestation/summary logic; keep pipeline
  orchestration in `tools/ci/pipeline_*.py`.
- wrapper workflow entrypoints and required gates are declared under
  `providerPipelineWrappers` in
  `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`; `ci-pipeline-check`
  derives its expected commands from that contract.
- validate with:
  - `sh tools/ci/run_task.sh ci-pipeline-check`
  - `sh tools/ci/run_task.sh ci-pipeline-test`

`premath instruction-check` validates instruction envelope schema/shape before
execution (`sh tools/ci/run_task.sh ci-instruction-check`).

`tools/ci/test_instruction_smoke.py` runs a deterministic instruction witness
smoke check against a golden fixture (`sh tools/ci/run_task.sh ci-instruction-smoke`).

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
  - runs `sh tools/ci/run_task.sh <task>` in the current environment.
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

Canonical CI decision surface is `python3 tools/ci/run_required_attested.py`.
Provider-neutral workflow entrypoint is `python3 tools/ci/pipeline_required.py`.
Instruction decision surface is `python3 tools/ci/run_instruction.py`.

Command-surface contract authority is
`specs/premath/draft/CONTROL-PLANE-CONTRACT.json` under `commandSurface`:

- `requiredDecision.canonicalEntrypoint`: `python3 tools/ci/run_required_attested.py`
- `requiredDecision.compatibilityAliases`: none
- `instructionEnvelopeCheck.canonicalEntrypoint`:
  `cargo run --package premath-cli -- instruction-check --instruction`
- `instructionDecision.canonicalEntrypoint`:
  `python3 tools/ci/run_instruction.py`
- `instructionDecision.compatibilityAliases`:
  `sh tools/ci/run_instruction.sh`

Provider-specific check naming/binding guidance lives in
`docs/design/control-plane/CI-PROVIDER-BINDINGS.md`.

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
PREMATH_CI_BASE_REF=origin/main PREMATH_CI_HEAD_REF=HEAD sh tools/ci/run_task.sh ci-verify-required-strict
PREMATH_CI_BASE_REF=origin/main PREMATH_CI_HEAD_REF=HEAD sh tools/ci/run_task.sh ci-decide-required
```

Provider-neutral pipelines map provider refs internally before strict delta
verification. Workflows should not call standalone provider-env export scripts.

## Example

```bash
PREMATH_SQUEAK_SITE_PROFILE=local sh tools/ci/run_task.sh ci-required

# external runner wrapper (user-provided)
PREMATH_SQUEAK_SITE_PROFILE=external \
PREMATH_SQUEAK_SITE_RUNNER=./tools/ci/executors/my_runner.sh \
sh tools/ci/run_task.sh ci-required

sh tools/ci/run_task.sh ci-command-surface-check
sh tools/ci/run_task.sh ci-hygiene-check
sh tools/ci/run_task.sh ci-pipeline-check
sh tools/ci/run_task.sh ci-pipeline-test
sh tools/ci/run_task.sh ci-observation-test
sh tools/ci/run_task.sh ci-observation-build
sh tools/ci/run_task.sh ci-observation-query
sh tools/ci/run_task.sh ci-observation-serve
sh tools/ci/run_task.sh ci-observation-check
sh tools/ci/run_task.sh ci-verify-required
sh tools/ci/run_task.sh ci-required-verified
sh tools/ci/run_task.sh ci-required-attested
sh tools/ci/run_task.sh ci-pipeline-required
sh tools/ci/run_task.sh ci-decide-required
sh tools/ci/run_task.sh ci-verify-decision

# strict mode: compare witness changedPaths to detected delta
sh tools/ci/run_task.sh ci-verify-required-strict

# strict mode + phase-in native-only requirement
sh tools/ci/run_task.sh ci-verify-required-strict-native
```

Instruction envelope run:

```bash
sh tools/ci/run_task.sh ci-instruction-check
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json sh tools/ci/run_task.sh ci-pipeline-instruction
INSTRUCTION=instructions/20260221T000000Z-bootstrap-gate.json sh tools/ci/run_task.sh ci-instruction
sh tools/ci/run_instruction.sh instructions/20260221T000000Z-bootstrap-gate.json
sh tools/ci/run_task.sh ci-instruction-smoke
```

GitHub manual dispatch workflow:

- `.github/workflows/instruction.yml`
- inputs: `instruction_path` and `allow_failure`
- validates envelope schema/shape, runs instruction, uploads witness artifact

Inspect projection plan without executing checks:

```bash
printf '{"changedPaths":["crates/premath-kernel/src/lib.rs"]}\n' > /tmp/premath-required-projection.json
cargo run --package premath-cli -- required-projection --input /tmp/premath-required-projection.json --json
```

## Terraform/OpenTofu Shape

Optional wrapper:

```bash
sh tools/ci/run_task.sh ci-check-tf
```

This runs `tools/infra/terraform/up.sh` to resolve `premath_cheese_runner`
from Terraform/OpenTofu output, then executes the gate through the external
runner profile.

Default Terraform runner profile is `local`.
Experimental runtime profile: `darwin_microvm_vfkit` (microvm.nix + `vfkit`).
Use:

```bash
# default (local profile)
sh tools/ci/run_task.sh ci-check-tf
# explicit local
sh tools/ci/run_task.sh ci-check-tf-local
# experimental
sh tools/ci/run_task.sh ci-check-tf-microvm
```
