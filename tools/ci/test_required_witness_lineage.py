#!/usr/bin/env python3
"""Tests for ci.required semantic/operational failure lineage mapping."""

from __future__ import annotations

import unittest
from typing import Any, Dict, List, Tuple

from change_projection import PROJECTION_POLICY, project_required_checks
from delta_snapshot import compute_typed_core_projection_digest
from gate_witness_envelope import stable_sha256
from required_witness import verify_required_witness_payload


def _lineage_fixture() -> Tuple[List[str], Dict[str, Any], Dict[str, Dict[str, Any]]]:
    changed_paths = ["crates/premath-bd/src/lib.rs"]
    projection = project_required_checks(changed_paths)
    required_checks = list(projection.required_checks)
    if required_checks != ["build", "test"]:
        raise AssertionError(f"unexpected required checks for fixture: {required_checks}")

    gate_build = {
        "witnessKind": "gate",
        "runId": "run1_fixture_build",
        "result": "accepted",
        "failures": [],
    }
    gate_test = {
        "witnessKind": "gate",
        "runId": "run1_fixture_test",
        "result": "rejected",
        "failures": [
            {
                "class": "descent_failure",
                "lawRef": "GATE-3.3",
                "message": "fixture failure",
            }
        ],
    }

    gate_payloads = {
        f"gates/{projection.projection_digest}/01-build.json": gate_build,
        f"gates/{projection.projection_digest}/02-test.json": gate_test,
    }
    normalizer_id = "normalizer.ci.required.v1"
    typed_core_projection_digest = compute_typed_core_projection_digest(
        projection.projection_digest,
        normalizer_id,
        PROJECTION_POLICY,
    )

    witness = {
        "ciSchema": 1,
        "witnessKind": "ci.required.v1",
        "projectionPolicy": PROJECTION_POLICY,
        "projectionDigest": projection.projection_digest,
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": projection.projection_digest,
        "normalizerId": normalizer_id,
        "policyDigest": PROJECTION_POLICY,
        "changedPaths": changed_paths,
        "requiredChecks": required_checks,
        "executedChecks": required_checks,
        "results": [
            {
                "checkId": "build",
                "status": "passed",
                "exitCode": 0,
                "durationMs": 10,
            },
            {
                "checkId": "test",
                "status": "failed",
                "exitCode": 1,
                "durationMs": 20,
            },
        ],
        "gateWitnessRefs": [
            {
                "checkId": "build",
                "artifactRelPath": f"gates/{projection.projection_digest}/01-build.json",
                "sha256": stable_sha256(gate_build),
                "source": "native",
                "runId": gate_build["runId"],
                "witnessKind": "gate",
                "result": "accepted",
                "failureClasses": [],
            },
            {
                "checkId": "test",
                "artifactRelPath": f"gates/{projection.projection_digest}/02-test.json",
                "sha256": stable_sha256(gate_test),
                "source": "native",
                "runId": gate_test["runId"],
                "witnessKind": "gate",
                "result": "rejected",
                "failureClasses": ["descent_failure"],
            },
        ],
        "verdictClass": "rejected",
        "operationalFailureClasses": ["check_failed"],
        "semanticFailureClasses": ["descent_failure"],
        "failureClasses": ["check_failed", "descent_failure"],
        "docsOnly": projection.docs_only,
        "reasons": list(projection.reasons),
    }
    return changed_paths, witness, gate_payloads


class RequiredWitnessLineageTests(unittest.TestCase):
    def test_lineage_fields_accept_when_union_matches_payload(self) -> None:
        changed_paths, witness, gate_payloads = _lineage_fixture()
        errors, _derived = verify_required_witness_payload(
            witness,
            changed_paths,
            gate_witness_payloads=gate_payloads,
        )
        self.assertEqual(errors, [])

    def test_rejects_missing_semantic_union_member(self) -> None:
        changed_paths, witness, gate_payloads = _lineage_fixture()
        witness["failureClasses"] = ["check_failed"]
        errors, _derived = verify_required_witness_payload(
            witness,
            changed_paths,
            gate_witness_payloads=gate_payloads,
        )
        self.assertTrue(any("failureClasses mismatch" in err for err in errors))

    def test_rejects_semantic_field_mismatch(self) -> None:
        changed_paths, witness, gate_payloads = _lineage_fixture()
        witness["semanticFailureClasses"] = ["locality_failure"]
        errors, _derived = verify_required_witness_payload(
            witness,
            changed_paths,
            gate_witness_payloads=gate_payloads,
        )
        self.assertTrue(any("semanticFailureClasses mismatch" in err for err in errors))


if __name__ == "__main__":
    unittest.main()
