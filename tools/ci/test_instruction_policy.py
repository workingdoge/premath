#!/usr/bin/env python3
"""Unit tests for instruction policy bindings."""

from __future__ import annotations

import unittest

from instruction_policy import (
    PolicyValidationError,
    validate_proposal_binding_matches_envelope,
    validate_requested_checks,
)

POLICY_CI = "pol1_4ba916ce38da5c5607eb7f41d963294b34b644deb1fa6d55e133b072ca001b39"
POLICY_SMOKE = "pol1_23a57a68a45e0c428868cce4b657206fc0bf100f4fd5b303eb0034ff29d92c9f"


class InstructionPolicyTests(unittest.TestCase):
    def test_validate_requested_checks_accepts_allowlisted(self) -> None:
        validate_requested_checks(POLICY_CI, ["hk-check", "conformance-run"], normalizer_id="normalizer.ci.v1")

    def test_validate_requested_checks_rejects_unknown_policy(self) -> None:
        with self.assertRaises(PolicyValidationError) as exc:
            validate_requested_checks("pol1_unknown", ["hk-check"])
        self.assertEqual(exc.exception.failure_class, "instruction_unknown_policy")

    def test_validate_requested_checks_rejects_disallowed_check(self) -> None:
        with self.assertRaises(PolicyValidationError) as exc:
            validate_requested_checks(POLICY_SMOKE, ["hk-check"], normalizer_id="normalizer.ci.v1")
        self.assertEqual(exc.exception.failure_class, "instruction_check_not_allowed")

    def test_validate_requested_checks_rejects_normalizer_mismatch(self) -> None:
        with self.assertRaises(PolicyValidationError) as exc:
            validate_requested_checks(POLICY_SMOKE, ["ci-wiring-check"], normalizer_id="normalizer.test.v1")
        self.assertEqual(exc.exception.failure_class, "instruction_normalizer_not_allowed")

    def test_validate_proposal_binding_matches_envelope(self) -> None:
        proposal = {
            "canonical": {
                "binding": {
                    "normalizerId": "normalizer.ci.v1",
                    "policyDigest": POLICY_CI,
                }
            }
        }
        validate_proposal_binding_matches_envelope(
            "normalizer.ci.v1",
            POLICY_CI,
            proposal,
        )

    def test_validate_proposal_binding_rejects_mismatch(self) -> None:
        proposal = {
            "canonical": {
                "binding": {
                    "normalizerId": "normalizer.other.v1",
                    "policyDigest": POLICY_CI,
                }
            }
        }
        with self.assertRaises(PolicyValidationError) as exc:
            validate_proposal_binding_matches_envelope(
                "normalizer.ci.v1",
                POLICY_CI,
                proposal,
            )
        self.assertEqual(exc.exception.failure_class, "proposal_binding_mismatch")


if __name__ == "__main__":
    unittest.main()
