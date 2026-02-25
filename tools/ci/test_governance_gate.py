#!/usr/bin/env python3
"""Unit tests for governance gate adapter behavior."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from core_cli_client import CoreCliClientError

import governance_gate


class GovernanceGateTests(unittest.TestCase):
    def test_governance_failure_classes_uses_core_cli_payload(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-governance-gate-") as tmp:
            root = Path(tmp)
            payload = {
                "action": "governance-promotion-check",
                "failureClasses": [
                    "governance.eval_gate_unmet",
                    "governance.eval_lineage_missing",
                ],
            }

            with patch("governance_gate.run_core_json_command", return_value=payload) as run_cmd:
                failures = governance_gate.governance_failure_classes(root)

            run_cmd.assert_called_once()
            self.assertEqual(
                failures,
                (
                    "governance.eval_gate_unmet",
                    "governance.eval_lineage_missing",
                ),
            )

    def test_governance_failure_classes_fails_closed_on_core_error(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-governance-gate-fail-") as tmp:
            root = Path(tmp)

            with patch(
                "governance_gate.run_core_json_command",
                side_effect=CoreCliClientError(
                    "governance_gate_invalid",
                    "core command failed",
                ),
            ):
                failures = governance_gate.governance_failure_classes(root)

            self.assertEqual(failures, ("governance.eval_lineage_missing",))

    def test_governance_failure_classes_passes_env_overrides_to_core_cli(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-governance-gate-env-") as tmp:
            root = Path(tmp)
            env_backup = {
                "PREMATH_GOVERNANCE_PROMOTION_REQUIRED": os.environ.get(
                    "PREMATH_GOVERNANCE_PROMOTION_REQUIRED"
                ),
                "PREMATH_GOVERNANCE_PROMOTION_EVIDENCE": os.environ.get(
                    "PREMATH_GOVERNANCE_PROMOTION_EVIDENCE"
                ),
            }
            try:
                os.environ["PREMATH_GOVERNANCE_PROMOTION_REQUIRED"] = "true"
                os.environ["PREMATH_GOVERNANCE_PROMOTION_EVIDENCE"] = "artifacts/ciwitness/custom.json"
                with patch(
                    "governance_gate.run_core_json_command",
                    return_value={"action": "governance-promotion-check", "failureClasses": []},
                ) as run_cmd:
                    governance_gate.governance_failure_classes(root)

                _, kwargs = run_cmd.call_args
                request_payload = kwargs.get("request_payload")
                self.assertIsInstance(request_payload, dict)
                self.assertEqual(request_payload.get("repoRoot"), str(root))
                self.assertEqual(request_payload.get("promotionRequired"), True)
                self.assertEqual(
                    request_payload.get("promotionEvidencePath"),
                    "artifacts/ciwitness/custom.json",
                )
            finally:
                for key, value in env_backup.items():
                    if value is None:
                        os.environ.pop(key, None)
                    else:
                        os.environ[key] = value


if __name__ == "__main__":
    unittest.main()
