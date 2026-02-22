# Instruction Policy Registry

Instruction policy artifacts are canonical allowlist definitions for
instruction-envelope execution.

Each policy JSON artifact MUST include:

- `schema` (`1`)
- `policyKind` (`ci.instruction.policy.v1`)
- `policyId` (human-stable identifier)
- `allowedChecks` (unique check IDs)
- `allowedNormalizers` (unique normalizer IDs)
- `policyDigest` (`pol1_<sha256(canonical-policy-payload)>`)

`policyDigest` is computed over the canonical payload without `policyDigest`
itself.

The instruction checker/runner treat policy artifacts as authoritative policy
surface. Envelope `policyDigest` values MUST reference a registered digest.
