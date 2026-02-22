---
slug: draft
shortname: SPEC-TRACEABILITY
title: workingdoge.com/premath/SPEC-TRACEABILITY
name: Draft Spec Traceability Matrix
status: draft
category: Informational
tags:
  - premath
  - conformance
  - traceability
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Scope

This matrix maps promoted draft specs under `specs/premath/draft/` to their
current executable conformance/check surfaces.

Purpose:

- keep spec coverage auditable from one place,
- classify coverage maturity consistently,
- make unmapped areas explicit as concrete coverage targets.

## 2. Coverage Status Classes

- `covered`: canonical executable vectors/checks exist in merge-gated command
  surfaces (`mise run baseline`, `mise run conformance-run`,
  `mise run doctrine-check`).
- `instrumented`: deterministic checks/tests exist, but no dedicated canonical
  conformance vector suite for the full spec contract.
- `gap`: no dedicated deterministic executable surface for the claimed contract.

## 3. Traceability Matrix (Draft Specs)

| Draft spec | Primary executable surface | Status | Coverage target |
| --- | --- | --- | --- |
| `DOCTRINE-INF.md` | `mise run doctrine-check` (declaration-set + edge coherence + reachability) | instrumented | `T-DINF-01` |
| `PREMATH-KERNEL.md` | `cargo test -p premath-kernel`; `mise run test-toy`; `mise run test-kcir-toy` | instrumented | `T-KERNEL-01` |
| `KCIR-CORE.md` | `capabilities.kcir_witnesses` (reference integrity slices only) | gap | `T-IC-01` |
| `REF-BINDING.md` | `capabilities.kcir_witnesses`; `capabilities.ci_witnesses` (partial) | gap | `T-IC-01` |
| `NF.md` | `capabilities.normal_forms` + kernel tests | instrumented | `T-IC-01` |
| `WIRE-FORMATS.md` | no dedicated vector suite yet | gap | `T-IC-01` |
| `ERROR-CODES.md` | no dedicated vector suite yet | gap | `T-IC-01` |
| `WITNESS-ID.md` | `premath-kernel` witness-id unit tests | instrumented | `T-WID-01` |
| `BIDIR-DESCENT.md` | `capabilities.instruction_typing`; `capabilities.adjoints_sites` | covered | - |
| `GATE.md` | `premath-kernel` gate tests + toy vectors | instrumented | `T-GATE-01` |
| `CONFORMANCE.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CAPABILITY-VECTORS.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CHANGE-MORPHISMS.md` | `capabilities.change_morphisms` vectors | covered | - |
| `DOCTRINE-SITE.md` | `mise run doctrine-check` | covered | - |
| `DOCTRINE-SITE.json` | `mise run doctrine-check` | covered | - |
| `LLM-INSTRUCTION-DOCTRINE.md` | `capabilities.instruction_typing`; `capabilities.ci_witnesses`; `mise run ci-pipeline-test` | covered | - |
| `LLM-PROPOSAL-CHECKING.md` | `capabilities.instruction_typing`; `tools/ci/test_instruction_policy.py`; `tools/ci/test_instruction_reject_witness.py` | covered | - |
| `SPEC-INDEX.md` | index/reference contract; validated indirectly by doc updates and doctrine-site references | instrumented | `T-INDEX-01` |

## 4. Coverage Targets (Current Gaps/Upgrades)

### `T-IC-01` Interop-Core Vectors

Add executable Interop Core suites for:

- `KCIR-CORE`,
- `REF-BINDING`,
- `NF` parser/constructor contracts,
- `WIRE-FORMATS`,
- `ERROR-CODES`.

Minimum acceptance:

- deterministic golden/adversarial vectors under `tests/conformance/fixtures/interop-core/`,
- runner integration into merge-gated conformance command surface.

### `T-GATE-01` Canonical Gate Vectors

Promote `tests/conformance/fixtures/gate/` from placeholder to executable suite.

Minimum acceptance:

- deterministic gate vectors for admissibility/rejection-class outcomes,
- runner integration into merge-gated conformance command surface.

### `T-WID-01` Witness-ID Conformance Vectors

Add fixture-level witness-id determinism vectors (not just unit tests).

Minimum acceptance:

- deterministic witness-id stability/sensitivity vectors in conformance fixtures,
- executable validation path in merge-gated conformance command surface.

### `T-DINF-01` Doctrine-Inf Semantic Coverage Upgrade

Extend beyond declaration/graph coherence into executable law-level checks for
declared preserved/not-preserved boundaries.

### `T-KERNEL-01` Cross-Model Kernel Vector Profile

Define a canonical kernel conformance vector profile for reproducible
cross-model comparison (in addition to host-specific proofs/tests).

### `T-INDEX-01` Index/Lifecycle Integrity Check

Add a deterministic check that all promoted draft specs are present in this
matrix and classified (`covered|instrumented|gap`) with a target when needed.

## 5. Maintenance Rules

- Every promoted draft spec MUST have exactly one matrix row in this document.
- `gap` rows MUST reference a concrete target ID.
- Coverage target implementation work SHOULD be tracked as discovered issues
  linked from the active traceability issue chain.
