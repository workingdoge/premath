#!/usr/bin/env python3
"""Unit tests for the shared required-witness client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_witness_client import RequiredWitnessError, run_required_witness


class RequiredWitnessClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "ciSchema": 1,
            "witnessKind": "ci.required.v1",
            "projectionPolicy": "ci-topos-v0",
            "projectionDigest": "proj1_demo",
            "changedPaths": ["README.md"],
            "requiredChecks": ["baseline"],
            "executedChecks": ["baseline"],
            "results": [
                {
                    "checkId": "baseline",
                    "status": "passed",
                    "exitCode": 0,
                    "durationMs": 25,
                }
            ],
            "gateWitnessRefs": [],
            "verdictClass": "accepted",
            "operationalFailureClasses": [],
            "semanticFailureClasses": [],
            "failureClasses": [],
            "docsOnly": False,
            "reasons": ["kernel_or_ci_or_governance_change"],
            "deltaSource": "explicit",
            "fromRef": "origin/main",
            "toRef": "HEAD",
            "policyDigest": "ci-topos-v0",
            "squeakSiteProfile": "local",
            "runStartedAt": "2026-02-22T00:00:00Z",
            "runFinishedAt": "2026-02-22T00:00:01Z",
            "runDurationMs": 1000,
        }

    def test_run_required_witness_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_witness_client.subprocess.run", return_value=completed):
            witness = run_required_witness(Path("."), {"projectionDigest": "proj1_demo"})
        self.assertEqual(witness["witnessKind"], "ci.required.v1")
        self.assertEqual(witness["verdictClass"], "accepted")

    def test_run_required_witness_accepts_legacy_alias_kind(self) -> None:
        payload = self._payload()
        payload["witnessKind"] = "ci.required.v0"
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("required_witness_client.subprocess.run", return_value=completed):
            witness = run_required_witness(Path("."), {"projectionDigest": "proj1_demo"})
        self.assertEqual(witness["witnessKind"], "ci.required.v1")

    def test_run_required_witness_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness"],
            returncode=2,
            stdout="",
            stderr="required_witness_runtime_invalid: bad runtime\n",
        )
        with patch("required_witness_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredWitnessError) as exc:
                run_required_witness(Path("."), {"projectionDigest": "proj1_demo"})
        self.assertEqual(exc.exception.failure_class, "required_witness_runtime_invalid")

    def test_run_required_witness_rejects_invalid_json(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness"],
            returncode=0,
            stdout="{not-json",
            stderr="",
        )
        with patch("required_witness_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredWitnessError) as exc:
                run_required_witness(Path("."), {"projectionDigest": "proj1_demo"})
        self.assertEqual(exc.exception.failure_class, "required_witness_runtime_invalid")

    def test_run_required_witness_retries_on_stale_local_payload_shape(self) -> None:
        stale = self._payload()
        stale.pop("witnessKind", None)
        first = subprocess.CompletedProcess(
            args=["premath", "required-witness"],
            returncode=0,
            stdout=json.dumps(stale),
            stderr="",
        )
        second = subprocess.CompletedProcess(
            args=["cargo", "run", "--package", "premath-cli", "--", "required-witness"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )

        with patch("required_witness_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch(
                "required_witness_client.subprocess.run",
                side_effect=[first, second],
            ) as run_mock:
                witness = run_required_witness(Path("."), {"projectionDigest": "proj1_demo"})
        self.assertEqual(witness["witnessKind"], "ci.required.v1")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
