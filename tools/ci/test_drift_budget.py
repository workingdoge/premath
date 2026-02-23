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
            "schemaLifecycle": {
                "governance": {
                    "mode": "rollover",
                    "decisionRef": "decision-0105",
                    "owner": "premath-core",
                    "rolloverCadenceMonths": 6,
                    "freezeReason": None,
                }
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
                "sessionPathEnvKey": "PREMATH_HARNESS_SESSION_PATH",
                "sessionPathDefault": ".premath/harness_session.json",
                "sessionIssueField": "issueId",
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
            SCHEMA_LIFECYCLE_GOVERNANCE_MODE=loaded_contract["schemaLifecycle"][
                "governance"
            ]["mode"],
            SCHEMA_LIFECYCLE_GOVERNANCE_DECISION_REF=loaded_contract[
                "schemaLifecycle"
            ]["governance"]["decisionRef"],
            SCHEMA_LIFECYCLE_GOVERNANCE_OWNER=loaded_contract["schemaLifecycle"][
                "governance"
            ]["owner"],
            SCHEMA_LIFECYCLE_ROLLOVER_CADENCE_MONTHS=loaded_contract[
                "schemaLifecycle"
            ]["governance"]["rolloverCadenceMonths"],
            SCHEMA_LIFECYCLE_FREEZE_REASON=loaded_contract["schemaLifecycle"][
                "governance"
            ]["freezeReason"],
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
            HARNESS_SESSION_PATH_ENV_KEY=loaded_contract["harnessRetry"][
                "sessionPathEnvKey"
            ],
            HARNESS_SESSION_PATH_DEFAULT=loaded_contract["harnessRetry"][
                "sessionPathDefault"
            ],
            HARNESS_SESSION_ISSUE_FIELD=loaded_contract["harnessRetry"][
                "sessionIssueField"
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

    def test_runtime_route_binding_drift_accepts_when_registry_bound(self) -> None:
        loaded_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                    },
                    "runGateTerraform": {
                        "operationId": "op/ci.run_gate_terraform",
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                    },
                },
                "failureClasses": {
                    "missingRoute": "runtime_route_missing",
                    "morphismDrift": "runtime_route_morphism_drift",
                    "contractUnbound": "runtime_route_contract_unbound",
                },
            }
        }
        control_plane_module = SimpleNamespace(
            RUNTIME_ROUTE_BINDINGS=loaded_contract["runtimeRouteBindings"][
                "requiredOperationRoutes"
            ],
            RUNTIME_ROUTE_FAILURE_CLASSES=(
                "runtime_route_missing",
                "runtime_route_morphism_drift",
                "runtime_route_contract_unbound",
            ),
        )
        doctrine_operations = {
            "op/ci.run_gate": {
                "path": "tools/ci/run_gate.sh",
                "morphisms": [
                    "dm.identity",
                    "dm.profile.execution",
                    "dm.transport.location",
                    "dm.transport.world",
                ],
            },
            "op/ci.run_gate_terraform": {
                "path": "tools/ci/run_gate_terraform.sh",
                "morphisms": [
                    "dm.identity",
                    "dm.profile.execution",
                    "dm.transport.location",
                    "dm.transport.world",
                ],
            },
        }
        failed, details = check_drift_budget.check_runtime_route_bindings(
            loaded_contract, control_plane_module, doctrine_operations
        )
        self.assertFalse(failed)
        self.assertEqual(details["missingOperationRoutes"], [])
        self.assertEqual(details["missingRequiredMorphisms"], [])

    def test_control_plane_kcir_mapping_drift_accepts_when_loader_matches(self) -> None:
        loaded_contract = {
            "controlPlaneKcirMappings": {
                "profileId": "cp.kcir.mapping.v0",
                "mappingTable": {
                    "instructionEnvelope": {
                        "sourceKind": "ci.instruction.envelope.v1",
                        "targetDomain": "kcir.node",
                        "targetKind": "ci.instruction.v1",
                        "identityFields": [
                            "instructionDigest",
                            "normalizerId",
                            "policyDigest",
                        ],
                    },
                    "requiredDecisionInput": {
                        "sourceKind": "ci.required.decision.input.v1",
                        "targetDomain": "kcir.node",
                        "targetKind": "ci.required.decision.v1",
                        "identityFields": [
                            "requiredDigest",
                            "decisionDigest",
                            "policyDigest",
                        ],
                    },
                },
                "compatibilityPolicy": {
                    "legacyNonKcirEncodings": {
                        "mode": "projection_only",
                        "authorityMode": "forbidden",
                        "supportUntilEpoch": "2026-06",
                        "failureClass": "kcir_mapping_legacy_encoding_authority_violation",
                    }
                },
            }
        }
        control_plane_module = SimpleNamespace(
            CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID="cp.kcir.mapping.v0",
            CONTROL_PLANE_KCIR_MAPPING_TABLE=loaded_contract["controlPlaneKcirMappings"][
                "mappingTable"
            ],
            CONTROL_PLANE_KCIR_LEGACY_POLICY=loaded_contract["controlPlaneKcirMappings"][
                "compatibilityPolicy"
            ]["legacyNonKcirEncodings"],
        )
        failed, details = check_drift_budget.check_control_plane_kcir_mappings(
            loaded_contract, control_plane_module
        )
        self.assertFalse(failed)
        self.assertEqual(details["missingRowsInLoader"], [])
        self.assertEqual(details["rowDrifts"], [])
        self.assertEqual(details["legacyPolicyDriftFields"], [])

    def test_control_plane_kcir_mapping_drift_rejects_row_and_legacy_policy_drift(
        self,
    ) -> None:
        loaded_contract = {
            "controlPlaneKcirMappings": {
                "profileId": "cp.kcir.mapping.v0",
                "mappingTable": {
                    "instructionEnvelope": {
                        "sourceKind": "ci.instruction.envelope.v1",
                        "targetDomain": "kcir.node",
                        "targetKind": "ci.instruction.v1",
                        "identityFields": [
                            "instructionDigest",
                            "normalizerId",
                            "policyDigest",
                        ],
                    }
                },
                "compatibilityPolicy": {
                    "legacyNonKcirEncodings": {
                        "mode": "projection_only",
                        "authorityMode": "forbidden",
                        "supportUntilEpoch": "2026-06",
                        "failureClass": "kcir_mapping_legacy_encoding_authority_violation",
                    }
                },
            }
        }
        control_plane_module = SimpleNamespace(
            CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID="cp.kcir.mapping.v0",
            CONTROL_PLANE_KCIR_MAPPING_TABLE={
                "instructionEnvelope": {
                    "sourceKind": "ci.instruction.envelope.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "ci.instruction.v1",
                    "identityFields": [
                        "instructionDigest",
                        "policyDigest",
                    ],
                }
            },
            CONTROL_PLANE_KCIR_LEGACY_POLICY={
                "mode": "projection_only",
                "authorityMode": "forbidden",
                "supportUntilEpoch": "2026-07",
                "failureClass": "kcir_mapping_legacy_encoding_authority_violation",
            },
        )
        failed, details = check_drift_budget.check_control_plane_kcir_mappings(
            loaded_contract, control_plane_module
        )
        self.assertTrue(failed)
        self.assertEqual(details["rowDrifts"][0]["rowId"], "instructionEnvelope")
        self.assertIn("identityFields", details["rowDrifts"][0]["driftFields"])
        self.assertIn("supportUntilEpoch", details["legacyPolicyDriftFields"])

    def test_runtime_route_binding_drift_rejects_missing_registry_route(self) -> None:
        loaded_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                    }
                },
                "failureClasses": {
                    "missingRoute": "runtime_route_missing",
                    "morphismDrift": "runtime_route_morphism_drift",
                    "contractUnbound": "runtime_route_contract_unbound",
                },
            }
        }
        control_plane_module = SimpleNamespace(
            RUNTIME_ROUTE_BINDINGS=loaded_contract["runtimeRouteBindings"][
                "requiredOperationRoutes"
            ],
            RUNTIME_ROUTE_FAILURE_CLASSES=(
                "runtime_route_missing",
                "runtime_route_morphism_drift",
                "runtime_route_contract_unbound",
            ),
        )
        doctrine_operations: dict = {}
        failed, details = check_drift_budget.check_runtime_route_bindings(
            loaded_contract, control_plane_module, doctrine_operations
        )
        self.assertTrue(failed)
        self.assertIn("DOCTRINE-OP-REGISTRY", " ".join(details["reasons"]))

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

    def _write_topology_fixture_repo(self, root: Path, *, design_docs: int = 1) -> None:
        draft_dir = root / "specs" / "premath" / "draft"
        draft_dir.mkdir(parents=True, exist_ok=True)
        (draft_dir / "README.md").write_text("# draft\n", encoding="utf-8")
        (draft_dir / "A.md").write_text(
            "---\nstatus: draft\n---\n\n# A\n",
            encoding="utf-8",
        )
        (draft_dir / "DOCTRINE-SITE.json").write_text(
            '{"edges":[{"id":"e1"}]}\n',
            encoding="utf-8",
        )
        (draft_dir / "DOCTRINE-OP-REGISTRY.json").write_text(
            '{"operations":[]}\n',
            encoding="utf-8",
        )
        (draft_dir / "DOCTRINE-SITE-INPUT.json").write_text(
            '{"schema":1}\n',
            encoding="utf-8",
        )
        (draft_dir / "SPEC-TRACEABILITY.md").write_text(
            "\n".join(
                [
                    "## 3. Traceability Matrix",
                    "",
                    "| Draft spec | Executable check surface | Status | Gap target |",
                    "| --- | --- | --- | --- |",
                    "| `A.md` | `test` | covered | - |",
                    "",
                ]
            )
            + "\n",
            encoding="utf-8",
        )

        design_dir = root / "docs" / "design"
        design_dir.mkdir(parents=True, exist_ok=True)
        (design_dir / "README.md").write_text("# design\n", encoding="utf-8")
        for idx in range(design_docs):
            (design_dir / f"DOC-{idx}.md").write_text("# doc\n", encoding="utf-8")

    def test_topology_budget_warns_without_failing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-topology-budget-warn-") as tmp:
            root = Path(tmp)
            self._write_topology_fixture_repo(root, design_docs=2)
            process_dir = root / "specs" / "process"
            process_dir.mkdir(parents=True, exist_ok=True)
            budget_path = process_dir / "TOPOLOGY-BUDGET.json"
            budget_path.write_text(
                """
{
  "schema": 1,
  "budgetKind": "premath.topology_budget.v1",
  "metrics": {
    "draftSpecNodes": {"failAbove": 5},
    "specTraceabilityRows": {"failAbove": 5},
    "designDocNodes": {"warnAbove": 1, "failAbove": 3},
    "doctrineSiteEdgeCount": {"failAbove": 10},
    "doctrineSiteAuthorityInputCount": {"warnAbove": 1, "failAbove": 1, "warnBelow": 1, "failBelow": 1},
    "doctrineSiteGeneratedViewCount": {"warnAbove": 2, "failAbove": 2, "warnBelow": 2, "failBelow": 2},
    "deprecatedDesignFragmentCount": {"failAbove": 0}
  },
  "deprecatedDesignFragments": ["docs/design/LEGACY.md"],
  "doctrineSiteAuthorityInputs": [
    "specs/premath/draft/DOCTRINE-SITE-INPUT.json",
    "specs/premath/draft/DOCTRINE-SITE-SOURCE.json"
  ],
  "doctrineSiteGeneratedViews": [
    "specs/premath/draft/DOCTRINE-SITE.json",
    "specs/premath/draft/DOCTRINE-OP-REGISTRY.json"
  ]
}
""".strip()
                + "\n",
                encoding="utf-8",
            )

            failed, warned, details = check_drift_budget.check_topology_budget(
                root, budget_path
            )
            self.assertFalse(failed)
            self.assertTrue(warned)
            self.assertIn("designDocNodes", " ".join(details["warnings"]))

    def test_topology_budget_fails_when_deprecated_fragment_exists(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-topology-budget-fail-") as tmp:
            root = Path(tmp)
            self._write_topology_fixture_repo(root, design_docs=1)
            legacy_path = root / "docs" / "design" / "LEGACY-FRAGMENT.md"
            legacy_path.write_text("# legacy\n", encoding="utf-8")
            process_dir = root / "specs" / "process"
            process_dir.mkdir(parents=True, exist_ok=True)
            budget_path = process_dir / "TOPOLOGY-BUDGET.json"
            budget_path.write_text(
                """
{
  "schema": 1,
  "budgetKind": "premath.topology_budget.v1",
  "metrics": {
    "draftSpecNodes": {"failAbove": 5},
    "specTraceabilityRows": {"failAbove": 5},
    "designDocNodes": {"failAbove": 5},
    "doctrineSiteEdgeCount": {"failAbove": 10},
    "doctrineSiteAuthorityInputCount": {"warnAbove": 1, "failAbove": 1, "warnBelow": 1, "failBelow": 1},
    "doctrineSiteGeneratedViewCount": {"warnAbove": 2, "failAbove": 2, "warnBelow": 2, "failBelow": 2},
    "deprecatedDesignFragmentCount": {"warnAbove": 0, "failAbove": 0}
  },
  "deprecatedDesignFragments": ["docs/design/LEGACY-FRAGMENT.md"],
  "doctrineSiteAuthorityInputs": [
    "specs/premath/draft/DOCTRINE-SITE-INPUT.json",
    "specs/premath/draft/DOCTRINE-SITE-SOURCE.json"
  ],
  "doctrineSiteGeneratedViews": [
    "specs/premath/draft/DOCTRINE-SITE.json",
    "specs/premath/draft/DOCTRINE-OP-REGISTRY.json"
  ]
}
""".strip()
                + "\n",
                encoding="utf-8",
            )

            failed, warned, details = check_drift_budget.check_topology_budget(
                root, budget_path
            )
            self.assertTrue(failed)
            self.assertFalse(warned)
            self.assertIn("deprecatedDesignFragmentCount", " ".join(details["reasons"]))


if __name__ == "__main__":
    unittest.main()
