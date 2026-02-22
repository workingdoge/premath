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


if __name__ == "__main__":
    unittest.main()

