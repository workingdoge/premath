#!/usr/bin/env python3
"""
Execute doctrine-inf semantic boundary vectors.

These vectors validate law-level preserved/not-preserved boundary behavior and
claim-gated governance-profile fail-closed semantics.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Set

DEFAULT_FIXTURES = (
    Path(__file__).resolve().parents[2]
    / "tests"
    / "conformance"
    / "fixtures"
    / "doctrine-inf"
)
CAPABILITY_REGISTRY_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "CAPABILITY-REGISTRY.json"
)
CAPABILITY_REGISTRY_KIND = "premath.capability_registry.v1"

GOVERNANCE_PROFILE_CLAIM_ID = "profile.doctrine_inf_governance.v0"
REQUIRED_GUARDRAIL_STAGES = ("pre_flight", "input", "output")
VALID_OBSERVABILITY_MODES = {"dashboard", "internal_processor", "disabled"}
VALID_RISK_TIERS = {"low", "moderate", "high"}
REQUIRED_EVAL_LINEAGE_FIELDS = (
    "datasetLineageRef",
    "graderConfigLineageRef",
    "metricThresholdsRef",
)


def load_json(path: Path) -> Dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
    except FileNotFoundError as exc:
        raise ValueError(f"missing file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json: {path} ({exc})") from exc
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item:
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item)
    return out


def canonical_set(values: List[str]) -> List[str]:
    return sorted(set(values))


def load_profile_overlay_claims(registry_path: Path) -> Set[str]:
    payload = load_json(registry_path)
    if payload.get("schema") != 1:
        raise ValueError(f"{registry_path}: schema must be 1")
    if payload.get("registryKind") != CAPABILITY_REGISTRY_KIND:
        raise ValueError(
            f"{registry_path}: registryKind must be {CAPABILITY_REGISTRY_KIND!r}"
        )
    claims = payload.get("profileOverlayClaims", [])
    if not isinstance(claims, list):
        raise ValueError(f"{registry_path}: profileOverlayClaims must be a list")
    out: Set[str] = set()
    for idx, claim in enumerate(claims):
        if not isinstance(claim, str) or not claim:
            raise ValueError(f"{registry_path}: profileOverlayClaims[{idx}] must be a non-empty string")
        out.add(claim)
    return out


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "doctrine-inf":
        raise ValueError("manifest.suiteId must be 'doctrine-inf'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def evaluate_governance_profile(profile: Dict[str, Any]) -> Set[str]:
    claim_id = ensure_string(profile.get("claimId"), "governanceProfile.claimId")
    if claim_id != GOVERNANCE_PROFILE_CLAIM_ID:
        raise ValueError(
            "governanceProfile.claimId must be "
            f"{GOVERNANCE_PROFILE_CLAIM_ID!r}"
        )

    claimed = profile.get("claimed")
    if not isinstance(claimed, bool):
        raise ValueError("governanceProfile.claimed must be a boolean")
    if not claimed:
        return set()

    failures: Set[str] = set()

    policy = profile.get("policyProvenance")
    if not isinstance(policy, dict):
        raise ValueError("governanceProfile.policyProvenance must be an object")
    pinned = policy.get("pinned")
    if not isinstance(pinned, bool):
        raise ValueError("governanceProfile.policyProvenance.pinned must be a boolean")
    package_ref = policy.get("packageRef")
    expected_digest = policy.get("expectedDigest")
    bound_digest = policy.get("boundDigest")
    if not pinned:
        failures.add("governance.policy_package_unpinned")
    else:
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

    guardrail_stages = ensure_string_list(
        profile.get("guardrailStages"),
        "governanceProfile.guardrailStages",
    )
    missing_stage = any(stage not in guardrail_stages for stage in REQUIRED_GUARDRAIL_STAGES)
    if missing_stage:
        failures.add("governance.guardrail_stage_missing")
    elif tuple(guardrail_stages) != REQUIRED_GUARDRAIL_STAGES:
        failures.add("governance.guardrail_stage_order_invalid")

    eval_gate = profile.get("evalGate")
    if not isinstance(eval_gate, dict):
        raise ValueError("governanceProfile.evalGate must be an object")
    eval_passed = eval_gate.get("passed")
    if not isinstance(eval_passed, bool):
        raise ValueError("governanceProfile.evalGate.passed must be a boolean")
    if not eval_passed:
        failures.add("governance.eval_gate_unmet")

    eval_evidence = profile.get("evalEvidence")
    if not isinstance(eval_evidence, dict):
        failures.add("governance.eval_lineage_missing")
    else:
        for field in REQUIRED_EVAL_LINEAGE_FIELDS:
            value = eval_evidence.get(field)
            if not isinstance(value, str) or not value:
                failures.add("governance.eval_lineage_missing")

    observability_mode = ensure_string(
        profile.get("observabilityMode"),
        "governanceProfile.observabilityMode",
    )
    if observability_mode not in VALID_OBSERVABILITY_MODES:
        failures.add("governance.trace_mode_violation")

    risk_tier = profile.get("riskTier")
    if not isinstance(risk_tier, dict):
        raise ValueError("governanceProfile.riskTier must be an object")
    risk_tier_name = ensure_string(
        risk_tier.get("tier"),
        "governanceProfile.riskTier.tier",
    )
    control_bound = risk_tier.get("controlProfileBound")
    if not isinstance(control_bound, bool):
        raise ValueError("governanceProfile.riskTier.controlProfileBound must be a boolean")
    if risk_tier_name not in VALID_RISK_TIERS or not control_bound:
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


def evaluate_boundary_case(case: Dict[str, Any]) -> Dict[str, Any]:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    registry = set(ensure_string_list(artifacts.get("doctrineRegistry", []), "artifacts.doctrineRegistry"))
    if not registry:
        raise ValueError("artifacts.doctrineRegistry must be non-empty")

    declares = artifacts.get("destinationDeclares")
    if not isinstance(declares, dict):
        raise ValueError("artifacts.destinationDeclares must be an object")

    preserved = set(ensure_string_list(declares.get("preserved", []), "artifacts.destinationDeclares.preserved"))
    not_preserved = set(
        ensure_string_list(declares.get("notPreserved", []), "artifacts.destinationDeclares.notPreserved")
    )
    edge_morphisms = ensure_string_list(artifacts.get("edgeMorphisms", []), "artifacts.edgeMorphisms")

    failure_classes: Set[str] = set()

    if preserved & not_preserved:
        failure_classes.add("doctrine_declaration_overlap")

    unknown_declaration = sorted((preserved | not_preserved).difference(registry))
    if unknown_declaration:
        failure_classes.add("doctrine_unknown_morphism")

    unknown_edge = sorted(set(edge_morphisms).difference(registry))
    if unknown_edge:
        failure_classes.add("doctrine_unknown_morphism")

    for morphism in edge_morphisms:
        if morphism in not_preserved:
            failure_classes.add("doctrine_boundary_not_preserved")
            continue
        if morphism not in preserved:
            failure_classes.add("doctrine_boundary_not_declared_preserved")

    governance_profile = case.get("governanceProfile")
    if governance_profile is not None:
        if not isinstance(governance_profile, dict):
            raise ValueError("governanceProfile must be an object when provided")
        failure_classes.update(evaluate_governance_profile(governance_profile))

    if failure_classes:
        return {
            "result": "rejected",
            "failureClasses": sorted(failure_classes),
        }
    return {
        "result": "accepted",
        "failureClasses": [],
    }


def run(fixtures: Path, registry_path: Path, enforce_repo_claims: bool) -> int:
    vectors = validate_manifest(fixtures)
    profile_overlay_claims = load_profile_overlay_claims(registry_path)
    repo_governance_claimed = GOVERNANCE_PROFILE_CLAIM_ID in profile_overlay_claims
    errors: List[str] = []
    executed = 0
    skipped_repo_claim = 0
    executed_governance_claimed = 0

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "doctrine-inf":
                raise ValueError(f"{case_path}: suiteId must be 'doctrine-inf'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            governance_profile = case.get("governanceProfile")
            if governance_profile is not None:
                if not isinstance(governance_profile, dict):
                    raise ValueError("governanceProfile must be an object when provided")
                governance_case_claimed = governance_profile.get("claimed")
                if not isinstance(governance_case_claimed, bool):
                    raise ValueError("governanceProfile.claimed must be a boolean")
                if (
                    enforce_repo_claims
                    and governance_case_claimed
                    and not repo_governance_claimed
                ):
                    print(
                        f"[skip] doctrine-inf/{vector_id} "
                        "(governance profile not claimed in CAPABILITY-REGISTRY)"
                    )
                    skipped_repo_claim += 1
                    continue
                if governance_case_claimed:
                    executed_governance_claimed += 1

            got = evaluate_boundary_case(case)
            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(expect.get("expectedFailureClasses", []), f"{expect_path}: expectedFailureClasses")
            )

            got_failure_classes = canonical_set(got.get("failureClasses", []))
            if got.get("result") != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={got!r}"
                )

            print(f"[ok] doctrine-inf/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if enforce_repo_claims and repo_governance_claimed and executed_governance_claimed == 0:
        errors.append(
            "repository claims profile.doctrine_inf_governance.v0 but no governanceProfile.claimed=true vectors were executed"
        )

    if errors:
        print(
            "[doctrine-inf-run] FAIL "
            f"(vectors={executed}, skipped={skipped_repo_claim}, errors={len(errors)}, "
            f"repoGovernanceClaimed={str(repo_governance_claimed).lower()})"
        )
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[doctrine-inf-run] OK "
        f"(vectors={executed}, skipped={skipped_repo_claim}, "
        f"repoGovernanceClaimed={str(repo_governance_claimed).lower()})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run doctrine-inf semantic boundary vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Doctrine-inf fixture root (default: {DEFAULT_FIXTURES})",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=CAPABILITY_REGISTRY_PATH,
        help=f"Capability registry path for profile-overlay claim binding (default: {CAPABILITY_REGISTRY_PATH})",
    )
    parser.add_argument(
        "--ignore-repo-claims",
        action="store_true",
        help="Ignore CAPABILITY-REGISTRY profileOverlayClaims gating and execute all vectors as listed in the fixture manifest.",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[error] fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures}")
        return 2
    registry = args.registry
    if not registry.exists():
        print(f"[error] registry path does not exist: {registry}")
        return 2
    if not registry.is_file():
        print(f"[error] registry path is not a file: {registry}")
        return 2
    try:
        return run(
            fixtures=fixtures,
            registry_path=registry,
            enforce_repo_claims=not args.ignore_repo_claims,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-inf-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
