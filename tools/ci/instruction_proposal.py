#!/usr/bin/env python3
"""Shared LLM proposal parsing/canonicalization for instruction tooling."""

from __future__ import annotations

import hashlib
import json
from typing import Any, Dict, List, Optional

PROPOSAL_KINDS = {"value", "derivation", "refinementPlan"}
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
GATE_FAILURE_TO_LAW_REF = {
    "stability_failure": "GATE-3.1",
    "locality_failure": "GATE-3.2",
    "descent_failure": "GATE-3.3",
    "glue_non_contractible": "GATE-3.4",
    "adjoint_triple_coherence_failure": "GATE-3.5",
}
REFINEMENT_OBLIGATION_HINTS = {
    "adjoint_triangle": "hint:adjoint_triangle",
    "beck_chevalley_sigma": "hint:beck_chevalley_sigma",
    "beck_chevalley_pi": "hint:beck_chevalley_pi",
    "refinement_invariance": "hint:refinement_invariance",
}


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def compute_proposal_digest(canonical_proposal: Dict[str, Any]) -> str:
    return "prop1_" + stable_hash(canonical_proposal)


def compute_proposal_kcir_ref(canonical_proposal: Dict[str, Any]) -> str:
    payload = {
        "kind": "kcir.proposal.v1",
        "canonicalProposal": canonical_proposal,
    }
    return "kcir1_" + stable_hash(payload)


