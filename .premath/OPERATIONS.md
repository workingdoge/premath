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

| Date (UTC) | Operation | Evidence |
| --- | --- | --- |
| 2026-02-22 | Applied `main` protection contract via GitHub API (strict `ci-required`, PR review required, `enforce_admins=true`, no force push/delete). | API response captured in `bd-64` notes. |
| 2026-02-22 | First `branch-policy` workflow run failed on pre-fix commit. | https://github.com/workingdoge/premath/actions/runs/22272316140 |
| 2026-02-22 | Fix branch and PR created for checker fallback/admin-bypass hardening. | https://github.com/workingdoge/premath/pull/8 |
| 2026-02-22 | `branch-policy` workflow passed after checker fix on PR branch. | https://github.com/workingdoge/premath/actions/runs/22272381740 |
