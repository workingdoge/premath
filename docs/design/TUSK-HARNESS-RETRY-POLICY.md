# Tusk Harness Retry Policy

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one canonical classify/retry/escalate table for provider-neutral harness
execution wrappers:

- `tools/ci/pipeline_required.py`
- `tools/ci/pipeline_instruction.py`

Authority boundary remains unchanged:

- policy classifies operational behavior only,
- semantic authority remains checker/discharge/witness.

## 2. Canonical policy surface

Canonical artifact:

- `policies/control/harness-retry-policy-v1.json`

Registry notes:

- `policies/control/README.md`

Policy kind:

- `ci.harness.retry.policy.v1`

Digest discipline:

- `policyDigest` uses `pol1_<sha256(canonical-policy-payload-without-policyDigest)>`
- wrappers fail closed on digest mismatch.

## 3. Rule model

Each rule binds:

- failure classes (set),
- `maxAttempts` (deterministic retry budget),
- `backoffClass` (typed schedule label),
- `escalationAction` (`issue_discover` | `mark_blocked` | `stop`).

Backoff is represented as a typed class (not sleep timings) so CI behavior
remains deterministic/replayable while still expressing intended retry shape.

## 4. v1 rule table

`transient_retry`
- failure classes: `executor_unavailable`, `gate_timeout`, `network_timeout`,
  `flaky_execution`, `flaky_io`
- decision: `maxAttempts=3`, `backoffClass=exponential_short`,
  `escalationAction=issue_discover`

`operational_retry`
- failure classes: `pipeline_missing_witness`, `pipeline_invalid_witness_json`,
  `pipeline_invalid_witness_shape`, `required_witness_runtime_invalid`,
  `instruction_runtime_invalid`
- decision: `maxAttempts=2`, `backoffClass=fixed_short`,
  `escalationAction=issue_discover`

`semantic_no_retry`
- failure classes: `check_failed`, `instruction_check_not_allowed`,
  `instruction_invalid_normalizer`, `instruction_unknown_unroutable`,
  `proposal_binding_mismatch`, `proposal_discharge_failed`,
  `proposal_invalid_step`, `proposal_nondeterministic`,
  `proposal_unbound_policy`
- decision: `maxAttempts=1`, `backoffClass=none`,
  `escalationAction=mark_blocked`

Default:

- decision: `maxAttempts=1`, `backoffClass=none`, `escalationAction=stop`

## 5. Enforcement path

Shared helper:

- `tools/ci/harness_retry_policy.py`

Helper responsibilities:

- validate policy schema + digest,
- parse witness failure classes from deterministic artifacts,
- return one typed decision per attempt.

Wrapper contract:

- run once,
- if success: return success,
- if failure: classify from witness -> decide retry or escalate,
- if retry allowed: run next attempt,
- if not: return failing exit code and surface escalation action.

Both wrappers append deterministic retry history to markdown summary output.

## 6. Verification commands

- `python3 tools/ci/test_harness_retry_policy.py`
- `python3 tools/ci/test_pipeline_required.py`
- `python3 tools/ci/test_pipeline_instruction.py`
- `mise run ci-pipeline-test`
