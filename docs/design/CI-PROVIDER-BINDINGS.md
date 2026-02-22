# CI Provider Bindings

Status: draft
Scope: design-level, non-normative

This document maps provider-specific CI controls to the provider-agnostic
Premath gate contract.

## Provider-Agnostic Contract

Required semantic surfaces:

- `mise run ci-required`
- `mise run ci-verify-required-strict`
- `mise run ci-decide-required`
- `mise run ci-verify-decision`

Required evidence surfaces:

- `artifacts/ciwitness/latest-required.json`
- `artifacts/ciwitness/latest-delta.json`
- `artifacts/ciwitness/latest-decision.json`

Required provider-neutral delta refs:

- `PREMATH_CI_BASE_REF` (optional)
- `PREMATH_CI_HEAD_REF` (optional, default `HEAD`)

Any provider binding MUST treat this contract as authoritative and MUST NOT
change admissibility semantics.

## GitHub Binding (Current Repo)

Current workflow file: `.github/workflows/baseline.yml`.

Binding:

- workflow job name: `ci-required`
- required status check in branch protection/rulesets: `ci-required`
- tracked server policy contract:
  `specs/process/GITHUB-BRANCH-POLICY.json` (checked by
  `tools/ci/check_branch_policy.py`)
- adapter step:
  `python3 tools/ci/providers/export_github_env.py >> "$GITHUB_ENV"`

Strict-delta verification uses:

- provider-neutral `PREMATH_CI_*` refs after adapter export via
  `mise run ci-verify-required-strict`

## Other Providers

Future provider mappings (GitLab, Buildkite, Jenkins, self-hosted orchestration)
should bind provider-required checks to the same canonical decision surface:
`mise run ci-decide-required`.
