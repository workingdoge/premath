#!/usr/bin/env python3
"""Unit tests for the shared required-decision-verify client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_decision_verify_client import (
    RequiredDecisionVerifyError,
    run_required_decision_verify,
)


class RequiredDecisionVerifyClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "errors": [],
            "derived": {
                "decision": "accept",
                "projectionDigest": "proj1_demo",
                "requiredChecks": ["baseline"],
            },
        }

    def test_run_required_decision_verify_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-decision-verify"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_decision_verify_client.subprocess.run", return_value=completed):
            payload = run_required_decision_verify(Path("."), {"decision": {}})
        self.assertEqual(payload["errors"], [])
        self.assertEqual(payload["derived"]["decision"], "accept")

    def test_run_required_decision_verify_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-decision-verify"],
            returncode=2,
            stdout="",
            stderr="required_decision_verify_invalid: bad input\n",
        )
        with patch("required_decision_verify_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredDecisionVerifyError) as exc:
                run_required_decision_verify(Path("."), {"decision": {}})
        self.assertEqual(exc.exception.failure_class, "required_decision_verify_invalid")

    def test_run_required_decision_verify_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = {"errors": []}
        first = subprocess.CompletedProcess(
            args=["premath", "required-decision-verify"],
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
                "required-decision-verify",
            ],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch(
            "required_decision_verify_client.resolve_premath_cli",
            return_value=["/tmp/premath"],
        ):
            with patch(
                "required_decision_verify_client.subprocess.run",
                side_effect=[first, second],
            ) as run_mock:
                payload = run_required_decision_verify(Path("."), {"decision": {}})
        self.assertEqual(payload["derived"]["projectionDigest"], "proj1_demo")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
