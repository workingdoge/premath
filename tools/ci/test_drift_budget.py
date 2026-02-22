#!/usr/bin/env python3
"""Unit tests for deterministic drift-budget sentinel checks."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import check_drift_budget


class DriftBudgetChecksTests(unittest.TestCase):
    def test_spec_index_capability_map_drift_detects_unknown_capability(self) -> None:
        failed, details = check_drift_budget.check_spec_index_capability_map(
            spec_map={"draft/LLM-INSTRUCTION-DOCTRINE": "capabilities.instruction_typing"},
            executable_capabilities=("capabilities.normal_forms",),
            conditional_docs_map={
                "draft/LLM-INSTRUCTION-DOCTRINE": "capabilities.instruction_typing"
            },
        )
        self.assertTrue(failed)
        self.assertIn("capabilities.instruction_typing", details["unknownCapabilities"])

    def test_control_plane_lane_binding_drift_detects_route_mismatch(self) -> None:
        loaded_contract = {
            "evidenceLanes": {
                "semanticDoctrine": "semantic_doctrine",
                "strictChecker": "strict_checker",
                "witnessCommutation": "witness_commutation",
                "runtimeTransport": "runtime_transport",
            },
            "laneArtifactKinds": {"semantic_doctrine": ["kernel_obligation"]},
            "laneOwnership": {
                "checkerCoreOnlyObligations": ("cwf_substitution_identity",),
                "requiredCrossLaneWitnessRoute": "span_square_commutation",
            },
            "laneFailureClasses": ("lane_unknown", "lane_kind_unbound"),
            "harnessRetry": {
                "policyKind": "ci.harness.retry.policy.v1",
                "policyPath": "policies/control/harness-retry-policy-v1.json",
                "escalationActions": ("issue_discover", "mark_blocked", "stop"),
                "activeIssueEnvKeys": (
                    "PREMATH_ACTIVE_ISSUE_ID",
                    "PREMATH_ISSUE_ID",
                ),
                "issuesPathEnvKey": "PREMATH_ISSUES_PATH",
            },
        }
        control_plane_module = SimpleNamespace(
            EVIDENCE_LANES=loaded_contract["evidenceLanes"],
            LANE_ARTIFACT_KINDS=loaded_contract["laneArtifactKinds"],
            CHECKER_CORE_ONLY_OBLIGATIONS=loaded_contract["laneOwnership"][
                "checkerCoreOnlyObligations"
            ],
            REQUIRED_CROSS_LANE_WITNESS_ROUTE="span_square_commutation",
            LANE_FAILURE_CLASSES=loaded_contract["laneFailureClasses"],
            HARNESS_RETRY_POLICY_KIND=loaded_contract["harnessRetry"]["policyKind"],
            HARNESS_RETRY_POLICY_PATH=loaded_contract["harnessRetry"]["policyPath"],
            HARNESS_ESCALATION_ACTIONS=loaded_contract["harnessRetry"][
                "escalationActions"
            ],
            HARNESS_ACTIVE_ISSUE_ENV_KEYS=loaded_contract["harnessRetry"][
                "activeIssueEnvKeys"
            ],
            HARNESS_ISSUES_PATH_ENV_KEY=loaded_contract["harnessRetry"][
                "issuesPathEnvKey"
            ],
        )
        gate_chain_details = {
            "laneRegistry": {
                "expectedCheckerCoreOnlyObligations": ["cwf_substitution_identity"],
                "requiredCrossLaneWitnessRoute": "wrong_route",
                "requiredLaneFailureClasses": ["lane_unknown"],
            }
        }
        failed, details = check_drift_budget.check_control_plane_lane_bindings(
            loaded_contract, control_plane_module, gate_chain_details
        )
        self.assertTrue(failed)
        self.assertIn("required cross-lane witness route", " ".join(details["reasons"]))

    def test_required_obligation_drift_detects_contract_checker_mismatch(self) -> None:
        coherence_contract = {
            "obligations": [{"id": "scope_noncontradiction"}, {"id": "gate_chain_parity"}],
            "requiredBidirObligations": ["stability"],
            "surfaces": {"obligationRegistryKind": "premath.obligation_gate_registry.v1"},
        }
        scope_details = {
            "requiredCoherenceObligations": ["scope_noncontradiction"],
            "requiredBidirObligations": ["locality"],
            "obligationRegistryKind": "premath.obligation_gate_registry.v2",
        }
        failed, details = check_drift_budget.check_coherence_required_obligations(
            coherence_contract, scope_details
        )
        self.assertTrue(failed)
        self.assertIn(
            "coherence required obligation set drifts between contract and checker",
            details["reasons"],
        )

    def test_sigpi_notation_drift_detects_alias(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-sigpi-drift-") as tmp:
            root = Path(tmp)
            for rel in check_drift_budget.SIGPI_NORMATIVE_DOCS:
                path = root / rel
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(
                    "Canonical path with SigPi and sig\\Pi notation.\n",
                    encoding="utf-8",
                )
            alias_path = root / check_drift_budget.SIGPI_NORMATIVE_DOCS[0]
            alias_path.write_text("This doc still says Sig/Pi.\n", encoding="utf-8")

            failed, details = check_drift_budget.check_sigpi_notation(root)
            self.assertTrue(failed)
            self.assertIn(
                check_drift_budget.SIGPI_NORMATIVE_DOCS[0], details["aliasHits"]
            )

    def test_cache_input_closure_drift_detects_missing_required_paths(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-cache-closure-drift-") as tmp:
            root = Path(tmp)
            fixture_module = SimpleNamespace(
                load_coherence_contract_input_paths=lambda: [root / "only-this-path"]
            )
            failed, details = check_drift_budget.check_cache_input_closure(
                root, fixture_module
            )
            self.assertTrue(failed)
            self.assertIn(
                "specs/premath/draft/COHERENCE-CONTRACT.json",
                details["missingPaths"],
            )


if __name__ == "__main__":
    unittest.main()
