# CI Provider Bindings

Status: draft
Scope: design-level, non-normative

This document maps provider-specific CI controls to the provider-agnostic
Premath gate contract.

## Provider-Agnostic Contract

Required semantic surfaces:

- `sh tools/ci/run_task.sh ci-required`
- `sh tools/ci/run_task.sh ci-required-attested`

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
- provider ref adapter:
  `tools/ci/pipeline_required.py` maps GitHub environment refs into
  `PREMATH_CI_*` internally before running the attested required-gate chain.

Strict-delta verification uses provider-neutral `PREMATH_CI_*` refs after the
pipeline has applied provider ref mapping.

## Other Providers

Future provider mappings (GitLab, Buildkite, Jenkins, self-hosted orchestration)
should bind provider-required checks to the same canonical decision surface:
`sh tools/ci/run_task.sh ci-required-attested`.
