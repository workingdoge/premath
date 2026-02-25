#!/usr/bin/env python3
"""Sentinel tests for frontend-parity core-authority execution."""

from __future__ import annotations

import copy
import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

THIS_DIR = Path(__file__).resolve().parent
if str(THIS_DIR) not in sys.path:
    sys.path.insert(0, str(THIS_DIR))

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

        with patch.object(
            frontend_parity,
            "_run_kernel_site_resolve",
            return_value={"result": "accepted", "failureClasses": []},
        ):
            with self.assertRaisesRegex(ValueError, "missing witness object"):
                frontend_parity.evaluate_case(
                    case,
                    self._golden_case_path(),
                )

    def test_evaluate_case_uses_core_authority_not_fixture_kernel_verdict(self) -> None:
        case = self._load_golden_case()
        case = copy.deepcopy(case)
        case["scenario"]["kernelVerdict"] = "rejected"
        row = case["scenario"]["frontends"]["steel"]

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
            )

        self.assertEqual(result, "accepted")
        self.assertEqual(failure_classes, [])

    def test_evaluate_case_rejects_missing_site_resolve_operation_id(self) -> None:
        case = self._load_golden_case()
        case = copy.deepcopy(case)
        case["scenario"]["siteResolve"].pop("operationId", None)

        with self.assertRaisesRegex(ValueError, "scenario.siteResolve.operationId must be set"):
            frontend_parity.evaluate_case(case, self._golden_case_path())


if __name__ == "__main__":
    unittest.main()
