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
| `DOCTRINE-INF.md` | `mise run doctrine-check` (declaration-set + edge coherence + reachability + doctrine-inf semantic boundary vectors + claim-gated governance-profile vectors for policy provenance pin/mismatch, staged guardrails, eval gate + lineage evidence, observability/risk-tier policy, and self-evolution declaration bounds) | covered | - |
| `PREMATH-KERNEL.md` | `python3 tools/conformance/run_kernel_profile_vectors.py`; `cargo test -p premath-kernel`; `mise run test-toy`; `mise run test-kcir-toy` | covered | - |
| `KCIR-CORE.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`kcir_domain_table_*`) | covered | - |
| `REF-BINDING.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`ref_projection_and_verify_*`) | covered | - |
| `NF.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`nf_*`) + `capabilities.normal_forms` + kernel tests | covered | - |
| `NORMALIZER.md` | `capabilities.normal_forms`; `python3 tools/conformance/run_interop_core_vectors.py` (`nf_*`) + normalized comparison checks in `capabilities.instruction_typing` | covered | - |
| `WIRE-FORMATS.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`wire_*`) | covered | - |
| `ERROR-CODES.md` | `python3 tools/conformance/run_interop_core_vectors.py` (`error_code_registry_*`) | covered | - |
| `WITNESS-ID.md` | `python3 tools/conformance/run_witness_id_vectors.py`; `premath-kernel` witness-id unit tests | covered | - |
| `BIDIR-DESCENT.md` | `capabilities.instruction_typing`; `capabilities.adjoints_sites` | covered | - |
| `GATE.md` | `python3 tools/conformance/run_gate_vectors.py` + `premath-kernel` gate tests + toy vectors | covered | - |
| `CONFORMANCE.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_interop_core_vectors.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CAPABILITY-VECTORS.md` | `python3 tools/conformance/check_stub_invariance.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CHANGE-MORPHISMS.md` | `capabilities.change_morphisms` vectors | covered | - |
| `DOCTRINE-SITE.md` | `mise run doctrine-check` (site roundtrip/reachability + runtime orchestration route checker + MCP doctrine-operation parity + doctrine-inf vectors) | covered | - |
| `DOCTRINE-SITE.json` | `mise run doctrine-check` (site roundtrip/reachability + runtime orchestration route checker + MCP doctrine-operation parity + doctrine-inf vectors) | covered | - |
| `DOCTRINE-SITE-INPUT.json` | `mise run doctrine-check`; `python3 tools/conformance/generate_doctrine_site.py --check` | covered | - |
| `DOCTRINE-OP-REGISTRY.json` | `mise run doctrine-check`; `python3 tools/conformance/generate_doctrine_site.py --check`; `python3 tools/conformance/run_runtime_orchestration_vectors.py` | covered | - |
| `HARNESS-RUNTIME.md` | `cargo test -p premath-cli`; `python3 tools/conformance/run_harness_typestate_vectors.py`; `python3 tools/conformance/check_runtime_orchestration.py` (runtime route presence/morphism coverage + routed CI path boundary + optional `controlPlaneKcirMappings` row-shape checks + phase-3 command-surface parity rows for `governancePromotionCheck`/`kcirMappingCheck`); `python3 tools/conformance/run_runtime_orchestration_vectors.py` (golden/adversarial + invariance profile-permutation vectors, including phase-3 command-surface vectors); `python3 tools/ci/check_issue_graph.py`; `mise run docs-coherence-check` | covered | - |
| `HARNESS-TYPESTATE.md` | `cargo test -p premath-tusk`; `cargo test -p premath-cli`; `python3 tools/conformance/run_harness_typestate_vectors.py`; `python3 tools/ci/check_issue_graph.py` | covered | - |
| `HARNESS-RETRY-ESCALATION.md` | `python3 tools/ci/test_harness_retry_policy.py`; `python3 tools/ci/test_harness_escalation.py`; `mise run ci-pipeline-test`; `mise run doctrine-check` | covered | - |
| `LLM-INSTRUCTION-DOCTRINE.md` | `capabilities.instruction_typing`; `capabilities.ci_witnesses`; `mise run ci-pipeline-test` | covered | - |
| `LLM-PROPOSAL-CHECKING.md` | `capabilities.instruction_typing`; `tools/ci/test_instruction_check_client.py`; `tools/ci/test_instruction_reject_witness.py` | covered | - |
| `PREMATH-COHERENCE.md` | `mise run coherence-check`; `cargo test -p premath-coherence`; `coherence-check` CLI smoke test | covered | - |
| `COHERENCE-CONTRACT.json` | `mise run coherence-check`; `coherence-check` CLI smoke test | covered | - |
| `CAPABILITY-REGISTRY.json` | `python3 tools/conformance/check_docs_coherence.py`; `python3 tools/conformance/run_capability_vectors.py`; `mise run coherence-check` | covered | - |
| `CONTROL-PLANE-CONTRACT.json` | `mise run coherence-check`; `mise run ci-pipeline-test`; `python3 tools/ci/test_control_plane_contract.py`; `python3 tools/ci/test_run_required_checks.py`; `python3 tools/ci/test_governance_gate.py`; `python3 tools/ci/test_kcir_mapping_gate.py` | covered | - |
| `UNIFICATION-DOCTRINE.md` | `mise run docs-coherence-check`; `mise run coherence-check` (`gate_chain_parity` Stage 1+Stage 2 parity/rollback/authority checks + Stage 2 direct bidir-evidence-route checks + Stage 3 typed-first closure mapping checks); `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract` (`gate_chain_parity_stage1_*` + `gate_chain_parity_stage2_*` vectors); `python3 tools/conformance/run_capability_vectors.py --capability capabilities.ci_witnesses` (boundary-authority lineage + obstruction roundtrip vectors); decision-log traceability via `check_spec_traceability.py` (Decisions 0106-0110) | covered | - |
| `SPAN-SQUARE-CHECKING.md` | `mise run coherence-check` (`span_square_commutation` via site vectors, including composition-law vectors); `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract` | covered | - |
| `SPEC-INDEX.md` | `python3 tools/conformance/check_spec_traceability.py` | covered | - |
| `SPEC-TRACEABILITY.md` | `python3 tools/conformance/check_spec_traceability.py` | covered | - |

## 4. Coverage Targets (Open Gaps/Upgrades)

No open coverage targets currently.

## 5. Maintenance Rules

- Every promoted draft spec MUST have exactly one matrix row in this document.
- `gap` rows MUST reference a concrete target ID.
- Coverage target implementation work SHOULD be tracked as discovered issues
  linked from the active traceability issue chain.
