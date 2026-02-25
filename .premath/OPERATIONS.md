# Premath Ops Conventions

This file captures repeatable operational conventions and rollout evidence.

## Main Branch Governance

1. Treat `main` as protected and PR-only.
2. Run local gate closure before push (`mise run ci-required-attested` via hooks).
3. Push to a topic branch, then open a PR to `main`.
4. Require `ci-required` on PR head before merge.

## Branch Policy Rollout Checklist

1. Validate tracked contract fixture:
   - `mise run ci-branch-policy-check`
2. Validate live server policy:
   - `mise run ci-branch-policy-check-live`
3. Ensure workflow credential exists:
   - repository secret `PREMATH_BRANCH_POLICY_TOKEN`
4. Trigger and inspect workflow:
   - `gh workflow run branch-policy.yml -R workingdoge/premath --ref <branch>`
   - `gh run list -R workingdoge/premath --workflow branch-policy.yml --limit 1`
   - `gh run view <run-id> -R workingdoge/premath --log-failed` (if needed)
5. Record evidence in issue notes and append to log below.

## Stage3 Lane Runbook

For `Ev` Stage 3 execution order, gate cadence, and issue hygiene workflow, use:

- `docs/design/EV-COHERENCE-OVERVIEW.md` (§8, Stage 3 Execution Runbook)

## Development Meta Loop (Default)

To avoid re-deriving process shape, use:

- `docs/design/DEVELOPMENT-META-LOOP.md`

Default execution order for non-trivial work:

1. architecture contract
2. spec/index + doctrine-site glue
3. control-plane parity
4. implementation
5. conformance vectors
6. docs/traceability closure

Default close-out checks:

- `mise run docs-coherence-check`
- `mise run traceability-check`
- `mise run coherence-check`
- `python3 tools/ci/check_issue_graph.py`

## Active WIP Topology Ownership Map (bd-280 snapshot)

Snapshot date: 2026-02-23 (UTC)
Authority for active scope: `.premath/issues.jsonl` (`bd-279` chain)

### Design and operations lane

| Cluster | Paths | Owning issue scope |
| --- | --- | --- |
| Issue/ops memory | `.premath/issues.jsonl`, `.premath/OPERATIONS.md`, `.premath/KCIR-SELF-HOSTING-WORKSPACE.md` | `bd-280`, `bd-286` |
| Architecture/design docs | `docs/design/ARCHITECTURE-MAP.md`, `docs/design/DEVELOPMENT-META-LOOP.md`, `docs/design/EV-COHERENCE-OVERVIEW.md`, `docs/design/TUSK-HARNESS-CONTRACT.md`, `docs/design/SQUEAK-DESIGN.md`, `docs/design/RALPH-PLAYBOOK-PREMATH.md`, `docs/design/README.md` | `bd-280`, `bd-285` |
| Root docs and task entrypoint | `README.md`, `COMMITMENT.md`, `RELEASE_NOTES.md`, `.mise.toml` | `bd-285`, `bd-286` |

### Doctrine and decision lane

| Cluster | Paths | Owning issue scope |
| --- | --- | --- |
| Draft doctrine/spec surfaces | `specs/premath/draft/*` (`SPEC-INDEX`, `CONFORMANCE`, `DOCTRINE-*`, `HARNESS-*`, `CAPABILITY-*`, `CONTROL-PLANE-CONTRACT`, `COHERENCE-CONTRACT`) | `bd-281` |
| Raw companion surfaces | `specs/premath/raw/TUSK-CORE.md`, `specs/premath/raw/SQUEAK-CORE.md` | `bd-281` |
| Process doctrine references | `specs/process/HARNESS-SPEC-PROMOTION-MAP.md`, `specs/process/TOPOLOGY-BUDGET.json`, `specs/process/decision-log.md` | `bd-281`, `bd-285` |

### Control/checker lane

| Cluster | Paths | Owning issue scope |
| --- | --- | --- |
| CI wrapper and control-plane parity | `tools/ci/pipeline_required.py`, `tools/ci/pipeline_instruction.py`, `tools/ci/control_plane_contract.py`, `tools/ci/governance_gate.py`, `tools/ci/kcir_mapping_gate.py`, `tools/ci/check_drift_budget.py`, `tools/ci/README.md` | `bd-282` |
| Control-plane tests | `tools/ci/test_*` (pipeline/control-plane/kcir/retry/drift/issue graph suites) | `bd-282` |

### Runtime implementation lane

