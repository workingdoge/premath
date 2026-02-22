#!/usr/bin/env python3
"""Unit tests for instruction proposal extraction shim."""

from __future__ import annotations

import unittest

from instruction_proposal import (
    ProposalValidationError,
    extract_instruction_proposal,
)


class InstructionProposalTests(unittest.TestCase):
    def _proposal(self) -> dict:
        return {
            "proposalKind": "value",
            "targetCtxRef": "ctx:demo",
            "targetJudgment": {
                "kind": "obj",
                "shape": "ObjNF:site",
            },
            "candidateRefs": ["obj:alpha"],
            "binding": {
                "normalizerId": "normalizer.ci.v1",
                "policyDigest": "pol1_demo",
            },
        }

    def test_extract_instruction_proposal_prefers_proposal_field(self) -> None:
        proposal = self._proposal()
        envelope = {"proposal": proposal}
        self.assertEqual(extract_instruction_proposal(envelope), proposal)

    def test_extract_instruction_proposal_accepts_legacy_alias(self) -> None:
        proposal = self._proposal()
        envelope = {"llmProposal": proposal}
        self.assertEqual(extract_instruction_proposal(envelope), proposal)

    def test_extract_instruction_proposal_rejects_dual_fields(self) -> None:
        proposal = self._proposal()
        envelope = {"proposal": proposal, "llmProposal": proposal}
        with self.assertRaises(ProposalValidationError) as exc:
            extract_instruction_proposal(envelope)
        self.assertEqual(exc.exception.failure_class, "proposal_invalid_shape")

    def test_extract_instruction_proposal_rejects_non_object(self) -> None:
        envelope = {"proposal": "not-an-object"}
        with self.assertRaises(ProposalValidationError) as exc:
            extract_instruction_proposal(envelope)
        self.assertEqual(exc.exception.failure_class, "proposal_invalid_shape")


if __name__ == "__main__":
    unittest.main()
