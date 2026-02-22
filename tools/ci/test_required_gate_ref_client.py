#!/usr/bin/env python3
"""Unit tests for the shared required-gate-ref client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_gate_ref_client import RequiredGateRefError, run_required_gate_ref


class RequiredGateRefClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "gateWitnessRef": {
                "checkId": "baseline",
                "artifactRelPath": "gates/proj1_demo/01-baseline.json",
                "sha256": "a" * 64,
                "source": "native",
                "runId": "run1_demo",
                "witnessKind": "gate",
                "result": "accepted",
                "failureClasses": [],
            },
            "gatePayload": None,
        }

    def test_run_required_gate_ref_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-gate-ref"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_gate_ref_client.subprocess.run", return_value=completed):
            payload = run_required_gate_ref(Path("."), {"checkId": "baseline"})
        self.assertEqual(payload["gateWitnessRef"]["checkId"], "baseline")

    def test_run_required_gate_ref_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-gate-ref"],
            returncode=2,
            stdout="",
            stderr="required_gate_ref_invalid: bad input\n",
        )
        with patch("required_gate_ref_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredGateRefError) as exc:
                run_required_gate_ref(Path("."), {"checkId": "baseline"})
        self.assertEqual(exc.exception.failure_class, "required_gate_ref_invalid")

    def test_run_required_gate_ref_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = {"gateWitnessRef": {"checkId": "baseline"}}
        first = subprocess.CompletedProcess(
            args=["premath", "required-gate-ref"],
            returncode=0,
            stdout=json.dumps(stale_payload),
            stderr="",
        )
        second = subprocess.CompletedProcess(
            args=[
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "required-gate-ref",
            ],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_gate_ref_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch("required_gate_ref_client.subprocess.run", side_effect=[first, second]) as run_mock:
                payload = run_required_gate_ref(Path("."), {"checkId": "baseline"})
        self.assertEqual(payload["gateWitnessRef"]["source"], "native")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