class ProposalValidationError(ValueError):
    """Validation error with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        super().__init__(message)


def _ensure_non_empty_string(value: Any, label: str, failure_class: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ProposalValidationError(failure_class, f"{label} must be a non-empty string")
    return value.strip()


def canonicalize_proposal(raw: Any) -> Dict[str, Any]:
    if not isinstance(raw, dict):
        raise ProposalValidationError("proposal_invalid_shape", "proposal must be an object")

    proposal_kind = _ensure_non_empty_string(raw.get("proposalKind"), "proposal.proposalKind", "proposal_invalid_kind")
    if proposal_kind not in PROPOSAL_KINDS:
        raise ProposalValidationError(
            "proposal_invalid_kind",
            f"proposal.proposalKind must be one of {sorted(PROPOSAL_KINDS)}",
        )

    target_ctx_ref = _ensure_non_empty_string(raw.get("targetCtxRef"), "proposal.targetCtxRef", "proposal_invalid_target")

    target_judgment = raw.get("targetJudgment")
    if not isinstance(target_judgment, dict):
        raise ProposalValidationError(
            "proposal_invalid_target_judgment",
            "proposal.targetJudgment must be an object",
        )
    target_kind = target_judgment.get("kind")
    if target_kind not in {"obj", "mor"}:
        raise ProposalValidationError(
            "proposal_invalid_target_judgment",
            "proposal.targetJudgment.kind must be 'obj' or 'mor'",
        )
    target_shape = _ensure_non_empty_string(
        target_judgment.get("shape"),
        "proposal.targetJudgment.shape",
        "proposal_invalid_target_judgment",
    )

    binding = raw.get("binding")
    if not isinstance(binding, dict):
        raise ProposalValidationError("proposal_unbound_policy", "proposal.binding must be an object")
    normalizer_id = _ensure_non_empty_string(
        binding.get("normalizerId"),
        "proposal.binding.normalizerId",
        "proposal_unbound_policy",
    )
    policy_digest = _ensure_non_empty_string(
        binding.get("policyDigest"),
        "proposal.binding.policyDigest",
        "proposal_unbound_policy",
    )

    candidate_refs_raw = raw.get("candidateRefs", [])
    if not isinstance(candidate_refs_raw, list):
        raise ProposalValidationError("proposal_invalid_step", "proposal.candidateRefs must be a list")
    candidate_refs = []
    for idx, item in enumerate(candidate_refs_raw):
        candidate_refs.append(
            _ensure_non_empty_string(
                item,
                f"proposal.candidateRefs[{idx}]",
                "proposal_invalid_step",
            )
        )
    candidate_refs = sorted(set(candidate_refs))

    steps_raw = raw.get("steps", [])
    if not isinstance(steps_raw, list):
        raise ProposalValidationError("proposal_invalid_step", "proposal.steps must be a list")
    if proposal_kind == "derivation" and not steps_raw:
        raise ProposalValidationError("proposal_invalid_step", "proposal.steps must be non-empty for derivation proposals")
    if proposal_kind != "derivation" and steps_raw:
        raise ProposalValidationError(
            "proposal_invalid_step",
            "proposal.steps is only valid for derivation proposals",
        )

    steps = []
    for idx, step in enumerate(steps_raw):
        if not isinstance(step, dict):
            raise ProposalValidationError(
                "proposal_invalid_step",
                f"proposal.steps[{idx}] must be an object",
            )
        rule_id = _ensure_non_empty_string(
            step.get("ruleId"),
            f"proposal.steps[{idx}].ruleId",
            "proposal_invalid_step",
        )
        claim = _ensure_non_empty_string(
            step.get("claim"),
            f"proposal.steps[{idx}].claim",
            "proposal_invalid_step",
        )
        inputs_raw = step.get("inputs", [])
        outputs_raw = step.get("outputs", [])
        if not isinstance(inputs_raw, list) or not isinstance(outputs_raw, list):
            raise ProposalValidationError(
                "proposal_invalid_step",
                f"proposal.steps[{idx}].inputs/outputs must be lists",
            )
        inputs = [
            _ensure_non_empty_string(
                item,
                f"proposal.steps[{idx}].inputs[{jdx}]",
                "proposal_invalid_step",
            )
            for jdx, item in enumerate(inputs_raw)
        ]
        outputs = [
            _ensure_non_empty_string(
                item,
                f"proposal.steps[{idx}].outputs[{jdx}]",
                "proposal_invalid_step",
            )
            for jdx, item in enumerate(outputs_raw)
        ]
        steps.append(
            {
                "ruleId": rule_id,
                "inputs": inputs,
                "outputs": outputs,
                "claim": claim,
            }
        )

    canonical = {
        "proposalKind": proposal_kind,
        "targetCtxRef": target_ctx_ref,
        "targetJudgment": {
            "kind": target_kind,
            "shape": target_shape,
        },
        "candidateRefs": candidate_refs,
        "binding": {
            "normalizerId": normalizer_id,
            "policyDigest": policy_digest,
        },
    }
    if steps:
        canonical["steps"] = steps
    return canonical


def extract_instruction_proposal(envelope: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    proposal = envelope.get("proposal")
    llm_proposal = envelope.get("llmProposal")

    if proposal is not None and llm_proposal is not None:
        raise ProposalValidationError(
            "proposal_invalid_shape",
            "provide only one proposal field: proposal or llmProposal",
        )
    if proposal is not None:
        return proposal
    if llm_proposal is not None:
        return llm_proposal
    return None


def validate_instruction_proposal(envelope: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    raw = extract_instruction_proposal(envelope)
    if raw is None:
        return None

    canonical = canonicalize_proposal(raw)
    digest = compute_proposal_digest(canonical)
    kcir_ref = compute_proposal_kcir_ref(canonical)

    declared_digest = raw.get("proposalDigest")
    if declared_digest is not None:
        declared_digest = _ensure_non_empty_string(
            declared_digest,
            "proposal.proposalDigest",
            "proposal_nondeterministic",
        )
        if declared_digest != digest:
            raise ProposalValidationError(
                "proposal_nondeterministic",
                "proposal.proposalDigest does not match canonical payload digest",
            )

    declared_kcir_ref = raw.get("proposalKcirRef")
    if declared_kcir_ref is not None:
        declared_kcir_ref = _ensure_non_empty_string(
            declared_kcir_ref,
            "proposal.proposalKcirRef",
            "proposal_kcir_ref_mismatch",
        )
        if declared_kcir_ref != kcir_ref:
            raise ProposalValidationError(
                "proposal_kcir_ref_mismatch",
                "proposal.proposalKcirRef does not match canonical KCIR ref",
            )

    return {
        "canonical": canonical,
        "digest": digest,
        "kcirRef": kcir_ref,
    }


def _proposal_subject_ref(canonical_proposal: Dict[str, Any]) -> str:
    candidate_refs = canonical_proposal.get("candidateRefs", [])
    if isinstance(candidate_refs, list) and candidate_refs:
        first = candidate_refs[0]
        if isinstance(first, str) and first:
            return first

    steps = canonical_proposal.get("steps", [])
    if isinstance(steps, list):
        for step in reversed(steps):
            if not isinstance(step, dict):
                continue
            outputs = step.get("outputs", [])
            if isinstance(outputs, list) and outputs:
                first = outputs[0]
                if isinstance(first, str) and first:
                    return first

    target_ctx_ref = str(canonical_proposal.get("targetCtxRef", "ctx:unknown"))
    target_judgment = canonical_proposal.get("targetJudgment", {})
    target_kind = "obj"
    if isinstance(target_judgment, dict):
        kind = target_judgment.get("kind")
        if isinstance(kind, str) and kind:
            target_kind = kind
    return f"{target_ctx_ref}#{target_kind}"


def _proposal_has_discharge_candidate(canonical_proposal: Dict[str, Any]) -> bool:
    candidate_refs = canonical_proposal.get("candidateRefs", [])
    if isinstance(candidate_refs, list) and candidate_refs:
        return True
    steps = canonical_proposal.get("steps", [])
    if not isinstance(steps, list):
        return False
    for step in steps:
        if not isinstance(step, dict):
            continue
        outputs = step.get("outputs", [])
        if isinstance(outputs, list) and outputs:
            return True
    return False


def compile_proposal_obligations(canonical_proposal: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Compile a canonical proposal into deterministic checker obligations."""

    proposal_kind = str(canonical_proposal.get("proposalKind", ""))
    target_ctx_ref = str(canonical_proposal.get("targetCtxRef", ""))
    target_judgment = canonical_proposal.get("targetJudgment", {})
    target_kind = "obj"
    if isinstance(target_judgment, dict):
        kind = target_judgment.get("kind")
        if isinstance(kind, str) and kind:
            target_kind = kind
    candidate_refs = canonical_proposal.get("candidateRefs", [])
    candidate_count = len(candidate_refs) if isinstance(candidate_refs, list) else 0
    steps = canonical_proposal.get("steps", [])
    step_count = len(steps) if isinstance(steps, list) else 0

    obligation_kinds = ["stability", "locality"]
    if _proposal_has_discharge_candidate(canonical_proposal):
        obligation_kinds.append("descent_exists")
    else:
        obligation_kinds.append("ext_gap")

    if proposal_kind == "value" and candidate_count > 1:
        obligation_kinds.append("ext_ambiguous")
    if proposal_kind == "refinementPlan":
        obligation_kinds.extend(
            [
                "adjoint_triple",
                "adjoint_triangle",
                "beck_chevalley_sigma",
                "beck_chevalley_pi",
                "refinement_invariance",
            ]
        )

    subject_ref = _proposal_subject_ref(canonical_proposal)
    obligations: List[Dict[str, Any]] = []
    for idx, kind in enumerate(obligation_kinds):
        core = {
            "kind": kind,
            "ctx": {"ref": target_ctx_ref},
            "subject": {
                "kind": target_kind,
                "ref": subject_ref,
            },
            "details": {
                "proposalKind": proposal_kind,
                "candidateCount": candidate_count,
                "stepCount": step_count,
                "obligationIndex": idx,
            },
        }
        obligation_id = "obl1_" + stable_hash(core)
        obligations.append({"obligationId": obligation_id, **core})

    return obligations


