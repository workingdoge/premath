---
slug: draft
shortname: LLM-INSTRUCTION-DOCTRINE
title: workingdoge.com/premath/LLM-INSTRUCTION-DOCTRINE
name: LLM Instruction Doctrine
status: draft
category: Standards Track
tags:
  - premath
  - llm
  - instruction
  - doctrine
  - ci
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

## 1. Scope

This specification defines doctrine-level constraints for LLM instruction
surfaces in Premath-first runtimes.

Goal:

- model LLM instructions as typed operational inputs,
- keep semantic admissibility authority in kernel/runtime layers,
- require deterministic instruction identity and witness binding.

This specification complements:

- `draft/DOCTRINE-INF` (morphism registry),
- `draft/DOCTRINE-SITE` (doctrine-to-operation map),
- `draft/LLM-PROPOSAL-CHECKING` (proposal ingestion/checking contract),
- `raw/PREMATH-CI` (CI control-loop execution contract).

Design map reference (non-normative):

- `../../../docs/design/ARCHITECTURE-MAP.md` (doctrine-to-operation path summary).

## 2. Core objects

An instruction envelope is modeled as:

```text
InstructionEnvelope {
  intent
  scope
  normalizer_id
  policy_digest
  capability_claims?
  requested_checks
}
```

Equivalent field names MAY be used in implementation payloads, but this shape is
the semantic contract.

Repository profile note (v0): this repository currently uses camelCase fields
in instruction JSON (`normalizerId`, `policyDigest`, `requestedChecks`).
It may additionally carry `instructionType` and `typingPolicy.allowUnknown`.
When proposal material is embedded inline, repository v0 uses `proposal`
(legacy alias `llmProposal` is accepted for compatibility).
`policyDigest` is bound to canonical policy artifacts in
`policies/instruction/*.json` and uses `pol1_<sha256(...)>` identity.

For deterministic binding, implementations SHOULD derive:

```text
instruction_id     // stable run/control-plane ID
instruction_digest // canonical digest over envelope payload
```

## 3. Instruction typing and unknown classification

Instruction typing MUST be explicit.

A runtime MUST classify instruction handling into one of:

- `typed(kind)`: recognized instruction kind under active policy,
- `unknown(reason)`: classification not established.

`unknown(reason)` is first-class and MUST NOT be silently coerced into
execution.

Policy MAY define allowed transitions from `unknown(reason)` (for example:
clarify-only, plan-only, escalation-required), but this policy MUST be explicit
and auditable.

## 4. Authority split

LLM layers MAY propose:

- instruction typing,
- candidate check sets,
- candidate plans/refinements.

LLM layers MUST NOT self-authorize admissibility or gate outcomes.

Authority remains split as:

- kernel/runtime (`draft/PREMATH-KERNEL`, `draft/GATE`, `raw/TUSK-CORE`):
  admissibility and Gate-class outcomes,
- Squeak/runtime-location layer (`raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`):
  transport/location execution,
- instruction doctrine layer (this spec):
  typing, binding, and auditable transition constraints.

## 5. Doctrine path to operation

Instruction operation SHOULD follow this chain:

```text
envelope -> classify -> bind(normalizer_id, policy_digest)
-> project(allowed_checks(policy_digest, scope))
-> execute(check runner) -> attest(CIWitness)
```

When an instruction carries or references LLM proposal material, operation SHOULD
extend as:

```text
envelope -> classify -> bind(normalizer_id, policy_digest)
-> proposal_ingest(checking-only) -> obligations -> discharge
-> project(allowed_checks(policy_digest, scope))
-> execute(check runner) -> attest(CIWitness)
```

The chain MUST preserve:

- explicit policy bindings,
- explicit normalizer bindings,
- explicit requested-check allowlist bounds under active policy,
- deterministic check ID sets,
- deterministic verdict-class attribution for fixed inputs/bindings.

Operational scripts (for example `tools/ci/run_instruction.sh`) are execution
surfaces only. They do not define semantic admissibility.

## 6. Determinism and attestation requirements

For fixed envelope payload and fixed policy bindings:

- `instruction_digest` MUST be deterministic,
- required/executed check sets MUST be deterministic up to canonical ordering,
- witness verdict class and failure classes MUST be deterministic.

When witness records are emitted, they SHOULD include:

- instruction identity material (`instruction_id`, `instruction_digest`),
- normalizer binding (`normalizer_id`),
- policy binding (`policy_digest`),
- capability claims (`capability_claims`) when policy-scoped action surfaces
  are enforced,
- required/executed checks,
- verdict class and failure classes.

When instruction witnesses expose split failure-lineage fields, they SHOULD use:

- `operational_failure_classes` for control-plane execution failures,
- `semantic_failure_classes` for proposal/gate semantic lineage where available,
- `failure_classes` as deterministic union for compatibility consumers.

When proposal material is present, witnesses SHOULD additionally include:

- deterministic proposal KCIR ref (`proposal_kcir_ref`) when KCIR-linked
  witness surfaces are enabled,
- deterministic compiled `obligations[]`,
- deterministic normalized `discharge` result,
- discharge failure classes (if any), bound to `(normalizer_id, policy_digest)`.

## 7. Conformance expectations

Implementations exposing instruction-envelope control loops SHOULD:

- reject malformed envelopes deterministically,
- reject duplicate check identifiers deterministically,
- reject `requested_checks` outside policy-bound allowlists deterministically,
- emit first-class pre-execution reject witnesses with deterministic
  `failure_classes` when envelope/policy/proposal validation fails,
- emit auditable CI witness artifacts bound to instruction identity material,
- keep instruction flow compatible with `raw/PREMATH-CI` invariance rules.

## 8. Security and robustness

Implementations MUST treat instruction payloads as untrusted input.

Implementations SHOULD:

- fail closed on missing required envelope fields,
- pin policy binding material in witness output,
- retain instruction-to-witness lineage logs for audit.

## 9. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.policy.rebind` (explicit rebinding requires explicit new instruction/run boundary)
- `dm.profile.execution` (executor choice does not alter typed instruction meaning)
- `dm.presentation.projection` (NL/UI/API presentation does not alter authority)
- `dm.commitment.attest` (instruction to witness binding is deterministic and auditable)

Not preserved:

- `dm.transport.world` / `dm.transport.location` (delegated to Squeak layer)
- `dm.refine.context` / `dm.refine.cover` (delegated to kernel/runtime layer)
- `dm.profile.evidence` (delegated to capability profile contracts in conformance/CI specs)
