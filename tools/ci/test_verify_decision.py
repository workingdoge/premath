#!/usr/bin/env python3
"""Unit tests for verify_decision typed-authority reporting semantics."""

from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from unittest.mock import patch

import verify_decision


class VerifyDecisionTests(unittest.TestCase):
    def _write_fixture_files(self, root: Path) -> tuple[Path, Path, Path]:
        decision_path = root / "decision.json"
        witness_path = root / "witness.json"
        delta_path = root / "delta.json"

        decision_path.write_text(json.dumps({"decision": "accept"}) + "\n", encoding="utf-8")
        witness_path.write_text(json.dumps({"schema": 1}) + "\n", encoding="utf-8")
        delta_path.write_text(json.dumps({"schema": 1}) + "\n", encoding="utf-8")
        return decision_path, witness_path, delta_path

    def test_main_accepts_verified_decision_with_typed_authority(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-verify-decision-") as tmp:
            root = Path(tmp)
            decision_path, witness_path, delta_path = self._write_fixture_files(root)
            payload = {
                "errors": [],
                "derived": {
                    "decision": "accept",
                    "typedCoreProjectionDigest": "ev1_demo",
                    "authorityPayloadDigest": "proj1_demo",
                },
            }
            with patch("verify_decision.run_required_decision_verify", return_value=payload):
                argv = [
                    "verify_decision.py",
                    str(decision_path),
                    "--witness",
                    str(witness_path),
                    "--delta-snapshot",
                    str(delta_path),
                ]
                with patch("sys.argv", argv):
                    rc = verify_decision.main()
        self.assertEqual(rc, 0)

    def test_main_rejects_verified_decision_without_typed_authority(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-verify-decision-missing-") as tmp:
            root = Path(tmp)
            decision_path, witness_path, delta_path = self._write_fixture_files(root)
            payload = {
                "errors": [],
                "derived": {
                    "decision": "accept",
                    "projectionDigest": "proj1_demo",
                },
            }
            with patch("verify_decision.run_required_decision_verify", return_value=payload):
                argv = [
                    "verify_decision.py",
                    str(decision_path),
                    "--witness",
                    str(witness_path),
                    "--delta-snapshot",
                    str(delta_path),
                ]
                stderr = io.StringIO()
                with patch("sys.argv", argv):
                    with redirect_stderr(stderr):
                        rc = verify_decision.main()
        self.assertEqual(rc, 1)
        self.assertIn("missing typedCoreProjectionDigest", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