def discharge_proposal_obligations(
    canonical_proposal: Dict[str, Any],
    obligations: List[Dict[str, Any]],
) -> Dict[str, Any]:
    """Deterministically discharge proposal obligations in normalized mode."""

    binding = canonical_proposal.get("binding", {})
    normalizer_id = ""
    policy_digest = ""
    if isinstance(binding, dict):
        normalizer_value = binding.get("normalizerId")
        policy_value = binding.get("policyDigest")
        if isinstance(normalizer_value, str):
            normalizer_id = normalizer_value
        if isinstance(policy_value, str):
            policy_digest = policy_value

    steps: List[Dict[str, Any]] = []
    failure_classes: List[str] = []
    candidate_refs = canonical_proposal.get("candidateRefs", [])
    candidate_ref_set = {
        item for item in candidate_refs if isinstance(item, str) and item
    } if isinstance(candidate_refs, list) else set()
    for idx, obligation in enumerate(obligations):
        obligation_id = f"obl-missing-{idx}"
        kind = ""
        if isinstance(obligation, dict):
            obligation_id_val = obligation.get("obligationId")
            kind_val = obligation.get("kind")
            if isinstance(obligation_id_val, str) and obligation_id_val:
                obligation_id = obligation_id_val
            if isinstance(kind_val, str):
                kind = kind_val

        failed = kind in {"ext_gap", "ext_ambiguous"}
        hint = REFINEMENT_OBLIGATION_HINTS.get(kind)
        if hint is not None and hint not in candidate_ref_set:
            failed = True
        step: Dict[str, Any] = {
            "obligationId": obligation_id,
            "kind": kind,
            "status": "failed" if failed else "passed",
            "mode": "normalized",
            "binding": {
                "normalizerId": normalizer_id,
                "policyDigest": policy_digest,
            },
        }
        if failed:
            failure_class = OBLIGATION_TO_GATE_FAILURE.get(kind, "descent_failure")
            step["failureClass"] = failure_class
            step["lawRef"] = GATE_FAILURE_TO_LAW_REF.get(failure_class, "GATE-3.3")
            if hint is not None and hint not in candidate_ref_set:
                step["missingHint"] = hint
            failure_classes.append(failure_class)
        steps.append(step)

    deduped_failures = sorted(set(failure_classes))
    return {
        "mode": "normalized",
        "binding": {
            "normalizerId": normalizer_id,
            "policyDigest": policy_digest,
        },
        "outcome": "accepted" if not deduped_failures else "rejected",
        "steps": steps,
        "failureClasses": deduped_failures,
    }
