#!/usr/bin/env python3
"""Deterministic governance promotion-gate evaluator for CI wrappers."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Dict, Sequence, Set, Tuple

GOVERNANCE_PROFILE_CLAIM_ID = "profile.doctrine_inf_governance.v0"
CAPABILITY_REGISTRY_REL_PATH = Path("specs/premath/draft/CAPABILITY-REGISTRY.json")
DEFAULT_PROMOTION_EVIDENCE_REL_PATH = Path("artifacts/ciwitness/governance-promotion.json")

REQUIRED_GUARDRAIL_STAGES: Tuple[str, ...] = ("pre_flight", "input", "output")
VALID_OBSERVABILITY_MODES = {"dashboard", "internal_processor", "disabled"}
VALID_RISK_TIERS = {"low", "moderate", "high"}
REQUIRED_EVAL_LINEAGE_FIELDS: Tuple[str, ...] = (
    "datasetLineageRef",
    "graderConfigLineageRef",
    "metricThresholdsRef",
)
_PASS_GRADES = {"pass", "accepted", "ok"}


def _is_truthy_env(var_name: str) -> bool:
    raw = os.environ.get(var_name, "").strip().lower()
    return raw in {"1", "true", "yes", "on"}


def _load_json_object(path: Path) -> Dict[str, Any] | None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, OSError):
        return None
    if not isinstance(payload, dict):
        return None
    return payload


def _load_profile_overlay_claims(repo_root: Path) -> Set[str] | None:
    payload = _load_json_object(repo_root / CAPABILITY_REGISTRY_REL_PATH)
    if payload is None:
        return None
    claims_raw = payload.get("profileOverlayClaims", [])
    if not isinstance(claims_raw, list):
        return None
    claims: Set[str] = set()
    for item in claims_raw:
        if isinstance(item, str) and item.strip():
            claims.add(item.strip())
    return claims


def _evaluate_governance_profile(profile: Dict[str, Any]) -> Set[str]:
    failures: Set[str] = set()

    claim_id = profile.get("claimId")
    claimed = profile.get("claimed")
    if not isinstance(claim_id, str) or not claim_id:
        failures.add("governance.eval_lineage_missing")
    elif claim_id != GOVERNANCE_PROFILE_CLAIM_ID:
        failures.add("governance.eval_lineage_missing")
    if claimed is not True:
        failures.add("governance.eval_lineage_missing")

    policy = profile.get("policyProvenance")
    if not isinstance(policy, dict):
        failures.add("governance.policy_package_unpinned")
    else:
        pinned = policy.get("pinned")
        package_ref = policy.get("packageRef")
        expected_digest = policy.get("expectedDigest")
        bound_digest = policy.get("boundDigest")
        if not isinstance(pinned, bool) or not pinned:
            failures.add("governance.policy_package_unpinned")
        if not isinstance(package_ref, str) or not package_ref:
            failures.add("governance.policy_package_unpinned")
        if not isinstance(expected_digest, str) or not expected_digest:
            failures.add("governance.policy_package_unpinned")
        if not isinstance(bound_digest, str) or not bound_digest:
            failures.add("governance.policy_package_unpinned")
        if (
            isinstance(expected_digest, str)
            and expected_digest
            and isinstance(bound_digest, str)
            and bound_digest
            and expected_digest != bound_digest
        ):
            failures.add("governance.policy_package_mismatch")

    guardrail_stages = profile.get("guardrailStages")
    if not isinstance(guardrail_stages, list) or not all(
        isinstance(item, str) and item for item in guardrail_stages
    ):
        failures.add("governance.guardrail_stage_missing")
    else:
        stages = [item for item in guardrail_stages if isinstance(item, str)]
        if any(stage not in stages for stage in REQUIRED_GUARDRAIL_STAGES):
            failures.add("governance.guardrail_stage_missing")
        elif tuple(stages) != REQUIRED_GUARDRAIL_STAGES:
            failures.add("governance.guardrail_stage_order_invalid")

    eval_gate = profile.get("evalGate")
    if not isinstance(eval_gate, dict):
        failures.add("governance.eval_gate_unmet")
    else:
        passed = eval_gate.get("passed")
        if not isinstance(passed, bool):
            failures.add("governance.eval_gate_unmet")
        elif not passed:
            failures.add("governance.eval_gate_unmet")

    eval_evidence = profile.get("evalEvidence")
    if not isinstance(eval_evidence, dict):
        failures.add("governance.eval_lineage_missing")
    else:
        for field in REQUIRED_EVAL_LINEAGE_FIELDS:
            value = eval_evidence.get(field)
            if not isinstance(value, str) or not value:
                failures.add("governance.eval_lineage_missing")

    observability_mode = profile.get("observabilityMode")
    if not isinstance(observability_mode, str) or observability_mode not in VALID_OBSERVABILITY_MODES:
        failures.add("governance.trace_mode_violation")

    risk_tier = profile.get("riskTier")
    if not isinstance(risk_tier, dict):
        failures.add("governance.risk_tier_profile_missing")
    else:
        tier = risk_tier.get("tier")
        bound = risk_tier.get("controlProfileBound")
        if not isinstance(tier, str) or tier not in VALID_RISK_TIERS or bound is not True:
            failures.add("governance.risk_tier_profile_missing")

    self_evolution = profile.get("selfEvolution")
    if not isinstance(self_evolution, dict):
        failures.add("governance.self_evolution_retry_missing")
        failures.add("governance.self_evolution_escalation_missing")
        failures.add("governance.self_evolution_rollback_missing")
    else:
        max_attempts = self_evolution.get("maxAttempts")
        terminal_escalation = self_evolution.get("terminalEscalation")
        rollback_ref = self_evolution.get("rollbackRef")
        if not isinstance(max_attempts, int) or max_attempts < 1:
            failures.add("governance.self_evolution_retry_missing")
        if not isinstance(terminal_escalation, str) or not terminal_escalation:
            failures.add("governance.self_evolution_escalation_missing")
        if not isinstance(rollback_ref, str) or not rollback_ref:
            failures.add("governance.self_evolution_rollback_missing")

    return failures


def _evaluate_workflow_trace(trace_payload: Any) -> Set[str]:
    failures: Set[str] = set()
    if not isinstance(trace_payload, dict):
        failures.add("governance.eval_lineage_missing")
        return failures

    trace_ref = trace_payload.get("traceRef")
    if not isinstance(trace_ref, str) or not trace_ref.strip():
        failures.add("governance.eval_lineage_missing")

    score = trace_payload.get("score")
    threshold = trace_payload.get("threshold")
    if not isinstance(score, (int, float)) or not isinstance(threshold, (int, float)):
        failures.add("governance.eval_lineage_missing")
    elif float(score) < float(threshold):
        failures.add("governance.eval_gate_unmet")

    grade = trace_payload.get("grade")
    if isinstance(grade, str) and grade.strip() and grade.strip().lower() not in _PASS_GRADES:
        failures.add("governance.eval_gate_unmet")

    return failures


def _evaluate_adversarial_gate(adversarial_payload: Any) -> Set[str]:
    failures: Set[str] = set()
    if not isinstance(adversarial_payload, dict):
        failures.add("governance.eval_lineage_missing")
        return failures

    passed = adversarial_payload.get("passed")
    if not isinstance(passed, bool):
        failures.add("governance.eval_lineage_missing")
    elif not passed:
        failures.add("governance.eval_gate_unmet")

    report_ref = adversarial_payload.get("reportRef")
    if not isinstance(report_ref, str) or not report_ref.strip():
        failures.add("governance.eval_lineage_missing")

    return failures


def _resolve_promotion_evidence_path(repo_root: Path) -> Path:
    raw_path = os.environ.get("PREMATH_GOVERNANCE_PROMOTION_EVIDENCE", "").strip()
    if raw_path:
        path = Path(raw_path)
        if not path.is_absolute():
            return (repo_root / path).resolve()
        return path
    return (repo_root / DEFAULT_PROMOTION_EVIDENCE_REL_PATH).resolve()


def governance_failure_classes(repo_root: Path) -> Tuple[str, ...]:
    """Return deterministic governance-gate failure classes for CI pipeline routing.

    Gate semantics are claim-gated by `profile.doctrine_inf_governance.v0` from
    CAPABILITY-REGISTRY. If promotion intent is not asserted, returns empty.

    Environment controls:
    - PREMATH_GOVERNANCE_PROMOTION_REQUIRED=1|true|yes|on: fail closed with
      `governance.eval_lineage_missing` when evidence is absent/invalid.
    - PREMATH_GOVERNANCE_PROMOTION_EVIDENCE=<path>: override evidence location.
    """

    claims = _load_profile_overlay_claims(repo_root)
    required = _is_truthy_env("PREMATH_GOVERNANCE_PROMOTION_REQUIRED")
    if claims is None:
        if required:
            return ("governance.eval_lineage_missing",)
        return tuple()
    if GOVERNANCE_PROFILE_CLAIM_ID not in claims:
        return tuple()

    evidence_path = _resolve_promotion_evidence_path(repo_root)
    payload = _load_json_object(evidence_path)
    if payload is None:
        if required:
            return ("governance.eval_lineage_missing",)
        return tuple()

    promotion_intent = payload.get("promotionIntent")
    if not isinstance(promotion_intent, bool):
        return ("governance.eval_lineage_missing",)
    if not promotion_intent:
        return tuple()

    failures: Set[str] = set()
    profile = payload.get("governanceProfile")
    if not isinstance(profile, dict):
        failures.add("governance.eval_lineage_missing")
    else:
        failures.update(_evaluate_governance_profile(profile))

    failures.update(_evaluate_workflow_trace(payload.get("workflowTrace")))
    failures.update(_evaluate_adversarial_gate(payload.get("adversarialGate")))
    return tuple(sorted(failures))
