#!/usr/bin/env python3
"""Unit tests for shared instruction proposal validation."""

from __future__ import annotations

import unittest
from copy import deepcopy

from instruction_proposal import (
    ProposalValidationError,
    canonicalize_proposal,
    compute_proposal_digest,
    compute_proposal_kcir_ref,
    validate_instruction_proposal,
    validate_proposal_payload,
)


class InstructionProposalTests(unittest.TestCase):
    def _base_proposal(self) -> dict:
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

    def test_validate_proposal_payload_accepts_matching_declared_refs(self) -> None:
        proposal = self._base_proposal()
        canonical = canonicalize_proposal(proposal)
        proposal["proposalDigest"] = compute_proposal_digest(canonical)
        proposal["proposalKcirRef"] = compute_proposal_kcir_ref(canonical)

        validated = validate_proposal_payload(proposal)
        self.assertEqual(validated["canonical"], canonical)
        self.assertEqual(validated["digest"], proposal["proposalDigest"])
        self.assertEqual(validated["kcirRef"], proposal["proposalKcirRef"])

    def test_validate_proposal_payload_rejects_declared_digest_mismatch(self) -> None:
        proposal = self._base_proposal()
        proposal["proposalDigest"] = "prop1_deadbeef"

        with self.assertRaises(ProposalValidationError) as exc:
            validate_proposal_payload(proposal)
        self.assertEqual(exc.exception.failure_class, "proposal_nondeterministic")

    def test_validate_proposal_payload_rejects_declared_kcir_ref_mismatch(self) -> None:
        proposal = self._base_proposal()
        proposal["proposalKcirRef"] = "kcir1_deadbeef"

        with self.assertRaises(ProposalValidationError) as exc:
            validate_proposal_payload(proposal)
        self.assertEqual(exc.exception.failure_class, "proposal_kcir_ref_mismatch")

    def test_validate_instruction_proposal_accepts_legacy_alias(self) -> None:
        envelope = {"llmProposal": deepcopy(self._base_proposal())}
        validated = validate_instruction_proposal(envelope)
        self.assertIsNotNone(validated)
        self.assertEqual(validated["canonical"]["proposalKind"], "value")


if __name__ == "__main__":
    unittest.main()