| Cluster | Paths | Owning issue scope |
| --- | --- | --- |
| BD/CLI/coherence/tusk crate work | `crates/premath-bd/*`, `crates/premath-cli/*`, `crates/premath-coherence/*`, `crates/premath-tusk/*` | `bd-283` |
| Control policy artifact bound to runtime wrappers | `policies/control/harness-retry-policy-v1.json` | `bd-282`, `bd-283` |

### Conformance lane

| Cluster | Paths | Owning issue scope |
| --- | --- | --- |
| Doctrine/harness/runtime vector fixtures | `tests/conformance/fixtures/doctrine-inf/*`, `tests/conformance/fixtures/harness-typestate/*`, `tests/conformance/fixtures/runtime-orchestration/*` | `bd-284` |
| Conformance check/vector runners | `tools/conformance/run_*`, `tools/conformance/check_*`, `tools/conformance/generate_doctrine_site.py`, `tools/conformance/README.md`, `tools/conformance/test_*` | `bd-284` |

Ownership closure assertion (snapshot):

- every dirty cluster from `git status --porcelain` is mapped above to an active
  issue scope in `bd-279` chain,
- no unowned dirty cluster remains outside tracked issue scope.

## Session Continuity

For restart-safe context continuity between MCP/server sessions, use the
canonical harness-session artifact and command surface:

- artifact path: `.premath/harness_session.json`
- read: `cargo run --package premath-cli -- harness-session read --path .premath/harness_session.json --json`
- write: `cargo run --package premath-cli -- harness-session write --path .premath/harness_session.json --state <active|stopped> --issue-id <bd-id> --summary <text> --next-step <text> --instruction-ref <path-or-ref> --witness-ref <path-or-ref> --json`
- bootstrap: `cargo run --package premath-cli -- harness-session bootstrap --path .premath/harness_session.json --feature-ledger .premath/harness_feature_ledger.json --json`

## Evidence Log

