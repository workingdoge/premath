#!/usr/bin/env python3
"""Instruction proposal field extraction shim.

Authoritative proposal semantics are provided by core `premath proposal-check`.
This module remains as a minimal extraction helper for envelope compatibility.
"""

from __future__ import annotations

from typing import Any, Dict, Optional

# Coherence contract currently introspects this map for obligation-key parity.
# Semantic authority remains in core `premath proposal-check`.
OBLIGATION_TO_GATE_FAILURE = {
    "stability": "stability_failure",
    "locality": "locality_failure",
    "descent_exists": "descent_failure",
    "descent_contractible": "glue_non_contractible",
    "adjoint_triangle": "adjoint_triple_coherence_failure",
    "beck_chevalley_sigma": "adjoint_triple_coherence_failure",
    "beck_chevalley_pi": "adjoint_triple_coherence_failure",
    "refinement_invariance": "stability_failure",
    "adjoint_triple": "adjoint_triple_coherence_failure",
    "ext_gap": "descent_failure",
    "ext_ambiguous": "glue_non_contractible",
}


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
