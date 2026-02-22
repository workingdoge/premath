#!/usr/bin/env python3
"""Unit tests for control-plane contract loader lane registry extensions."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import control_plane_contract


def _base_payload() -> dict:
    return {
        "schema": 1,
        "contractKind": "premath.control_plane.contract.v1",
        "contractId": "control-plane.default.v1",
        "schemaLifecycle": {
            "activeEpoch": "2026-02",
            "governance": {
                "mode": "rollover",
                "decisionRef": "decision-0105",
                "owner": "premath-core",
                "rolloverCadenceMonths": 6,
            },
            "kindFamilies": {
                "controlPlaneContractKind": {
                    "canonicalKind": "premath.control_plane.contract.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "premath.control_plane.contract.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "premath.control_plane.contract.v1",
                        }
                    ],
                },
                "requiredWitnessKind": {
                    "canonicalKind": "ci.required.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.required.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.v1",
                        }
                    ],
                },
                "requiredDecisionKind": {
                    "canonicalKind": "ci.required.decision.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.required.decision.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.decision.v1",
                        }
                    ],
                },
                "instructionWitnessKind": {
                    "canonicalKind": "ci.instruction.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.instruction.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.instruction.v1",
                        }
                    ],
                },
                "instructionPolicyKind": {
                    "canonicalKind": "ci.instruction.policy.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.instruction.policy.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.instruction.policy.v1",
                        }
                    ],
                },
                "requiredProjectionPolicy": {
                    "canonicalKind": "ci-topos-v0",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci-topos-v0-preview",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci-topos-v0",
                        }
                    ],
                },
                "requiredDeltaKind": {
                    "canonicalKind": "ci.required.delta.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.delta.v1",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.delta.v1",
                        }
                    ],
                },
            },
        },
        "requiredGateProjection": {
            "projectionPolicy": "ci-topos-v0",
            "checkIds": {
                "baseline": "baseline",
                "build": "build",
                "test": "test",
                "testToy": "test-toy",
                "testKcirToy": "test-kcir-toy",
                "conformanceCheck": "conformance-check",
                "conformanceRun": "conformance-run",
                "doctrineCheck": "doctrine-check",
            },
            "checkOrder": [
                "baseline",
                "build",
                "test",
                "test-toy",
                "test-kcir-toy",
                "conformance-check",
                "conformance-run",
                "doctrine-check",
            ],
        },
        "requiredWitness": {
            "witnessKind": "ci.required.v1",
            "decisionKind": "ci.required.decision.v1",
        },
        "instructionWitness": {
            "witnessKind": "ci.instruction.v1",
            "policyKind": "ci.instruction.policy.v1",
            "policyDigestPrefix": "pol1_",
        },
        "harnessRetry": {
            "policyKind": "ci.harness.retry.policy.v1",
            "policyPath": "policies/control/harness-retry-policy-v1.json",
            "escalationActions": [
                "issue_discover",
                "mark_blocked",
                "stop",
            ],
            "activeIssueEnvKeys": [
                "PREMATH_ACTIVE_ISSUE_ID",
                "PREMATH_ISSUE_ID",
            ],
            "issuesPathEnvKey": "PREMATH_ISSUES_PATH",
            "sessionPathEnvKey": "PREMATH_HARNESS_SESSION_PATH",
            "sessionPathDefault": ".premath/harness_session.json",
            "sessionIssueField": "issueId",
        },
    }


def _with_lane_registry(payload: dict) -> dict:
    out = dict(payload)
    out["evidenceLanes"] = {
        "semanticDoctrine": "semantic_doctrine",
        "strictChecker": "strict_checker",
        "witnessCommutation": "witness_commutation",
        "runtimeTransport": "runtime_transport",
    }
    out["laneArtifactKinds"] = {
        "semantic_doctrine": ["kernel_obligation"],
        "strict_checker": ["coherence_obligation"],
        "witness_commutation": ["square_witness"],
        "runtime_transport": ["squeak_site_witness"],
    }
    out["laneOwnership"] = {
        "checkerCoreOnlyObligations": ["cwf_substitution_identity"],
        "requiredCrossLaneWitnessRoute": {
            "pullbackBaseChange": "span_square_commutation"
        },
    }
    out["laneFailureClasses"] = [
        "lane_unknown",
        "lane_kind_unbound",
        "lane_ownership_violation",
        "lane_route_missing",
    ]
    return out


class ControlPlaneContractTests(unittest.TestCase):
    def _load(self, payload: dict) -> dict:
        with tempfile.TemporaryDirectory(prefix="control-plane-contract-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            return control_plane_contract.load_control_plane_contract(path)

    def test_load_accepts_lane_registry_extension(self) -> None:
        payload = _with_lane_registry(_base_payload())
        loaded = self._load(payload)
        self.assertEqual(
            loaded["evidenceLanes"]["semanticDoctrine"], "semantic_doctrine"
        )
        self.assertEqual(
            loaded["laneOwnership"]["requiredCrossLaneWitnessRoute"],
            "span_square_commutation",
        )
        self.assertIn("lane_route_missing", loaded["laneFailureClasses"])
        self.assertEqual(
            loaded["schemaLifecycle"]["epochDiscipline"]["rolloverEpoch"],
            "2026-06",
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["epochDiscipline"]["aliasRunwayMonths"],
            4,
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["mode"],
            "rollover",
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["rolloverCadenceMonths"],
            6,
        )
        self.assertEqual(
            loaded["harnessRetry"]["policyKind"],
            "ci.harness.retry.policy.v1",
        )
        self.assertIn("mark_blocked", loaded["harnessRetry"]["escalationActions"])
        self.assertEqual(
            loaded["harnessRetry"]["sessionPathEnvKey"],
            "PREMATH_HARNESS_SESSION_PATH",
        )

    def test_load_rejects_duplicate_lane_ids(self) -> None:
        payload = _with_lane_registry(_base_payload())
        payload["evidenceLanes"]["runtimeTransport"] = "strict_checker"
        with self.assertRaises(ValueError):
            self._load(payload)

    def test_load_rejects_unknown_lane_artifact_mapping(self) -> None:
        payload = _with_lane_registry(_base_payload())
        payload["laneArtifactKinds"]["unknown_lane"] = ["opaque_kind"]
        with self.assertRaises(ValueError):
            self._load(payload)

    def test_resolve_schema_kind_accepts_alias_within_support_window(self) -> None:
        self.assertEqual(
            control_plane_contract.resolve_schema_kind(
                "requiredWitnessKind",
                "ci.required.v0",
                active_epoch="2026-06",
            ),
            "ci.required.v1",
        )

    def test_resolve_schema_kind_rejects_expired_alias(self) -> None:
        with self.assertRaisesRegex(ValueError, "expired"):
            control_plane_contract.resolve_schema_kind(
                "requiredWitnessKind",
                "ci.required.v0",
                active_epoch="2026-07",
            )

    def test_load_rejects_mixed_rollover_epochs(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["kindFamilies"]["requiredWitnessKind"][
            "compatibilityAliases"
        ][0]["supportUntilEpoch"] = "2026-07"
        with self.assertRaisesRegex(ValueError, "one shared supportUntilEpoch"):
            self._load(payload)

    def test_load_rejects_rollover_runway_too_large(self) -> None:
        payload = _base_payload()
        for family in payload["schemaLifecycle"]["kindFamilies"].values():
            aliases = family.get("compatibilityAliases", [])
            for alias_row in aliases:
                alias_row["supportUntilEpoch"] = "2027-03"
        with self.assertRaisesRegex(ValueError, "max runway"):
            self._load(payload)

    def test_load_rejects_duplicate_harness_escalation_actions(self) -> None:
        payload = _base_payload()
        payload["harnessRetry"]["escalationActions"] = [
            "issue_discover",
            "issue_discover",
        ]
        with self.assertRaisesRegex(ValueError, "must not contain duplicates"):
            self._load(payload)

    def test_load_rejects_rollover_without_cadence(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"].pop("rolloverCadenceMonths", None)
        with self.assertRaisesRegex(ValueError, "rolloverCadenceMonths"):
            self._load(payload)

    def test_load_rejects_freeze_with_aliases(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"] = {
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze",
        }
        with self.assertRaisesRegex(ValueError, "mode=freeze requires no active compatibility aliases"):
            self._load(payload)

    def test_load_accepts_freeze_without_aliases(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"] = {
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze",
        }
        for family in payload["schemaLifecycle"]["kindFamilies"].values():
            family["compatibilityAliases"] = []
        loaded = self._load(payload)
        self.assertEqual(loaded["schemaLifecycle"]["governance"]["mode"], "freeze")
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["freezeReason"],
            "release-freeze",
        )


if __name__ == "__main__":
    unittest.main()