| Date (UTC) | Operation | Issue | Decision | Evidence |
| --- | --- | --- | --- | --- |
| 2026-02-22 | Applied `main` protection contract via GitHub API (strict `ci-required`, PR review required, `enforce_admins=true`, no force push/delete). | `bd-64` | - | API response captured in `bd-64` notes. |
| 2026-02-22 | First `branch-policy` workflow run failed on pre-fix commit. | `bd-64` | - | https://github.com/workingdoge/premath/actions/runs/22272316140 |
| 2026-02-22 | Fix branch and PR created for checker fallback/admin-bypass hardening. | `bd-64` | - | https://github.com/workingdoge/premath/pull/8 |
| 2026-02-22 | `branch-policy` workflow passed after checker fix on PR branch. | `bd-64` | - | https://github.com/workingdoge/premath/actions/runs/22272381740 |
| 2026-02-22 | `branch-policy` workflow passed on latest PR head (`658b72f`). | `bd-64` | - | https://github.com/workingdoge/premath/actions/runs/22272407035 |
| 2026-02-22 | Set bootstrap review mode (`required_approving_review_count=0`) while retaining PR-only + `ci-required` + `enforce_admins=true`. | `bd-67` | - | Transition tracked by `bd-67`. |
| 2026-02-22 | PR #8 merged to `main` (`98988bd`). | `bd-64` | - | https://github.com/workingdoge/premath/pull/8 |
| 2026-02-22 | `branch-policy` workflow passed on `main` post-merge (`98988bd`). | `bd-67` | - | https://github.com/workingdoge/premath/actions/runs/22272487569 |
| 2026-02-23 | Parallel harness pilot dispatch over two detached worktrees using explicit `human-override` mode (`bd-210..bd-213`). | `bd-210`, `bd-211`, `bd-212`, `bd-213` | All four pilot slices closed; queue returned to `ready=0` with only manual blocked governance item remaining. | `mise run harness-coordinator-loop -- --worktree ../premath-w1 --worktree ../premath-w2 --rounds 2 --worker-prefix lane --max-steps-per-worker 1 --mutation-mode human-override --override-reason 'operator approved parallel pilot run' --work-cmd 'python3 tools/ci/check_issue_graph.py >/dev/null' --verify-cmd 'python3 tools/ci/check_issue_graph.py >/dev/null'`; `cargo run --package premath-cli -- harness-trajectory query --path .premath/harness_trajectory.jsonl --mode latest --limit 10 --json`; `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`; `cargo run --package premath-cli -- issue blocked --issues .premath/issues.jsonl --json` |
| 2026-02-23 | Parallel ops-probe batch over two detached worktrees using explicit `human-override` mode (`bd-214..bd-217`). | `bd-214`, `bd-215`, `bd-216`, `bd-217` | All four ops-probe slices closed after rerun; first `bd-214` attempt failed due untrusted `mise` config in detached worktree and was recovered by rerunning with non-`mise` command bundle. | `mise run harness-coordinator-loop -- --worktree ../premath-w1 --worktree ../premath-w2 --rounds 2 --worker-prefix lane --max-steps-per-worker 1 --mutation-mode human-override --override-reason 'operator approved parallel ops probe batch' --work-cmd 'python3 tools/ci/check_issue_graph.py >/dev/null && python3 tools/ci/check_command_surface.py >/dev/null && python3 tools/harness/benchmark_kpi.py --json >/dev/null' --verify-cmd 'python3 tools/ci/check_issue_graph.py >/dev/null && cargo run --package premath-cli -- issue check --issues .premath/issues.jsonl --json >/dev/null'`; `cargo run --package premath-cli -- harness-trajectory query --path .premath/harness_trajectory.jsonl --mode latest --limit 12 --json`; `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`; `cargo run --package premath-cli -- issue blocked --issues .premath/issues.jsonl --json` |
| 2026-02-24 | Worldization closure pass: route-binding enforcement + overlay boundary + newcomer docs sync. | `bd-306`, `bd-308`, `bd-309` | Decision 0120 adopted; worldized route vectors expanded and torsor overlay constrained to non-authority interpretation. | `python3 tools/conformance/run_runtime_orchestration_vectors.py`; `mise run conformance-run`; `python3 tools/conformance/run_capability_vectors.py`; `mise run doctrine-check`; `mise run coherence-check`; `mise run docs-coherence-check`; `python3 tools/conformance/check_spec_traceability.py` |
| 2026-02-24 | World-in-kernel topology reduction + docs/traceability closure pass. | `bd-318`, `bd-319` | Core `world-registry-check` now derives required world bindings from control-plane contract; wrapper world semantics removed; runtime vectors split to adapter parity while world semantics run in dedicated world-core suite; newcomer/docs/spec traceability narrative synchronized. | `cargo test -p premath-cli world_registry_check`; `python3 tools/conformance/test_runtime_orchestration.py`; `python3 tools/conformance/run_runtime_orchestration_vectors.py --fixtures tests/conformance/fixtures/runtime-orchestration`; `python3 tools/conformance/run_world_core_vectors.py --fixtures tests/conformance/fixtures/world-core`; `mise run ci-pipeline-test`; `mise run doctrine-check`; `mise run docs-coherence-check`; `python3 tools/conformance/check_spec_traceability.py`; `mise run conformance-run` |
| 2026-02-24 | Locked KCIR-first resolver roadmap before context rollover (epic + ordered tasks + session handoff). | `bd-332`..`bd-343` | Established canonical dependency chain `bd-333 -> bd-339 -> bd-338 -> bd-340 -> bd-334 -> bd-341 -> bd-335 -> bd-336 -> bd-342 -> bd-343 -> bd-337`; persisted restart state in harness-session artifact. | `cargo run --package premath-cli -- issue add ...`; `cargo run --package premath-cli -- dep add ...`; `cargo run --package premath-cli -- issue ready --issues .premath/issues.jsonl --json`; `cargo run --package premath-cli -- harness-session write --path .premath/harness_session.json --state stopped --issue-id bd-333 --summary '<kcir-first resolver staged>' --next-step '<claim bd-333>' --instruction-ref issues:bd-332->bd-343 --witness-ref issue-memory:.premath/issues.jsonl --json`; `cargo run --package premath-cli -- issue update bd-332 --notes ... --json` |
| 2026-02-24 | Closed resolver K0 spec contract and resolver K0.2 operation-class contract. | `bd-333`, `bd-339` | Decision 0124 + Decision 0125 adopted; SITE-RESOLVE promoted and doctrine operation registry now enforces explicit class policy with fail-closed route eligibility binding. | `mise run docs-coherence-check`; `mise run traceability-check`; `mise run doctrine-check`; `python3 tools/conformance/test_doctrine_site_contract.py`; `python3 tools/conformance/generate_doctrine_site.py`; `cargo run --package premath-cli -- issue update bd-333 --status closed ...`; `cargo run --package premath-cli -- issue update bd-339 --status closed ...`; `cargo run --package premath-cli -- harness-session write --path .premath/harness_session.json --state stopped --issue-id bd-338 ... --json` |
