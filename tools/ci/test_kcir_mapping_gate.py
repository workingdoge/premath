#!/usr/bin/env python3
"""Unit tests for KCIR mapping gate coverage and fail-closed behavior."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import kcir_mapping_gate


class KcirMappingGateTests(unittest.TestCase):
    def test_instruction_mapping_reports_declared_six_rows_with_coverage(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-kcir-gate-instruction-") as tmp:
            root = Path(tmp)
            instruction_path = root / "instructions" / "sample.json"
            instruction_path.parent.mkdir(parents=True, exist_ok=True)
            instruction_path.write_text(
                json.dumps({"proposal": {"proposalKind": "value"}}, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)
            (ciwitness / "sample.json").write_text(
                json.dumps(
                    {
                        "instructionDigest": "instr_demo",
                        "normalizerId": "normalizer.demo.v1",
                        "policyDigest": "pol1_demo",
                        "proposalIngest": {
                            "proposalDigest": "prop_demo",
                            "proposalKcirRef": "kcir1_demo",
                            "policyDigest": "pol1_demo",
                        },
                    },
                    ensure_ascii=False,
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            report = kcir_mapping_gate.evaluate_instruction_mapping(
                root,
                instruction_path=instruction_path,
                instruction_id="sample",
                strict=True,
            )
            self.assertEqual(len(set(report.declared_rows)), 6)
            self.assertEqual(len(set(report.checked_rows)), 4)
            self.assertEqual(report.failure_classes, tuple())

            summary = "\n".join(kcir_mapping_gate.render_mapping_summary_lines(report))
            self.assertIn("- KCIR mapping coverage: `4/6`", summary)
            self.assertIn(
                "- KCIR mapping rows missing: `coherenceCheckPayload, requiredDecisionInput`",
                summary,
            )

    def test_required_mapping_reports_declared_six_rows_with_coverage(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-kcir-gate-required-") as tmp:
            root = Path(tmp)
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)
            (ciwitness / "latest-required.json").write_text(
                json.dumps(
                    {
                        "projectionDigest": "proj_demo",
                        "normalizerId": "normalizer.ci.required.v1",
                        "policyDigest": "pol1_demo",
                    },
                    ensure_ascii=False,
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            (ciwitness / "latest-decision.json").write_text(
                json.dumps(
                    {
                        "witnessSha256": "witness_demo",
                        "policyDigest": "pol1_demo",
                    },
                    ensure_ascii=False,
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            report = kcir_mapping_gate.evaluate_required_mapping(root, strict=True)
            self.assertEqual(len(set(report.declared_rows)), 6)
            self.assertEqual(len(set(report.checked_rows)), 4)
            self.assertEqual(report.failure_classes, tuple())

            summary = "\n".join(kcir_mapping_gate.render_mapping_summary_lines(report))
            self.assertIn("- KCIR mapping coverage: `4/6`", summary)
            self.assertIn(
                "- KCIR mapping rows missing: `instructionEnvelope, proposalPayload`",
                summary,
            )

    def test_instruction_mapping_rejects_missing_declared_row_set(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-kcir-gate-rowset-") as tmp:
            root = Path(tmp)
            instruction_path = root / "instructions" / "sample.json"
            instruction_path.parent.mkdir(parents=True, exist_ok=True)
            instruction_path.write_text(
                json.dumps({"proposal": {"proposalKind": "value"}}, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)
            (ciwitness / "sample.json").write_text(
                json.dumps(
                    {
                        "instructionDigest": "instr_demo",
                        "normalizerId": "normalizer.demo.v1",
                        "policyDigest": "pol1_demo",
                        "proposalIngest": {
                            "proposalDigest": "prop_demo",
                            "proposalKcirRef": "kcir1_demo",
                            "policyDigest": "pol1_demo",
                        },
                    },
                    ensure_ascii=False,
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )

            mapped_rows = dict(kcir_mapping_gate.CONTROL_PLANE_KCIR_MAPPING_TABLE)
            mapped_rows.pop("requiredDecisionInput", None)

            with patch(
                "kcir_mapping_gate.CONTROL_PLANE_KCIR_MAPPING_TABLE",
                mapped_rows,
            ):
                report = kcir_mapping_gate.evaluate_instruction_mapping(
                    root,
                    instruction_path=instruction_path,
                    instruction_id="sample",
                    strict=True,
                )

            self.assertIn(
                kcir_mapping_gate.KCIR_MAPPING_CONTRACT_VIOLATION,
                report.failure_classes,
            )


if __name__ == "__main__":
    unittest.main()
