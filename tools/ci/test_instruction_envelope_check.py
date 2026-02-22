#!/usr/bin/env python3
"""Unit tests for instruction envelope checking."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest.mock import patch

from check_instruction_envelope import validate_envelope
from proposal_check_client import ProposalCheckError


FIXTURE_PATH = (
    Path(__file__).resolve().parents[2]
    / "tests"
    / "ci"
    / "fixtures"
    / "instructions"
    / "20260221T010000Z-ci-wiring-golden.json"
)


class InstructionEnvelopeCheckTests(unittest.TestCase):
    def _fixture_payload(self) -> dict:
        return json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))

    def test_validate_envelope_uses_core_proposal_check(self) -> None:
        payload = self._fixture_payload()
        checked_proposal = {
            "canonical": payload["proposal"],
            "digest": "prop1_demo",
            "kcirRef": "kcir1_demo",
            "obligations": [],
            "discharge": {
                "mode": "normalized",
                "binding": payload["proposal"]["binding"],
                "outcome": "accepted",
                "steps": [],
                "failureClasses": [],
            },
        }
        with patch(
            "check_instruction_envelope.run_proposal_check",
            return_value=checked_proposal,
        ) as mocked:
            validate_envelope(
                Path("20260221T010000Z-ci-wiring-golden.json"),
                payload,
                Path("."),
            )
        mocked.assert_called_once()

    def test_validate_envelope_propagates_core_failure_class(self) -> None:
        payload = self._fixture_payload()
        with patch(
            "check_instruction_envelope.run_proposal_check",
            side_effect=ProposalCheckError("proposal_invalid_step", "missing ruleId"),
        ):
            with self.assertRaises(ValueError) as exc:
                validate_envelope(
                    Path("20260221T010000Z-ci-wiring-golden.json"),
                    payload,
                    Path("."),
                )
        self.assertTrue(str(exc.exception).startswith("proposal_invalid_step:"))

    def test_validate_envelope_without_proposal(self) -> None:
        payload = self._fixture_payload()
        payload.pop("proposal", None)
        validate_envelope(
            Path("20260221T010000Z-ci-wiring-golden.json"),
            payload,
            Path("."),
        )


if __name__ == "__main__":
    unittest.main()
