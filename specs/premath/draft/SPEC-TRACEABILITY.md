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

Coverage rows are repository-root claims. They reference executable tools,
fixtures, crates, and CI scripts in a full Premath checkout. A specs-only
release bundle may include this matrix without those executable surfaces; such a
bundle is not self-verifying unless paired with the repository checkout.

## 2. Coverage Status Classes

- `covered`: canonical executable vectors/checks exist in merge-gated command
  surfaces (`sh tools/ci/run_task.sh baseline`, `sh tools/ci/run_task.sh conformance-run`,
  `sh tools/ci/run_task.sh doctrine-check`).
- `instrumented`: deterministic checks/tests exist, but no dedicated canonical
  conformance vector suite for the full spec contract.
- `gap`: no dedicated deterministic executable surface for the claimed contract.

## 3. Traceability Matrix (Draft Specs)

Authority classes are defined in `../AUTHORITY-MAP.json`.

| Draft spec | Authority class | Primary executable surface | Status | Coverage target |
| --- | --- | --- | --- | --- |
| `DOCTRINE-INF.md` | `doctrine-preservation` | `sh tools/ci/run_task.sh doctrine-check` (declaration-set + edge coherence + reachability + doctrine-inf semantic boundary vectors + claim-gated governance-profile vectors for policy provenance pin/mismatch, staged guardrails, eval gate + lineage evidence, observability/risk-tier policy, and self-evolution declaration bounds) | covered | - |
| `PREMATH-KERNEL.md` | `core` | `python3 tools/conformance/run_kernel_profile_vectors.py`; `python3 tools/conformance/check_statement_index.py`; `python3 tools/conformance/run_statement_index_vectors.py`; `python3 tools/conformance/run_statement_kcir_vectors.py`; `cargo test -p premath-kernel`; `cargo test -p premath-kernel --test toy_vectors`; `sh tools/ci/run_task.sh test-kcir-toy` | covered | - |
| `KERNEL-STATEMENT-BINDINGS.json` | `kernel-index` | `python3 tools/conformance/check_statement_bindings.py`; `python3 tools/conformance/run_statement_binding_vectors.py`; `python3 tools/conformance/check_statement_projection_lane.py`; `cargo test -p premath-bd` | covered | - |
| `KCIR-CORE.md` | `interop` | `python3 tools/conformance/run_interop_core_vectors.py` (`kcir_domain_table_*`) | covered | - |
| `REF-BINDING.md` | `interop` | `python3 tools/conformance/run_interop_core_vectors.py` (`ref_projection_and_verify_*`) | covered | - |
| `NF.md` | `interop` | `python3 tools/conformance/run_interop_core_vectors.py` (`nf_*`) + `capabilities.normal_forms` + kernel tests | covered | - |
| `NORMALIZER.md` | `interop` | `capabilities.normal_forms`; `python3 tools/conformance/run_interop_core_vectors.py` (`nf_*`) + normalized comparison checks in `capabilities.instruction_typing` | covered | - |
| `WIRE-FORMATS.md` | `interop` | `python3 tools/conformance/run_interop_core_vectors.py` (`wire_*`) | covered | - |
| `ERROR-CODES.md` | `interop` | `python3 tools/conformance/run_interop_core_vectors.py` (`error_code_registry_*`) | covered | - |
| `WITNESS-ID.md` | `core` | `python3 tools/conformance/run_witness_id_vectors.py`; `premath-kernel` witness-id unit tests | covered | - |
| `OBLIGATION-DISCHARGE.md` | `core` | `sh tools/ci/run_task.sh coherence-check` (`scope_noncontradiction` obligation vocabulary parity); `capabilities.instruction_typing`; `capabilities.adjoints_sites` | covered | - |
| `GATE.md` | `core` | `python3 tools/conformance/run_gate_vectors.py` + `premath-kernel` gate tests + toy vectors | covered | - |
| `CONFORMANCE.md` | `conformance` | `sh tools/ci/run_task.sh conformance-check` (`premath conformance-check`); `python3 tools/conformance/run_interop_core_vectors.py`; `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CAPABILITY-VECTORS.md` | `capability-registry` | `sh tools/ci/run_task.sh conformance-check` (`premath conformance-check`); `python3 tools/conformance/run_capability_vectors.py` | covered | - |
| `CHANGE-MORPHISMS.md` | `control-plane` | `capabilities.change_morphisms` vectors | covered | - |
| `DOCTRINE-SITE.md` | `control-plane` | `sh tools/ci/run_task.sh doctrine-check` (site roundtrip/reachability + runtime orchestration route checker + MCP doctrine-operation parity + doctrine-inf vectors) | covered | - |
| `DOCTRINE-SITE.json` | `control-plane` | `sh tools/ci/run_task.sh doctrine-check` (site roundtrip/reachability + runtime orchestration route checker + MCP doctrine-operation parity + doctrine-inf vectors) | covered | - |
| `DOCTRINE-SITE-INPUT.json` | `control-plane` | `sh tools/ci/run_task.sh doctrine-check`; `python3 tools/conformance/generate_doctrine_site.py --check` | covered | - |
| `DOCTRINE-OP-REGISTRY.json` | `control-plane` | `sh tools/ci/run_task.sh doctrine-check`; `python3 tools/conformance/generate_doctrine_site.py --check`; `python3 tools/conformance/run_runtime_orchestration_vectors.py` | covered | - |
| `HARNESS-RUNTIME.md` | `control-plane` | `cargo test -p premath-cli`; `python3 tools/conformance/run_harness_typestate_vectors.py`; `python3 tools/conformance/check_runtime_orchestration.py` (runtime route presence/morphism coverage + routed CI path boundary + optional `controlPlaneKcirMappings` row-shape checks); `python3 tools/conformance/run_runtime_orchestration_vectors.py` (golden/adversarial + invariance profile-permutation vectors); `sh tools/ci/run_task.sh ci-hygiene-check`; `sh tools/ci/run_task.sh ci-drift-budget-check` | covered | - |
| `HARNESS-TYPESTATE.md` | `control-plane` | `cargo test -p premath-tusk`; `cargo test -p premath-cli`; `python3 tools/conformance/run_harness_typestate_vectors.py`; `sh tools/ci/run_task.sh ci-hygiene-check` | covered | - |
| `HARNESS-RETRY-ESCALATION.md` | `control-plane` | `python3 tools/ci/test_harness_retry_policy.py`; `python3 tools/ci/test_harness_escalation.py`; `sh tools/ci/run_task.sh ci-pipeline-test`; `sh tools/ci/run_task.sh doctrine-check` | covered | - |
| `LLM-INSTRUCTION-DOCTRINE.md` | `control-plane` | `capabilities.instruction_typing`; `capabilities.ci_witnesses`; `sh tools/ci/run_task.sh ci-pipeline-test` | covered | - |
| `LLM-PROPOSAL-CHECKING.md` | `control-plane` | `capabilities.instruction_typing`; `tools/ci/test_instruction_check_client.py`; `tools/ci/test_instruction_reject_witness.py` | covered | - |
| `PREMATH-COHERENCE.md` | `control-plane` | `sh tools/ci/run_task.sh coherence-check`; `cargo test -p premath-coherence`; `coherence-check` CLI smoke test | covered | - |
| `COHERENCE-CONTRACT.json` | `control-plane` | `sh tools/ci/run_task.sh coherence-check`; `coherence-check` CLI smoke test | covered | - |
| `CAPABILITY-REGISTRY.json` | `capability-registry` | `python3 tools/conformance/run_capability_vectors.py`; `sh tools/ci/run_task.sh coherence-check`; `sh tools/ci/run_task.sh ci-drift-budget-check` | covered | - |
| `CONTROL-PLANE-CONTRACT.json` | `control-plane` | `sh tools/ci/run_task.sh coherence-check`; `sh tools/ci/run_task.sh ci-pipeline-test`; `python3 tools/ci/test_control_plane_contract.py`; `python3 tools/ci/test_run_required_checks.py` | covered | - |
| `UNIFICATION-DOCTRINE.md` | `control-plane` | `sh tools/ci/run_task.sh coherence-check` (`gate_chain_parity` Stage 1+Stage 2 parity/rollback/authority checks + Stage 2 direct Core-obligation-evidence-route checks + Stage 3 typed-first closure mapping checks); `sh tools/ci/run_task.sh ci-drift-budget-check`; `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract` (`gate_chain_parity_stage1_*` + `gate_chain_parity_stage2_*` vectors); `python3 tools/conformance/run_capability_vectors.py --capability capabilities.ci_witnesses` (boundary-authority lineage + obstruction roundtrip vectors); decision-log traceability via `premath traceability-check` (Decisions 0106-0110) | covered | - |
| `SPAN-SQUARE-CHECKING.md` | `control-plane` | `sh tools/ci/run_task.sh coherence-check` (`span_square_commutation` via site vectors, including composition-law vectors); `python3 tools/conformance/run_fixture_suites.py --suite coherence-contract` | covered | - |
| `SPEC-INDEX.md` | `authority-index` | `sh tools/ci/run_task.sh traceability-check` (`premath traceability-check`) | covered | - |
| `SPEC-TRACEABILITY.md` | `authority-index` | `sh tools/ci/run_task.sh traceability-check` (`premath traceability-check`) | covered | - |

## 4. Coverage Targets (Open Gaps/Upgrades)

- MH coverage now belongs to `fish/sites/mh` and downstream Kurma/Tusk wires,
  not to the promoted Premath draft matrix. See `raw/MH-SITE-DEPENDENCY.md`.

## 5. Maintenance Rules

- Every promoted draft spec MUST have exactly one matrix row in this document.
- `gap` rows MUST reference a concrete target ID.
- Coverage target implementation work SHOULD be tracked as discovered issues
  linked from the active traceability issue chain.
