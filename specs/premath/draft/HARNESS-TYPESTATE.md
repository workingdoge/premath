---
slug: draft
shortname: HARNESS-TYPESTATE
title: workingdoge.com/premath/HARNESS-TYPESTATE
name: Harness Tool-Calling Typestate Contract
status: draft
category: Standards Track
tags:
  - premath
  - harness
  - typestate
  - tool-calling
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

This specification defines deterministic typestate closure and mutation
admissibility for tool-calling harness turns.

It provides normative authority for runtime closure gating. Design discussion in
`docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md` is non-normative commentary.

Semantic admissibility remains in checker/kernel doctrine (`draft/GATE`,
`draft/BIDIR-DESCENT`, `draft/LLM-INSTRUCTION-DOCTRINE`).

Shared harness-surface partitioning and command-route authority are declared
once in `draft/HARNESS-RUNTIME` §1.1. This spec defines only typestate closure
and mutation-admissibility semantics for tool-calling turns.

## 2. Canonical typestate chain

A conforming runtime MUST evaluate one turn through the ordered chain:

`CallSpec -> ToolRequests -> ToolResults -> ToolUse -> JoinClosed -> MutationReady`

`MutationReady` MUST be fail-closed. If unsatisfied, projection artifacts MAY be
written, but issue/dependency mutation operations MUST NOT execute.

## 3. Typed evidence contract

### 3.1 `CallSpec` minimum binding fields

A conforming turn `CallSpec` MUST bind at least:

- `callId`
- `modelRef`
- `actionMode` (`code|json|text`)
- `executionPattern`
- `normalizerId`
- `mutationPolicyDigest`
- `governancePolicyDigest`
- `toolRenderProtocolDigest`
- `reminderQueuePolicyDigest`
- `stateViewPolicyDigest`
- `decompositionPolicyDigest`

### 3.2 Tool/protocol/handoff/context evidence

A conforming turn payload MUST carry deterministic rows for:

- `toolRequests` keyed by `toolCallId`
- terminal `toolResults` keyed by `toolCallId`
- `toolUse` dispositions keyed by `toolCallId`
- protocol state (`stopReason`, continuation allowance)
- optional handoff packet (target + required artifacts + return path)
- context reconstruction evidence (`toolRender`, `reminderQueue`, `stateViews`)

`ToolResults` error rows MUST include a machine-readable envelope:

- `errorCode`
- `retryable`
- `errorMessage` (or equivalent digestable field)

Missing typed envelope fields on error rows MUST fail closed.

## 4. Deterministic normalization contract

Normalization output MUST be deterministic and kinded as:

- `premath.harness.typestate_normalized.v1`

Normalization MUST produce stable digest bindings for:

- call spec
- request/result/use sets
- tool-render/reminder-queue/state-view sets
- protocol state
- optional handoff packet
- join set

Equivalent semantic inputs MUST produce byte-stable digest outputs.

## 5. `JoinClosed` contract

`JoinClosed` MUST be true only when all hold:

- every requested `toolCallId` has a terminal result row,
- no orphan result rows exist,
- every terminal result has typed `ToolUse` disposition evidence,
- no `ToolUse` row references an unknown/non-terminal result,
- stop reason is admitted by protocol policy,
- required context reconstruction is present and policy-valid before continuing
  iterative loops.

When `executionPattern` implies decomposition or transfer, the runtime MUST
additionally enforce decomposition admissibility and handoff artifact/target/
return-path constraints before closure.

## 6. `MutationReady` contract

`MutationReady` MUST require:

- `JoinClosed = true`,
- context continuation readiness when iterative continuation is in scope,
- active mutation policy digest binding,
- deterministic witness linkage for the turn trajectory/session surfaces.

Instruction-linked mutation surfaces MUST reject mutation when closure evidence
is missing, including missing join-gate witness linkage.

## 7. Action-mode typing

- `code` mode MAY emit multiple tool invocations in one action; closure still
  applies over the full turn graph.
- `json` mode SHOULD emit one structured invocation per action object.
- `text` mode MUST NOT invoke tools from free text directly; if tool execution
  is attempted, text MUST first be normalized through a deterministic adapter
  profile into structured tool-use blocks.

## 8. Fail-closed classes

A conforming implementation MUST emit deterministic, machine-readable
failure-class strings for typestate gate failures.

Minimum typestate classes:

- `tool.schema_invalid`
- `tool.result_missing`
- `tool.result_orphan`
- `tool.use_missing`
- `tool.use_without_result`
- `tool.join_incomplete`
- `tool.response_truncation_policy_violation`
- `protocol.stop_reason_unhandled`
- `protocol.parallel_transport_order_invalid`
- `context.injection_point_missing`
- `context.queue_policy_violation`
- `coordination.decomposition_policy_violation`
- `handoff.required_artifact_missing`
- `handoff.target_not_allowed`
- `handoff.return_path_missing`
- `mutation.use_evidence_missing`

Claim-gated governance provenance classes (when
`profile.doctrine_inf_governance.v0` is claimed):

- `governance.claim_id_invalid`
- `governance.policy_package_unpinned`
- `governance.policy_package_mismatch`

Additional governance classes are defined in `draft/DOCTRINE-INF` and remain
claim-gated through `draft/CONFORMANCE`.

## 9. Canonical command and vector anchors

Conforming repositories MUST provide deterministic executable surfaces for this
contract. Canonical surfaces in this repository are:

- `premath harness-join-check --input <json> --json`
- `python3 tools/conformance/run_harness_typestate_vectors.py`
- `mise run conformance-run`
- `python3 tools/ci/check_issue_graph.py`

## 10. Related surfaces

- `draft/HARNESS-RUNTIME`
- `draft/HARNESS-RETRY-ESCALATION`
- `draft/DOCTRINE-INF`
- `draft/CONFORMANCE`
- `docs/design/TOOL-CALLING-HARNESS-TYPESTATE.md` (non-normative commentary)
