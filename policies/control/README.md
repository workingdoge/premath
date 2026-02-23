# Harness Retry Policy Registry

Harness retry policy artifacts are canonical classify/retry/escalate tables for
provider-neutral pipeline wrappers.

Each retry policy JSON artifact MUST include:

- `schema` (`1`)
- `policyKind` (`ci.harness.retry.policy.v1`)
- `policyId` (human-stable identifier)
- `defaultRule` (`maxAttempts`, `backoffClass`, `escalationAction`)
- `rules` (ordered failure-class rules with unique `ruleId` and
  non-overlapping `failureClasses`)
- `policyDigest` (`pol1_<sha256(canonical-policy-payload)>`)

`policyDigest` is computed over the canonical payload without `policyDigest`
itself.

Pipeline wrappers must treat the policy artifact as authoritative and fail
closed if the digest or schema is invalid.
