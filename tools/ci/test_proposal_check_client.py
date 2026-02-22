#!/usr/bin/env python3
"""Unit tests for the shared proposal-check client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from proposal_check_client import ProposalCheckError, run_proposal_check


class ProposalCheckClientTests(unittest.TestCase):
    @staticmethod
    def _valid_payload() -> dict:
        return {
            "canonical": {
                "proposalKind": "value",
                "targetCtxRef": "ctx/demo",
                "targetJudgment": {"kind": "obj", "shape": "Type"},
                "candidateRefs": [],
                "binding": {
                    "normalizerId": "normalizer.ci.v1",
                    "policyDigest": "pol1_demo",
                },
            },
            "digest": "prop1_demo",
            "kcirRef": "kcir1_demo",
            "obligations": [],
            "discharge": {
                "mode": "normalized",
                "binding": {
                    "normalizerId": "normalizer.ci.v1",
                    "policyDigest": "pol1_demo",
                },
                "outcome": "accepted",
                "steps": [],
                "failureClasses": [],
            },
        }

    def test_run_proposal_check_accepts_valid_payload(self) -> None:
        payload = self._valid_payload()
        completed = subprocess.CompletedProcess(
            args=["premath", "proposal-check"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )

        with patch("proposal_check_client.subprocess.run", return_value=completed):
            checked = run_proposal_check(Path("."), {"proposalKind": "value"})

        self.assertEqual(checked, payload)

    def test_run_proposal_check_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "proposal-check"],
            returncode=2,
            stdout="",
            stderr="proposal_invalid_step: missing ruleId\n",
        )

        with patch("proposal_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(ProposalCheckError) as exc:
                run_proposal_check(Path("."), {"proposalKind": "derivation"})

        self.assertEqual(exc.exception.failure_class, "proposal_invalid_step")

    def test_run_proposal_check_rejects_invalid_json(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "proposal-check"],
            returncode=0,
            stdout="not-json",
            stderr="",
        )

        with patch("proposal_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(ProposalCheckError) as exc:
                run_proposal_check(Path("."), {"proposalKind": "value"})

        self.assertEqual(exc.exception.failure_class, "proposal_invalid_shape")

    def test_run_proposal_check_rejects_missing_kcir_ref(self) -> None:
        payload = self._valid_payload()
        payload.pop("kcirRef", None)
        completed = subprocess.CompletedProcess(
            args=["premath", "proposal-check"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )

        with patch("proposal_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(ProposalCheckError) as exc:
                run_proposal_check(Path("."), {"proposalKind": "value"})

        self.assertEqual(exc.exception.failure_class, "proposal_kcir_ref_mismatch")

    def test_run_proposal_check_retries_on_stale_local_payload_shape(self) -> None:
        stale_payload = self._valid_payload()
        stale_payload.pop("kcirRef", None)
        first = subprocess.CompletedProcess(
            args=["premath", "proposal-check"],
            returncode=0,
            stdout=json.dumps(stale_payload),
            stderr="",
        )
        second_payload = self._valid_payload()
        second = subprocess.CompletedProcess(
            args=["cargo", "run", "--package", "premath-cli", "--", "proposal-check"],
            returncode=0,
            stdout=json.dumps(second_payload),
            stderr="",
        )

        with patch("proposal_check_client.resolve_premath_cli", return_value=["/tmp/premath"]):
            with patch(
                "proposal_check_client.subprocess.run",
                side_effect=[first, second],
            ) as run_mock:
                checked = run_proposal_check(Path("."), {"proposalKind": "value"})

        self.assertEqual(checked, second_payload)
        self.assertEqual(run_mock.call_count, 2)


if __name__ == "__main__":
    unittest.main()
