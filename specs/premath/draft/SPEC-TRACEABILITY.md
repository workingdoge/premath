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
| `DOCTRINE-INF.md` | `mise run doctrine-check` (declaration-set + edge coherence + reachability + doctrine-inf semantic boundary vectors) | covered | - |
| `PREMATH-KERNEL.md` | `python3 tools/conformance/run_kernel_profile_vectors.py`; `cargo test -p premath-kernel`; `mise run test-toy`; `mise run test-kcir-toy` | covered | - |
| `KCIR-CORE.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`kcir_domain_table_*`) | covered | - |
| `REF-BINDING.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`ref_projection_and_verify_*`) | covered | - |
| `NF.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`nf_*`) + `capabilities.normal_forms` + kernel tests | covered | - |
| `WIRE-FORMATS.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`wire_*`) | covered | - |
| `ERROR-CODES.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`error_code_registry_*`) | covered | - |
| `WITNESS-ID.md` | `python3 tools/conformance/run_witness_id_vectors.py`; `premath-kernel` witness-id unit tests | covered | - |
| `BIDIR-DESCENT.md` | `capabilities.instruction_typing`; `capabilities.adjoints_sites` | covered | - |
| `GATE.md` | `python3 tools/conformance/run_gate_vectors.py` + `premath-kernel` gate tests + toy vectors | covered | - |
| `CONFORMANCE.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_interop_core_vectors.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CAPABILITY-VECTORS.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CHANGE-MORPHISMS.md` | `capabilities.change_morphisms` vectors | covered | - |
| `DOCTRINE-SITE.md` | `mise run doctrine-check` | covered | - |
| `DOCTRINE-SITE.json` | `mise run doctrine-check` | covered | - |
| `LLM-INSTRUCTION-DOCTRINE.md` | `capabilities.instruction_typing`; `capabilities.ci_witnesses`; `mise run ci-pipeline-test` | covered | - |
| `LLM-PROPOSAL-CHECKING.md` | `capabilities.instruction_typing`; `tools/ci/test_instruction_policy.py`; `tools/ci/test_instruction_reject_witness.py` | covered | - |
| `PREMATH-COHERENCE.md` | `mise run coherence-check`; `cargo test -p premath-coherence`; `coherence-check` CLI smoke test | covered | - |
| `COHERENCE-CONTRACT.json` | `mise run coherence-check`; `coherence-check` CLI smoke test | covered | - |
| `SPEC-INDEX.md` | `python3 tools/conformance/check_spec_traceability.py` | covered | - |
| `SPEC-TRACEABILITY.md` | `python3 tools/conformance/check_spec_traceability.py` | covered | - |

## 4. Coverage Targets (Open Gaps/Upgrades)

No open coverage targets currently.

## 5. Maintenance Rules

- Every promoted draft spec MUST have exactly one matrix row in this document.
- `gap` rows MUST reference a concrete target ID.
- Coverage target implementation work SHOULD be tracked as discovered issues
  linked from the active traceability issue chain.
