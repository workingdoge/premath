#!/usr/bin/env python3
"""Sentinel tests for frontend-parity core-authority execution."""

from __future__ import annotations

import copy
import os
import unittest
from pathlib import Path
from unittest.mock import patch

import run_frontend_parity_vectors as frontend_parity


class FrontendParityVectorTests(unittest.TestCase):
    def _golden_case_path(self) -> Path:
        return (
            frontend_parity.DEFAULT_FIXTURES
            / "golden"
            / "host_action_issue_claim_next_parity_accept"
            / "case.json"
        )

    def _load_golden_case(self) -> dict:
        return frontend_parity.load_json(self._golden_case_path())

    def _load_control_plane_contract(self) -> dict:
        return frontend_parity.load_json(frontend_parity.DEFAULT_CONTROL_PLANE_CONTRACT)

    def test_site_resolve_command_override_rejects_noncanonical_prefix(self) -> None:
        with patch.dict(
            os.environ,
            {"PREMATH_SITE_RESOLVE_CMD": "python3 tools/conformance/run_frontend_parity_vectors.py"},
            clear=False,
        ):
            with self.assertRaisesRegex(ValueError, "site-resolve command surface drift"):
                frontend_parity._resolve_site_resolve_command()  # noqa: SLF001

    def test_evaluate_case_fails_closed_when_core_payload_missing_witness(self) -> None:
        case = self._load_golden_case()
        control_plane_contract = self._load_control_plane_contract()

        with patch.object(
            frontend_parity,
            "_run_kernel_site_resolve",
            return_value={"result": "accepted", "failureClasses": []},
        ):
            with self.assertRaisesRegex(ValueError, "missing witness object"):
                frontend_parity.evaluate_case(
                    case,
                    self._golden_case_path(),
                    control_plane_contract,
                )

    def test_evaluate_case_uses_core_authority_not_fixture_kernel_verdict(self) -> None:
        case = self._load_golden_case()
        case = copy.deepcopy(case)
        case["scenario"]["kernelVerdict"] = "rejected"
        row = case["scenario"]["frontends"]["steel"]
        control_plane_contract = self._load_control_plane_contract()

        core_payload = {
            "result": "accepted",
            "failureClasses": [],
            "selected": {
                "routeFamilyId": row["worldRouteId"],
            },
            "witness": {
                "siteId": row["resolverWitness"]["siteId"],
                "operationId": row["resolverWitness"]["operationId"],
                "routeFamilyId": row["resolverWitness"]["routeFamilyId"],
                "worldId": row["resolverWitness"]["worldId"],
                "morphismRowId": row["resolverWitness"]["morphismRowId"],
                "semanticDigest": "sr1_fixture_core_authority",
                "failureClasses": [],
            },
        }

        with patch.object(
            frontend_parity,
            "_run_kernel_site_resolve",
            return_value=core_payload,
        ):
            result, failure_classes = frontend_parity.evaluate_case(
                case,
                self._golden_case_path(),
                control_plane_contract,
            )

        self.assertEqual(result, "accepted")
        self.assertEqual(failure_classes, [])


if __name__ == "__main__":
    unittest.main()
