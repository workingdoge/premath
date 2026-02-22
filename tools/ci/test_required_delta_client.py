#!/usr/bin/env python3
"""Unit tests for the shared required-delta client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_delta_client import RequiredDeltaError, run_required_delta


class RequiredDeltaClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "schema": 1,
            "deltaKind": "ci.required.delta.v1",
            "changedPaths": ["README.md"],
            "source": "git_diff",
            "fromRef": "origin/main",
            "toRef": "HEAD",
        }

    def test_run_required_delta_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-delta"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_delta_client.subprocess.run", return_value=completed):
            payload = run_required_delta(Path("."), {"repoRoot": "."})
        self.assertEqual(payload["changedPaths"], ["README.md"])
        self.assertEqual(payload["toRef"], "HEAD")

    def test_run_required_delta_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-delta"],
            returncode=2,
            stdout="",
            stderr="required_delta_invalid: bad input\n",
        )
        with patch("required_delta_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredDeltaError) as exc:
                run_required_delta(Path("."), {"repoRoot": "."})
        self.assertEqual(exc.exception.failure_class, "required_delta_invalid")

    def test_run_required_delta_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = {"schema": 1, "deltaKind": "ci.required.delta.v1"}
        first = subprocess.CompletedProcess(
            args=["premath", "required-delta"],
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
                "required-delta",
            ],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_delta_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch("required_delta_client.subprocess.run", side_effect=[first, second]) as run_mock:
                payload = run_required_delta(Path("."), {"repoRoot": "."})
        self.assertEqual(payload["source"], "git_diff")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
