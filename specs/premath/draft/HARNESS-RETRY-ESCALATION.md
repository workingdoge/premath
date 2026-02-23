---
slug: draft
shortname: HARNESS-RETRY-ESCALATION
title: workingdoge.com/premath/HARNESS-RETRY-ESCALATION
name: Harness Retry and Escalation Contract
status: draft
category: Standards Track
tags:
  - premath
  - harness
  - retry
  - escalation
  - control-plane
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Purpose and authority boundary

This specification defines one canonical classify/retry/escalate contract for
provider-neutral harness execution wrappers.

This policy classifies operational outcomes only. It MUST NOT authorize
semantic admissibility. Semantic authority remains in checker/discharge
surfaces (`draft/BIDIR-DESCENT.md`, `draft/GATE.md`).

## 2. Canonical policy artifact and binding

Canonical policy artifact:

- `policies/control/harness-retry-policy-v1.json`

Canonical policy kind:

- `ci.harness.retry.policy.v1`

Digest discipline:

- `policyDigest` is
  `pol1_<sha256(canonical-policy-payload-without-policyDigest)>`.
- Wrappers MUST fail closed on digest mismatch.

## 3. Rule model

Each retry rule binds:

- one set of `failureClasses`,
- one deterministic `maxAttempts`,
- one typed `backoffClass`,
- one terminal `escalationAction`:
  `issue_discover | mark_blocked | stop`.

Backoff class is a typed label, not wall-clock authority. Replay semantics MUST
remain deterministic.

## 4. Canonical v1 decision table

The policy MUST include deterministic rule classes equivalent to:

- `transient_retry`
  - failures include transient executor/timeout/flaky classes,
  - `maxAttempts=3`, `backoffClass=exponential_short`,
  - `escalationAction=issue_discover`.
- `operational_retry`
  - failures include witness/runtime-shape pipeline classes,
  - `maxAttempts=2`, `backoffClass=fixed_short`,
  - `escalationAction=issue_discover`.
- `semantic_no_retry`
  - failures include check/proposal/policy semantic classes,
  - `maxAttempts=1`, `backoffClass=none`,
  - `escalationAction=mark_blocked`.
- default
  - `maxAttempts=1`, `backoffClass=none`,
  - `escalationAction=stop`.

Class membership is policy data. Wrappers MUST treat policy rows as canonical
and MUST NOT hardcode independent rule tables.

## 5. Enforcement path and wrappers

Shared helper:

- `tools/ci/harness_retry_policy.py`

Wrapper surfaces:

- `tools/ci/pipeline_required.py`
- `tools/ci/pipeline_instruction.py`

Wrapper requirements:

1. execute underlying gate step once,
2. on failure, classify from deterministic process output plus witness surfaces,
3. decide retry/escalation from canonical policy artifact,
4. append deterministic retry history summary,
5. fail closed on policy load/classification/context errors.

## 6. Escalation mutation mapping

Escalation mutation bridge:

- `tools/ci/harness_escalation.py`

Terminal action mapping:

- `issue_discover` -> `premath issue discover <active-issue-id> ...`
- `mark_blocked` -> `premath issue update <active-issue-id> --status blocked --notes ...`
- `stop` -> no mutation

Issue context resolution order MUST be deterministic:

1. `PREMATH_ACTIVE_ISSUE_ID`
2. `PREMATH_ISSUE_ID`
3. harness-session artifact `issueId`:
   `PREMATH_HARNESS_SESSION_PATH` override else
   `.premath/harness_session.json`
4. issue-memory ready frontier when exactly one row exists.

Fail-closed context classes include:

- `escalation_issue_context_unbound`
- `escalation_issue_context_ambiguous`
- `escalation_session_invalid`
- `escalation_session_read_failed`

Mutation command failure MUST remain fail-closed with non-success exit.

## 7. Doctrine-site routing note

This spec reuses existing routed operations in
`draft/DOCTRINE-OP-REGISTRY.json` and `draft/DOCTRINE-SITE.json`.
It MUST NOT introduce parallel mutation authority surfaces.

Escalation routes include:

- `op/mcp.issue_discover`
- `op/mcp.issue_lease_projection`
- `op/mcp.dep_diagnostics`

When escalation writes issue status/notes through mutation tooling, those writes
remain subject to routed mutation policy in:

- `op/mcp.issue_update`
- `op/mcp.issue_claim`
- `op/mcp.issue_lease_renew`
- `op/mcp.issue_lease_release`

## 8. Verification surfaces

Minimum deterministic verification:

- `python3 tools/ci/test_harness_retry_policy.py`
- `python3 tools/ci/test_harness_escalation.py`
- `python3 tools/ci/test_pipeline_required.py`
- `python3 tools/ci/test_pipeline_instruction.py`
- `mise run ci-pipeline-test`
- `mise run doctrine-check`

## 9. Related surfaces

- design docs:
  - `docs/design/TUSK-HARNESS-RETRY-POLICY.md`
  - `docs/design/TUSK-HARNESS-CONTRACT.md`
- index/authority overlays:
  - `draft/SPEC-INDEX.md`
  - `draft/UNIFICATION-DOCTRINE.md`
