---
slug: raw
shortname: CI-TOPOS
title: workingdoge.com/premath/CI-TOPOS
name: CI Closure and Change Projection
status: raw
category: Standards Track
tags:
  - premath
  - ci
  - closure
  - conformance
  - change-projection
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

This specification defines a vendor-agnostic CI model as a closure operator over
repository changes.

This document is the closure/projection specialization under `raw/PREMATH-CI`.

Goal: minimize required checks per change while preserving Premath kernel
invariance and capability conformance.

Design map reference (non-normative):

- `../../../docs/design/ARCHITECTURE-MAP.md` (doctrine-to-operation path summary).

## 2. Closure model

Let:

- `Delta` be a change set (code, specs, fixtures, tooling),
- `P` be a deterministic change projection,
- `G(Delta)` be the required check set.

Define:

```text
G(Delta) = Closure(P(Delta))
```

A conforming closure operator MUST satisfy:

- monotonicity: if `Delta1` subseteq `Delta2`, then `G(Delta1)` subseteq `G(Delta2)`
- idempotence: `G(G(Delta)) = G(Delta)`
- determinism: fixed repo state and `Delta` yield fixed `G(Delta)`

## 3. Required baseline checks (v0)

The v0 baseline check set is:

1. workspace build
2. Rust tests
3. semantic toy vectors
4. KCIR toy vectors
5. conformance stub invariance

Canonical local command:

```bash
just baseline
```

If a convenience task runner is unavailable, implementations MUST execute an
equivalent baseline check set directly.

## 4. Change projection rules (v0)

A v0 projection SHOULD include:

- docs-only changes:
  - run conformance stub checker if `specs/premath/raw/` or
    `tests/conformance/` are touched
- Rust crate changes:
  - run build and Rust tests
  - include toy and KCIR toy vectors when `crates/premath-kernel` is touched
- conformance fixture/schema/tooling changes:
  - run conformance checker and both toy vector suites
- profile/capability semantics changes:
  - run full baseline

Implementations MAY run full baseline for all changes.

## 5. Requiredness policy

This specification is runner-agnostic and host-agnostic.

A repository profile MAY mark CI as required by setting policy equivalent to:

```text
ci_required = true
```

When `ci_required=true`, merge/accept operations MUST reject if any check in
`G(Delta)` fails.

How that rejection is enforced (hooks, server gate, local gate, third-party CI)
is implementation-defined.

## 6. Code-as-data framing

CI inputs and outputs SHOULD be modeled as data projections over repository
state.

Recommended projections:

- source projection (files, dependency graph, changed paths),
- semantic projection (capability claims, spec status transitions),
- check projection (required check identifiers, execution plan, attestation).

This framing enables deterministic replay and portable CI semantics across
runners.

## 7. CI witness and attestation

A conforming runner SHOULD emit deterministic attestation records:

```text
CIWitness {
  ci_schema
  repo_state_ref
  delta_ref
  required_checks
  executed_checks
  results
  projection_digest
  policy_digest
}
```

`projection_digest` MUST bind change-projection semantics used to compute
`G(Delta)`.

For required projected-gate records (for example `ci.required` witnesses),
implementations MUST provide deterministic verification that recomputes
projection semantics and rejects on:

- projection digest mismatch,
- required/executed check-set mismatch,
- verdict/failure-class mismatch with recorded results.
- when `gate_witness_refs` are present:
  - check/ref ordering mismatch,
  - referenced gate payload digest mismatch,
  - referenced gate payload verdict inconsistency with check result.

When strict delta-compare mode is enabled, verification MUST also compare
witness `changed_paths` to the evaluated CI delta for active base/head refs and
reject on mismatch.

Conforming CI implementations SHOULD publish verified witness artifacts and
digest sidecars as attestation outputs.

## 8. Relationship to Tusk and Squeak

CI checks may be executed via Tusk units and transport-verified via Squeak.
CI closure semantics remain independent of specific runtime topology.

No separate “bridge” subsystem is required:

- local admissibility checks belong to Tusk,
- transport plus destination handoff/non-bypass checks belong to Squeak.

CI MUST enforce kernel-level invariants even when optional evidence profiles or
operational variants are used.

## 9. Security and robustness

Implementations MUST treat repository content, spec artifacts, and test fixtures
as untrusted.

Implementations SHOULD:

- fail closed on missing required checks,
- pin tool versions for deterministic replay,
- keep check identifiers stable across runner backends.

## 10. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.profile.execution` (closure semantics independent of runner backend)
- `dm.commitment.attest` (projection/policy digest binding in CI witness records)
- `dm.presentation.projection` (code-as-data projection discipline)

Not preserved:

- `dm.transport.world` / `dm.transport.location` (handled by Squeak layer)
- `dm.refine.context` / `dm.refine.cover` (handled by kernel/runtime layer)
- `dm.profile.evidence` (handled by capability profile contracts in higher CI spec)
