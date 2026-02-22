#!/usr/bin/env python3
"""Unit tests for required-check runner behavior."""

from __future__ import annotations

import json
import stat
import tempfile
import unittest
from pathlib import Path

import run_required_checks


def _make_stub_gate_script(root: Path) -> None:
    gate_script = root / "tools" / "ci" / "run_gate.sh"
    gate_script.parent.mkdir(parents=True, exist_ok=True)
    gate_script.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    gate_script.chmod(gate_script.stat().st_mode | stat.S_IEXEC)


class RunRequiredChecksTests(unittest.TestCase):
    def test_run_check_ignores_stale_native_artifacts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-run-required-") as tmp:
            root = Path(tmp)
            out_dir = root / "artifacts" / "ciwitness"
            out_dir.mkdir(parents=True, exist_ok=True)
            _make_stub_gate_script(root)

            projection = "proj1_stale_gate_demo"
            check_id = "baseline"
            gate_path = run_required_checks._gate_artifact_path(
                out_dir, projection, check_id, 0
            )
            source_path = run_required_checks._gate_source_path(
                out_dir, projection, check_id, 0
            )

            gate_path.write_text(
                json.dumps({"witnessKind": "gate", "result": "rejected"}, ensure_ascii=False)
                + "\n",
                encoding="utf-8",
            )
            source_path.write_text("native\n", encoding="utf-8")

            row = run_required_checks.run_check_with_witness(
                root=root,
                out_dir=out_dir,
                check_id=check_id,
                projection_digest=projection,
                policy_digest="ci-topos-v0",
                from_ref="origin/main",
                to_ref="HEAD",
                index=0,
            )

            self.assertEqual(row["checkId"], check_id)
            self.assertEqual(row["status"], "passed")
            self.assertEqual(row["exitCode"], 0)
            self.assertNotIn("nativeGateWitnessRef", row)
            self.assertFalse(gate_path.exists())
            self.assertFalse(source_path.exists())


if __name__ == "__main__":
    unittest.main()
