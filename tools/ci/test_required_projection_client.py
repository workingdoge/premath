#!/usr/bin/env python3
"""Unit tests for the shared required-projection client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_projection_client import RequiredProjectionError, run_required_projection


class RequiredProjectionClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "schema": 1,
            "projectionPolicy": "ci-topos-v0",
            "projectionDigest": "proj1_demo",
            "changedPaths": ["README.md"],
            "requiredChecks": ["doctrine-check"],
            "docsOnly": True,
            "reasons": ["docs_only_doctrine_surface_touched"],
        }

    def test_run_required_projection_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-projection"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_projection_client.subprocess.run", return_value=completed):
            payload = run_required_projection(Path("."), {"changedPaths": ["README.md"]})
        self.assertEqual(payload["projectionDigest"], "proj1_demo")
        self.assertEqual(payload["requiredChecks"], ["doctrine-check"])

    def test_run_required_projection_accepts_legacy_alias_policy(self) -> None:
        payload = self._payload()
        payload["projectionPolicy"] = "ci-topos-v0-preview"
        completed = subprocess.CompletedProcess(
            args=["premath", "required-projection"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("required_projection_client.subprocess.run", return_value=completed):
            out = run_required_projection(Path("."), {"changedPaths": ["README.md"]})
        self.assertEqual(out["projectionPolicy"], "ci-topos-v0")

    def test_run_required_projection_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-projection"],
            returncode=2,
            stdout="",
            stderr="required_projection_invalid: bad input\n",
        )
        with patch("required_projection_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredProjectionError) as exc:
                run_required_projection(Path("."), {"changedPaths": ["README.md"]})
        self.assertEqual(exc.exception.failure_class, "required_projection_invalid")

    def test_run_required_projection_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = {"schema": 1, "projectionPolicy": "ci-topos-v0"}
        first = subprocess.CompletedProcess(
            args=["premath", "required-projection"],
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
                "required-projection",
            ],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_projection_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch("required_projection_client.subprocess.run", side_effect=[first, second]) as run_mock:
                payload = run_required_projection(Path("."), {"changedPaths": ["README.md"]})
        self.assertEqual(payload["projectionDigest"], "proj1_demo")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
