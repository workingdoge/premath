#!/usr/bin/env python3
"""Unit tests for instruction envelope checking."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest.mock import patch

from check_instruction_envelope import validate_envelope
from instruction_check_client import InstructionCheckError


FIXTURE_PATH = (
    Path(__file__).resolve().parents[2]
    / "tests"
    / "ci"
    / "fixtures"
    / "instructions"
    / "20260221T010000Z-ci-wiring-golden.json"
)


class InstructionEnvelopeCheckTests(unittest.TestCase):
    def test_validate_envelope_uses_core_instruction_check(self) -> None:
        payload = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        checked_instruction = {
            "intent": payload["intent"],
            "scope": payload["scope"],
            "normalizerId": payload["normalizerId"],
            "policyDigest": payload["policyDigest"],
            "requestedChecks": payload["requestedChecks"],
            "typingPolicy": {"allowUnknown": False},
            "capabilityClaims": [],
            "proposal": None,
        }
        with patch(
            "check_instruction_envelope.run_instruction_check",
            return_value=checked_instruction,
        ) as mocked:
            validate_envelope(
                Path("20260221T010000Z-ci-wiring-golden.json"),
                Path("."),
            )
        mocked.assert_called_once()

    def test_validate_envelope_propagates_core_failure_class(self) -> None:
        with patch(
            "check_instruction_envelope.run_instruction_check",
            side_effect=InstructionCheckError("proposal_invalid_step", "missing ruleId"),
        ):
            with self.assertRaises(ValueError) as exc:
                validate_envelope(
                    Path("20260221T010000Z-ci-wiring-golden.json"),
                    Path("."),
                )
        self.assertTrue(str(exc.exception).startswith("proposal_invalid_step:"))

    def test_validate_envelope_without_proposal(self) -> None:
        payload = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
        payload.pop("proposal", None)
        checked_instruction = {
            "intent": payload["intent"],
            "scope": payload["scope"],
            "normalizerId": payload["normalizerId"],
            "policyDigest": payload["policyDigest"],
            "requestedChecks": payload["requestedChecks"],
            "typingPolicy": {"allowUnknown": False},
            "capabilityClaims": [],
            "proposal": None,
        }
        with patch(
            "check_instruction_envelope.run_instruction_check",
            return_value=checked_instruction,
        ):
            validate_envelope(
                Path("20260221T010000Z-ci-wiring-golden.json"),
                Path("."),
            )


if __name__ == "__main__":
    unittest.main()
