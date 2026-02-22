#!/usr/bin/env python3
"""Unit tests for the shared required-witness-decide client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from required_witness_decide_client import (
    RequiredWitnessDecideError,
    run_required_witness_decide,
)


class RequiredWitnessDecideClientTests(unittest.TestCase):
    @staticmethod
    def _payload() -> dict:
        return {
            "decisionKind": "ci.required.decision.v1",
            "decision": "accept",
            "reasonClass": "verified_accept",
            "projectionDigest": "proj1_demo",
            "typedCoreProjectionDigest": "ev1_demo",
            "authorityPayloadDigest": "proj1_demo",
            "normalizerId": "normalizer.ci.required.v1",
            "policyDigest": "ci-topos-v0",
            "requiredChecks": ["baseline"],
            "errors": [],
        }

    def test_run_required_witness_decide_accepts_valid_payload(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness-decide"],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_witness_decide_client.subprocess.run", return_value=completed):
            payload = run_required_witness_decide(Path("."), {"witness": {}})
        self.assertEqual(payload["decision"], "accept")
        self.assertEqual(payload["projectionDigest"], "proj1_demo")

    def test_run_required_witness_decide_accepts_legacy_alias_kind(self) -> None:
        payload = self._payload()
        payload["decisionKind"] = "ci.required.decision.v0"
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness-decide"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("required_witness_decide_client.subprocess.run", return_value=completed):
            out = run_required_witness_decide(Path("."), {"witness": {}})
        self.assertEqual(out["decisionKind"], "ci.required.decision.v1")

    def test_run_required_witness_decide_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "required-witness-decide"],
            returncode=2,
            stdout="",
            stderr="required_witness_decide_invalid: bad input\n",
        )
        with patch("required_witness_decide_client.subprocess.run", return_value=completed):
            with self.assertRaises(RequiredWitnessDecideError) as exc:
                run_required_witness_decide(Path("."), {"witness": {}})
        self.assertEqual(exc.exception.failure_class, "required_witness_decide_invalid")

    def test_run_required_witness_decide_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = {
            "decisionKind": "ci.required.decision.v1",
            "decision": "accept",
            "errors": [],
        }
        first = subprocess.CompletedProcess(
            args=["premath", "required-witness-decide"],
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
                "required-witness-decide",
            ],
            returncode=0,
            stdout=json.dumps(self._payload()),
            stderr="",
        )
        with patch("required_witness_decide_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch(
                "required_witness_decide_client.subprocess.run",
                side_effect=[first, second],
            ) as run_mock:
                payload = run_required_witness_decide(Path("."), {"witness": {}})
        self.assertEqual(payload["decision"], "accept")
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
