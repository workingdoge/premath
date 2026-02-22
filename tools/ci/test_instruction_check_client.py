#!/usr/bin/env python3
"""Unit tests for the shared instruction-check client."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from instruction_check_client import (
    InstructionCheckError,
    InstructionWitnessError,
    run_instruction_check,
    run_instruction_witness,
)


class InstructionCheckClientTests(unittest.TestCase):
    def test_run_instruction_check_accepts_valid_payload(self) -> None:
        payload = {
            "intent": "verify",
            "scope": {"kind": "repo"},
            "normalizerId": "normalizer.ci.v1",
            "policyDigest": "pol1_demo",
            "requestedChecks": ["hk-check"],
            "instructionClassification": {"state": "typed", "kind": "ci.gate.check"},
            "executionDecision": {"state": "execute"},
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
                    "executionDecision": {"state": "execute"},
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

    def test_run_instruction_witness_accepts_valid_payload(self) -> None:
        payload = {
            "ciSchema": 1,
            "witnessKind": "ci.instruction.v1",
            "instructionId": "20260221T010000Z-ci-wiring-golden",
            "instructionRef": "tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json",
            "instructionDigest": "instr1_demo",
            "verdictClass": "accepted",
            "normalizerId": "normalizer.ci.v1",
            "policyDigest": "pol1_demo",
            "results": [],
            "failureClasses": [],
            "operationalFailureClasses": [],
            "semanticFailureClasses": [],
        }
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-witness"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            witness = run_instruction_witness(
                Path("."),
                Path("instructions/demo.json"),
                {
                    "instructionId": "20260221T010000Z-ci-wiring-golden",
                    "instructionRef": "instructions/demo.json",
                    "instructionDigest": "instr1_demo",
                    "squeakSiteProfile": "local",
                    "runStartedAt": "2026-02-22T00:00:00Z",
                    "runFinishedAt": "2026-02-22T00:00:01Z",
                    "runDurationMs": 1000,
                    "results": [],
                },
            )
        self.assertEqual(witness["witnessKind"], "ci.instruction.v1")

    def test_run_instruction_witness_propagates_failure_class(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-witness"],
            returncode=2,
            stdout="",
            stderr="instruction_runtime_invalid: bad runtime\n",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed):
            with self.assertRaises(InstructionWitnessError) as exc:
                run_instruction_witness(
                    Path("."),
                    Path("instructions/demo.json"),
                    {
                        "instructionId": "20260221T010000Z-ci-wiring-golden",
                        "instructionRef": "instructions/demo.json",
                        "instructionDigest": "instr1_demo",
                        "squeakSiteProfile": "local",
                        "runStartedAt": "2026-02-22T00:00:00Z",
                        "runFinishedAt": "2026-02-22T00:00:01Z",
                        "runDurationMs": 1000,
                        "results": [],
                    },
                )
        self.assertEqual(exc.exception.failure_class, "instruction_runtime_invalid")

    def test_run_instruction_witness_accepts_pre_execution_payload(self) -> None:
        payload = {
            "ciSchema": 1,
            "witnessKind": "ci.instruction.v1",
            "instructionId": "20260222T000001Z-invalid-normalizer",
            "instructionRef": "instructions/20260222T000001Z-invalid-normalizer.json",
            "instructionDigest": "instr1_demo",
            "verdictClass": "rejected",
            "normalizerId": None,
            "policyDigest": "pol1_demo",
            "results": [],
            "failureClasses": ["instruction_invalid_normalizer"],
            "operationalFailureClasses": ["instruction_invalid_normalizer"],
            "semanticFailureClasses": [],
            "rejectStage": "pre_execution",
            "rejectReason": "normalizerId must be a non-empty string",
        }
        completed = subprocess.CompletedProcess(
            args=["premath", "instruction-witness"],
            returncode=0,
            stdout=json.dumps(payload),
            stderr="",
        )
        with patch("instruction_check_client.subprocess.run", return_value=completed) as run_mock:
            witness = run_instruction_witness(
                Path("."),
                Path("instructions/demo.json"),
                {
                    "instructionId": "20260222T000001Z-invalid-normalizer",
                    "instructionRef": "instructions/demo.json",
                    "instructionDigest": "instr1_demo",
                    "squeakSiteProfile": "local",
                    "runStartedAt": "2026-02-22T00:00:00Z",
                    "runFinishedAt": "2026-02-22T00:00:01Z",
                    "runDurationMs": 1000,
                    "results": [],
                },
                pre_execution_failure_class="instruction_invalid_normalizer",
                pre_execution_reason="normalizerId must be a non-empty string",
            )
        self.assertEqual(witness["rejectStage"], "pre_execution")
        cmd = run_mock.call_args[0][0]
        self.assertIn("--pre-execution-failure-class", cmd)
        self.assertIn("instruction_invalid_normalizer", cmd)
        self.assertIn("--pre-execution-reason", cmd)

    def test_run_instruction_witness_rejects_partial_pre_execution_args(self) -> None:
        with self.assertRaises(InstructionWitnessError) as exc:
            run_instruction_witness(
                Path("."),
                Path("instructions/demo.json"),
                {
                    "instructionId": "20260222T000001Z-invalid-normalizer",
                    "instructionRef": "instructions/demo.json",
                    "instructionDigest": "instr1_demo",
                    "squeakSiteProfile": "local",
                    "runStartedAt": "2026-02-22T00:00:00Z",
                    "runFinishedAt": "2026-02-22T00:00:01Z",
                    "runDurationMs": 1000,
                    "results": [],
                },
                pre_execution_failure_class="instruction_invalid_normalizer",
            )
        self.assertEqual(exc.exception.failure_class, "instruction_runtime_invalid")


if __name__ == "__main__":
    unittest.main()
