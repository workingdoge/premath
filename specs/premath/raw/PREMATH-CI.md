---
slug: raw
shortname: PREMATH-CI
title: workingdoge.com/premath/PREMATH-CI
name: Premath CI/CD Control Loop
status: raw
category: Standards Track
tags:
  - premath
  - ci
  - cd
  - devops
  - closure
  - conformance
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

This specification defines a vendor-agnostic **higher-order CI/CD model** for
Premath implementations.

Goal:

- keep kernel semantics invariant across executors/runners,
- model CI/CD as a deterministic control loop over change sets,
- make local coding environment checks and hosted CI checks semantically
  equivalent when bound to the same policy.

This document is an umbrella contract. `raw/CI-TOPOS` defines closure and
projection details.

## 2. Plane and authority split

Conforming implementations MUST preserve this split:

- semantic admissibility: kernel laws (`draft/PREMATH-KERNEL`, `draft/GATE`)
- single-world execution: `raw/TUSK-CORE`
- inter-world transport/composition: `raw/SQUEAK-CORE`
- context lineage source: implementation-defined (`jj`, git, or equivalent)
- gate execution/runtime: hooks, local runner, CI backend

Runner/tooling layers MUST NOT redefine semantic admissibility.

### 2.1 Relationship to `draft/PREMATH-COHERENCE`

`raw/PREMATH-CI` and `draft/PREMATH-COHERENCE` are two roles in one
control-plane layer:

- coherence checker role: deterministic consistency checking over declared
  control-plane surfaces,
- CI role: deterministic execution and attestation transport over required
  checks and witnesses.

Neither role is semantic admissibility authority. Kernel semantic authority
remains in `draft/PREMATH-KERNEL`, `draft/GATE`, and `draft/BIDIR-DESCENT`.

## 3. Control-loop object

Let:

- `Delta` be a repository change set,
- `G(Delta)` be required checks under active CI policy.

A conforming policy SHOULD define `G` via deterministic change projection as in
`raw/CI-TOPOS`.

Equivalent policy bindings MUST produce equivalent `G(Delta)` and equivalent
Gate-class outcomes.

## 4. Trigger/executor neutrality

The same required gate surface MAY be triggered from:

- local commands,
- local hooks,
- JJ aliases or equivalent control-plane commands,
- hosted CI/CD providers.

A conforming implementation MUST treat these as execution surfaces, not distinct
semantic policies.

## 4.1 Executor profile contract

Implementations MAY expose executor profiles (for example `local`,
`microvm-backed`, `remote-worker`).

For fixed semantic inputs and fixed policy bindings:

- executor profile choice MUST NOT change `G(Delta)`,
- executor profile choice MUST NOT change Gate-class outcomes.

Provisioning/startup/transport failures in an executor profile MUST be reported
as execution diagnostics, not reclassified as kernel admissibility failures.

## 4.2 Infra profile contract

Implementations MAY expose infrastructure provisioning profiles (for example
Terraform/OpenTofu-based startup, VM orchestration, remote worker pools).

For fixed semantic inputs and fixed policy bindings:

- infra profile choice MUST NOT change `G(Delta)`,
- infra profile choice MUST NOT change Gate-class outcomes.

Infra profile responsibilities are provisioning/binding only (for example
materializing `executor_runner` targets). They MUST NOT redefine admissibility.
Such profiles MAY be realized as runtime adapters in the operational execution
layer (for example Squeak `Cheese` profiles over `raw/SQUEAK-SITE`).

## 5. Requiredness policy

A repository profile MAY define:

```text
ci_required = true
```

When `ci_required=true`, accept/merge/promote operations MUST fail if any check
in `G(Delta)` fails.

Enforcement mechanism is implementation-defined (server, local gate, hook, CI).

## 6. Evidence and attestation

Implementations SHOULD emit deterministic CI witnesses:

```text
CIWitness {
  ci_schema
  run_id
  delta_ref
  required_checks
  executed_checks
  results
  projection_digest
  policy_digest
  operational_failure_classes?
  semantic_failure_classes?
  failure_classes?
  gate_witness_refs?
}
```

`projection_digest` binds change-projection semantics.
`policy_digest` binds requiredness/profile policy.
When `operational_failure_classes` and `semantic_failure_classes` are present,
`failure_classes` MUST be their deterministic set-union (for compatibility
consumers that only ingest one surface).
`gate_witness_refs` MAY bind CI outcomes to kernel witness artifacts.
When present, each `gate_witness_ref` SHOULD include provenance
`source in {native, fallback}`.

