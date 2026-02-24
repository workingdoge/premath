#!/usr/bin/env python3
"""Unit tests for KCIR mapping gate adapter behavior."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from core_cli_client import CoreCliClientError

import kcir_mapping_gate


class KcirMappingGateTests(unittest.TestCase):
    def test_instruction_mapping_uses_core_cli_payload(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-kcir-gate-instruction-") as tmp:
            root = Path(tmp)
            instruction_path = root / "instructions" / "sample.json"
            instruction_path.parent.mkdir(parents=True, exist_ok=True)
            instruction_path.write_text("{}\n", encoding="utf-8")

            payload = {
                "action": "kcir-mapping-check",
                "scope": "instruction",
                "profileId": "cp.kcir.mapping.v0",
                "declaredRows": [
                    "instructionEnvelope",
                    "proposalPayload",
                    "coherenceObligations",
                    "coherenceCheckPayload",
                    "doctrineRouteBinding",
                    "requiredDecisionInput",
                ],
                "checkedRows": [
                    "instructionEnvelope",
                    "coherenceObligations",
                    "doctrineRouteBinding",
                    "proposalPayload",
                ],
                "failureClasses": [],
            }

            with patch("kcir_mapping_gate.run_core_json_command", return_value=payload) as run_cmd:
                report = kcir_mapping_gate.evaluate_instruction_mapping(
                    root,
                    instruction_path=instruction_path,
                    instruction_id="sample",
                    strict=True,
                )

            run_cmd.assert_called_once()
            self.assertEqual(report.profile_id, "cp.kcir.mapping.v0")
            self.assertEqual(len(set(report.declared_rows)), 6)
            self.assertEqual(len(set(report.checked_rows)), 4)
            self.assertEqual(report.failure_classes, tuple())

            summary = "\n".join(kcir_mapping_gate.render_mapping_summary_lines(report))
            self.assertIn("- KCIR mapping coverage: `4/6`", summary)
            self.assertIn("- KCIR mapping failures: `(none)`", summary)

    def test_required_mapping_fail_closed_when_core_command_errors(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-kcir-gate-required-") as tmp:
            root = Path(tmp)

            with patch(
                "kcir_mapping_gate.run_core_json_command",
                side_effect=CoreCliClientError(
                    "kcir_mapping_contract_violation",
                    "core command failed",
                ),
            ):
                report = kcir_mapping_gate.evaluate_required_mapping(root, strict=True)

            self.assertEqual(report.profile_id, kcir_mapping_gate.CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID)
            self.assertIn(
                kcir_mapping_gate.KCIR_MAPPING_CONTRACT_VIOLATION,
                report.failure_classes,
            )
            self.assertEqual(
                report.checked_rows,
                (
                    "coherenceCheckPayload",
                    "requiredDecisionInput",
                    "coherenceObligations",
                    "doctrineRouteBinding",
                ),
            )


if __name__ == "__main__":
    unittest.main()
