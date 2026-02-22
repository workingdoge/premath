#!/usr/bin/env python3
"""Instruction proposal field extraction shim.

Authoritative proposal semantics are provided by core `premath proposal-check`.
This module remains as a minimal extraction helper for envelope compatibility.
"""

from __future__ import annotations

from typing import Any, Dict, Optional

class ProposalValidationError(ValueError):
    """Extraction/shape error with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        super().__init__(message)


def extract_instruction_proposal(envelope: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    proposal = envelope.get("proposal")
    llm_proposal = envelope.get("llmProposal")

    if proposal is not None and llm_proposal is not None:
        raise ProposalValidationError(
            "proposal_invalid_shape",
            "provide only one proposal field: proposal or llmProposal",
        )
    if proposal is not None:
        if not isinstance(proposal, dict):
            raise ProposalValidationError("proposal_invalid_shape", "proposal must be an object")
        return proposal
    if llm_proposal is not None:
        if not isinstance(llm_proposal, dict):
            raise ProposalValidationError("proposal_invalid_shape", "llmProposal must be an object")
        return llm_proposal
    return None
