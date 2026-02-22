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