When an implementation exposes Tusk-local witness envelopes, `gate_witness_refs`
SHOULD reference those envelope artifacts (for example GateWitnessEnvelope IDs
or content-addressed refs), instead of duplicating admissibility payloads.

### 6.2 Required witness verification (strict CI mode)

For projection-driven required gates (for example `ci.required` witness records):

- implementations MUST provide deterministic witness verification that
  recomputes projection semantics from the witness `changed_paths`,
- verification MUST reject if any of the following diverge from recomputed
  semantics:
  - `projection_digest`
  - `required_checks`
  - `executed_checks`
  - verdict/failure-class consistency with check results.
  - if failure-lineage split fields are present:
    - `operational_failure_classes` consistency with execution-surface outcomes,
    - `semantic_failure_classes` consistency with linked gate witness payload
      classes when available,
    - `failure_classes` consistency with deterministic union of the two.
- when `gate_witness_refs` are present, verification MUST also reject on
  linkage mismatch:
  - check/ref ordering mismatch,
  - missing or invalid provenance `source`,
  - referenced gate payload digest mismatch,
  - referenced gate payload verdict inconsistency with recorded check result.

Implementations MAY define `native_required_checks` policy bindings.
When configured, verification MUST reject if any listed check has
`gate_witness_ref.source != native`.

When CI is operating in strict delta-compare mode, verification MUST also
compare witness `changed_paths` to the CI-evaluated delta for the active base/head
refs and reject on mismatch.

Implementations SHOULD surface verified witness artifacts and digests as CI
attestation outputs for audit.

Implementations SHOULD expose one deterministic decision surface
(`accept|reject`) derived from verified required witnesses, independent of CI
vendor.

### 6.1 Instruction-envelope control loop (v0)

Implementations MAY expose instruction envelopes as first-class CI inputs:

```text
CIInstructionEnvelope {
  intent
  scope
  instruction_type?   // optional explicit typed kind
  typing_policy?      // optional unknown-routing policy
  normalizer_id
  policy_digest
  capability_claims?  // optional action-surface capability claims
  requested_checks
}
```

For deterministic witnessing, implementations SHOULD bind CI witnesses to
instruction identity material (for example `instruction_id` + digest over
canonical envelope payload).

For fixed envelope content and fixed policy bindings, verdict class and
required/executed check sets SHOULD be stable.

Requested checks SHOULD be validated as
`requested_checks ⊆ allowed_checks(policy_digest, scope)` and rejected
deterministically on violation.
Implementations SHOULD bind `policy_digest` to canonical policy-artifact digests
for reproducible allowlist provenance.

Implementations exposing this flow SHOULD also emit instruction classification
material (`typed(kind)` or `unknown(reason)`) in CI witness records.
When proposal payloads are present, implementations SHOULD also emit
`proposalIngest.obligations[]` and deterministic normalized `proposalIngest.discharge`
records so acceptance is auditably discharge-determined.
If envelope validation fails pre-execution, implementations SHOULD still emit a
typed reject witness artifact with deterministic `failure_classes`.

Doctrine typing/binding constraints for this flow are specified in
`draft/LLM-INSTRUCTION-DOCTRINE`.

## 7. Invariance requirements

For fixed semantic inputs and fixed policy bindings:

- Gate class outcomes MUST be invariant across executor surfaces.
- Optional evidence profiles MAY change representation but MUST NOT change
  kernel-level admissibility class.
- destination admissibility MUST still hold after any transport handoff
  (`raw/SQUEAK-CORE` non-bypass rule).

## 8. Operational profiles (v0 guidance)

Implementations MAY expose policy profiles such as:

- local-fast gate (hygiene/fix profile),
- local-full closure gate,
- required CI gate.

Profile names are implementation-defined, but mapping to `G(Delta)` MUST remain
deterministic and auditable.

## 9. Security and robustness

Implementations MUST treat repository content, fixtures, and spec artifacts as
untrusted.

Implementations SHOULD:

- fail closed on missing required checks,
- pin tool versions/configs for replayability,
- keep check identifiers stable across runner backends,
- retain CI witness logs for diagnosis and audit.

## 10. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.profile.execution` (executor/infra profile neutrality)
- `dm.profile.evidence` (representation/profile changes preserve kernel class)
- `dm.commitment.attest` (CI witness bindings and deterministic attestation)
- `dm.presentation.projection` (trigger-surface neutrality for fixed policy)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.refine.context` / `dm.refine.cover` (handled by kernel/runtime layer)
