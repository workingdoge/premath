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

Required evidence surfaces:

- `artifacts/ciwitness/latest-required.json`
- `artifacts/ciwitness/latest-decision.json`

Any provider binding MUST treat this contract as authoritative and MUST NOT
change admissibility semantics.

## GitHub Binding (Current Repo)

Current workflow file: `.github/workflows/baseline.yml`.

Binding:

- workflow job name: `ci-required`
- required status check in branch protection/rulesets: `ci-required`

Strict-delta verification uses:

- `GITHUB_BASE_REF` when available (fallback `main`) via
  `mise run ci-verify-required-strict`

## Other Providers

Future provider mappings (GitLab, Buildkite, Jenkins, self-hosted orchestration)
should bind provider-required checks to the same canonical decision surface:
`mise run ci-decide-required`.
