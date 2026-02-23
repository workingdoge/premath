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

- `docs/design/EV-STAGE3-EXECUTION-RUNBOOK.md`

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
