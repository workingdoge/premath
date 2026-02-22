#!/usr/bin/env python3
"""Unit tests for the shared instruction-check client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from instruction_check_client import InstructionCheckError, run_instruction_check


class InstructionCheckClientTests(unittest.TestCase):
    def test_run_instruction_check_accepts_valid_payload(self) -> None:
        payload = {
            "intent": "verify",
            "scope": {"kind": "repo"},
            "normalizerId": "normalizer.ci.v1",
            "policyDigest": "pol1_demo",
            "requestedChecks": ["hk-check"],
            "instructionClassification": {"state": "typed", "kind": "ci.gate.check"},
            "typingPolicy": {"allowUnknown": False},
            "capabilityClaims": [],
            "proposal": None,
        }
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-check"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            checked = run_instruction_check(Path("."), Path("instructions/demo.json"))
        self.assertEqual(checked["intent"], "verify")
        self.assertEqual(checked["requestedChecks"], ["hk-check"])

    def test_run_instruction_check_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-check"],
            returncode=2,
            stdout="",
            stderr="proposal_binding_mismatch: mismatch\n",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(InstructionCheckError) as exc:
                run_instruction_check(Path("."), Path("instructions/demo.json"))
        self.assertEqual(exc.exception.failure_class, "proposal_binding_mismatch")

    def test_run_instruction_check_rejects_invalid_json(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-check"],
            returncode=0,
            stdout="{not-json",
            stderr="",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(InstructionCheckError) as exc:
                run_instruction_check(Path("."), Path("instructions/demo.json"))
        self.assertEqual(exc.exception.failure_class, "instruction_envelope_invalid_shape")

    def test_run_instruction_check_rejects_missing_scope(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-check"],
            returncode=0,
            stdout=json.dumps(
                {
                    "intent": "verify",
                    "normalizerId": "normalizer.ci.v1",
                    "policyDigest": "pol1_demo",
                    "requestedChecks": ["hk-check"],
                    "instructionClassification": {"state": "typed", "kind": "ci.gate.check"},
                    "typingPolicy": {"allowUnknown": False},
                    "capabilityClaims": [],
                    "proposal": None,
                }
            ),
            stderr="",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(InstructionCheckError) as exc:
                run_instruction_check(Path("."), Path("instructions/demo.json"))
        self.assertEqual(exc.exception.failure_class, "instruction_envelope_invalid_shape")


if __name__ == "__main__":
    unittest.main()
