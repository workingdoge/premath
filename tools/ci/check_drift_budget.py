#!/usr/bin/env python3
"""Fail-closed drift-budget sentinel across docs/contracts/checkers."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple


SCHEMA = 1
CHECK_KIND = "ci.drift_budget.v1"

DRIFT_CLASS_SPEC_INDEX = "spec_index_capability_map_drift"
DRIFT_CLASS_PROFILE_OVERLAYS = "profile_overlay_claim_drift"
DRIFT_CLASS_LANE_BINDINGS = "control_plane_lane_binding_drift"
DRIFT_CLASS_KCIR_MAPPINGS = "control_plane_kcir_mapping_drift"
DRIFT_CLASS_RUNTIME_ROUTE_BINDINGS = "runtime_route_binding_drift"
DRIFT_CLASS_REQUIRED_OBLIGATIONS = "coherence_required_obligation_drift"
DRIFT_CLASS_SIGPI_NOTATION = "sigpi_notation_drift"
DRIFT_CLASS_CACHE_CLOSURE = "coherence_cache_input_closure_drift"
DRIFT_CLASS_TOPOLOGY_BUDGET = "topology_budget_drift"
WARN_CLASS_TOPOLOGY_BUDGET = "topology_budget_watch"

TOPOLOGY_BUDGET_SCHEMA = 1
TOPOLOGY_BUDGET_KIND = "premath.topology_budget.v1"

_DOC_MAP_RE = re.compile(r"- `([^`]+)`\s+\(for `([^`]+)`\)")
_PROFILE_CLAIM_RE = re.compile(r"`(profile\.[a-z0-9_.]+)`")
_SIGPI_ALIAS_RE = re.compile(r"\bSig/Pi\b", re.IGNORECASE)
CODE_REF_RE = re.compile(r"`([^`]+)`")

SIGPI_NORMATIVE_DOCS: Tuple[str, ...] = (
    "specs/premath/draft/SPEC-INDEX.md",
    "specs/premath/draft/UNIFICATION-DOCTRINE.md",
    "specs/premath/profile/ADJOINTS-AND-SITES.md",
)

CACHE_CLOSURE_REQUIRED_PATHS: Tuple[str, ...] = (
    "specs/premath/draft/COHERENCE-CONTRACT.json",
    "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
    "tools/ci/control_plane_contract.py",
    "crates/premath-coherence/src",
    "crates/premath-cli/src/commands/coherence_check.rs",
)


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fail-closed drift-budget checks across docs/contracts/checkers."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    parser.add_argument(
        "--coherence-json",
        type=Path,
        default=None,
        help="Optional precomputed coherence-check witness JSON.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit deterministic JSON payload.",
    )
    parser.add_argument(
        "--topology-budget",
        type=Path,
        default=None,
        help=(
            "Optional topology-budget contract path. "
            "Default: specs/process/TOPOLOGY-BUDGET.json under repo root."
        ),
    )
    return parser.parse_args()


def load_json(path: Path) -> Dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: root must be an object")
    return payload


def extract_heading_section(text: str, heading_prefix: str) -> str:
    heading_re = re.compile(rf"^### {re.escape(heading_prefix)}.*?$", re.MULTILINE)
    match = heading_re.search(text)
    if match is None:
        raise ValueError(f"missing heading: {heading_prefix!r}")
    section_start = match.end()
    tail = text[section_start:]
    next_heading = re.search(r"^### ", tail, re.MULTILINE)
    if next_heading is None:
        return tail
    return tail[: next_heading.start()]


def parse_spec_index_capability_doc_map(spec_index_path: Path) -> Dict[str, str]:
    text = spec_index_path.read_text(encoding="utf-8")
    section_54 = extract_heading_section(text, "5.4")
    out: Dict[str, str] = {}
    for doc_ref, capability_id in _DOC_MAP_RE.findall(section_54):
        out[doc_ref] = capability_id
    if not out:
        raise ValueError(f"{spec_index_path}: §5.4 capability doc map is empty")
    return out


@dataclass(frozen=True)
class CapabilityRegistryContract:
    executable_capabilities: List[str]
    profile_overlay_claims: List[str]


def parse_capability_registry(registry_path: Path) -> CapabilityRegistryContract:
    payload = load_json(registry_path)
    capabilities = payload.get("executableCapabilities")
    if not isinstance(capabilities, list) or not capabilities:
        raise ValueError(f"{registry_path}: executableCapabilities must be a non-empty list")
    out: List[str] = []
    for idx, value in enumerate(capabilities):
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{registry_path}: executableCapabilities[{idx}] must be non-empty")
        out.append(value.strip())
    if len(set(out)) != len(out):
        raise ValueError(f"{registry_path}: executableCapabilities must not contain duplicates")
    overlay_claims_raw = payload.get("profileOverlayClaims", [])
    if not isinstance(overlay_claims_raw, list):
        raise ValueError(f"{registry_path}: profileOverlayClaims must be a list")
    overlay_claims: List[str] = []
    for idx, value in enumerate(overlay_claims_raw):
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{registry_path}: profileOverlayClaims[{idx}] must be non-empty")
        overlay_claims.append(value.strip())
    if len(set(overlay_claims)) != len(overlay_claims):
        raise ValueError(f"{registry_path}: profileOverlayClaims must not contain duplicates")
    return CapabilityRegistryContract(
        executable_capabilities=out,
        profile_overlay_claims=overlay_claims,
    )


def parse_conformance_profile_overlay_claims(conformance_path: Path) -> List[str]:
    text = conformance_path.read_text(encoding="utf-8")
    section_24 = extract_heading_section(text, "2.4")
    return sorted(set(_PROFILE_CLAIM_RE.findall(section_24)))


def parse_conditional_capability_docs(coherence_contract: Dict[str, Any]) -> Dict[str, str]:
    docs = coherence_contract.get("conditionalCapabilityDocs")
    if not isinstance(docs, list) or not docs:
        raise ValueError("coherence contract conditionalCapabilityDocs must be a non-empty list")
    out: Dict[str, str] = {}
    for idx, row in enumerate(docs):
        if not isinstance(row, dict):
            raise ValueError(f"conditionalCapabilityDocs[{idx}] must be an object")
        doc_ref = row.get("docRef")
        capability_id = row.get("capabilityId")
        if not isinstance(doc_ref, str) or not doc_ref.strip():
            raise ValueError(f"conditionalCapabilityDocs[{idx}].docRef must be non-empty")
        if not isinstance(capability_id, str) or not capability_id.strip():
            raise ValueError(
                f"conditionalCapabilityDocs[{idx}].capabilityId must be non-empty"
            )
        out[doc_ref.strip()] = capability_id.strip()
    return out


def parse_required_obligation_ids(coherence_contract: Dict[str, Any]) -> List[str]:
    obligations = coherence_contract.get("obligations")
    if not isinstance(obligations, list) or not obligations:
        raise ValueError("coherence contract obligations must be a non-empty list")
    out: List[str] = []
    for idx, row in enumerate(obligations):
        if not isinstance(row, dict):
            raise ValueError(f"obligations[{idx}] must be an object")
        obligation_id = row.get("id")
        if not isinstance(obligation_id, str) or not obligation_id.strip():
            raise ValueError(f"obligations[{idx}].id must be a non-empty string")
        out.append(obligation_id.strip())
    return out


def parse_required_bidir_obligations(coherence_contract: Dict[str, Any]) -> List[str]:
    values = coherence_contract.get("requiredBidirObligations")
    if not isinstance(values, list) or not values:
        raise ValueError("coherence contract requiredBidirObligations must be non-empty")
    out: List[str] = []
    for idx, value in enumerate(values):
        if not isinstance(value, str) or not value.strip():
            raise ValueError(
                f"requiredBidirObligations[{idx}] must be a non-empty string"
            )
        out.append(value.strip())
    return out


def import_module_from_path(module_name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(module_name, path)
    if spec is None or spec.loader is None:
        raise ValueError(f"failed to load module spec: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)  # type: ignore[call-arg]
    return module


def run_coherence_check(repo_root: Path, contract_path: Path) -> Dict[str, Any]:
    cmd = [
        "cargo",
        "run",
        "--package",
        "premath-cli",
        "--",
        "coherence-check",
        "--contract",
        str(contract_path),
        "--repo-root",
        str(repo_root),
        "--json",
    ]
    completed = subprocess.run(
        cmd,
        cwd=repo_root,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise ValueError(
            "coherence-check command failed: "
            + (completed.stderr.strip() or completed.stdout.strip() or "unknown error")
        )
    payload = json.loads(completed.stdout)
    if not isinstance(payload, dict):
        raise ValueError("coherence-check JSON output must be an object")
    return payload


def obligation_details(witness: Dict[str, Any], obligation_id: str) -> Dict[str, Any]:
    obligations = witness.get("obligations")
    if not isinstance(obligations, list):
        raise ValueError("coherence witness obligations must be a list")
    for row in obligations:
        if not isinstance(row, dict):
            continue
        if row.get("obligationId") == obligation_id:
            details = row.get("details")
            if not isinstance(details, dict):
                raise ValueError(
                    f"coherence witness obligation {obligation_id} details must be an object"
                )
            return details
    raise ValueError(f"coherence witness missing obligation details for {obligation_id!r}")


def as_sorted_strings(values: Any) -> List[str]:
    if not isinstance(values, Iterable):
        return []
    out: List[str] = []
    for value in values:
        if isinstance(value, str) and value.strip():
            out.append(value.strip())
    return sorted(set(out))


def normalize_lane_artifact_kinds(values: Any) -> Dict[str, List[str]]:
    if not isinstance(values, dict):
        return {}
    out: Dict[str, List[str]] = {}
    for key, raw in values.items():
        if not isinstance(key, str) or not key.strip():
            continue
        out[key.strip()] = as_sorted_strings(raw)
    return out


def _normalize_kcir_mapping_row(row: Any) -> Dict[str, Any]:
    if not isinstance(row, dict):
        return {
            "sourceKind": "",
            "targetDomain": "",
            "targetKind": "",
            "identityFields": tuple(),
        }
    identity_fields = row.get("identityFields", ())
    if not isinstance(identity_fields, (list, tuple)):
        identity_fields = ()
    normalized_identity_fields: List[str] = []
    for value in identity_fields:
        if isinstance(value, str) and value.strip():
            normalized_identity_fields.append(value.strip())
    return {
        "sourceKind": str(row.get("sourceKind", "")).strip(),
        "targetDomain": str(row.get("targetDomain", "")).strip(),
        "targetKind": str(row.get("targetKind", "")).strip(),
        "identityFields": tuple(normalized_identity_fields),
    }


def normalize_kcir_mapping_table(values: Any) -> Dict[str, Dict[str, Any]]:
    if not isinstance(values, dict):
        return {}
    out: Dict[str, Dict[str, Any]] = {}
    for row_id, row in values.items():
        if not isinstance(row_id, str) or not row_id.strip():
            continue
        out[row_id.strip()] = _normalize_kcir_mapping_row(row)
    return out


def normalize_kcir_legacy_policy(values: Any) -> Dict[str, str]:
    if not isinstance(values, dict):
        return {}
    return {
        "mode": str(values.get("mode", "")).strip(),
        "authorityMode": str(values.get("authorityMode", "")).strip(),
        "supportUntilEpoch": str(values.get("supportUntilEpoch", "")).strip(),
        "failureClass": str(values.get("failureClass", "")).strip(),
    }


def parse_doctrine_operation_registry(registry_path: Path) -> Dict[str, Dict[str, Any]]:
    payload = load_json(registry_path)
    operations = payload.get("operations")
    if not isinstance(operations, list) or not operations:
        raise ValueError(f"{registry_path}: operations must be a non-empty list")
    out: Dict[str, Dict[str, Any]] = {}
    for idx, row in enumerate(operations):
        if not isinstance(row, dict):
            raise ValueError(f"{registry_path}: operations[{idx}] must be an object")
        operation_id = row.get("id")
        if not isinstance(operation_id, str) or not operation_id.strip():
            raise ValueError(f"{registry_path}: operations[{idx}].id must be non-empty")
        operation_id = operation_id.strip()
        if operation_id in out:
            raise ValueError(f"{registry_path}: duplicate operation id {operation_id!r}")
        out[operation_id] = {
            "path": str(row.get("path", "")).strip(),
            "morphisms": as_sorted_strings(row.get("morphisms", ())),
        }
    return out


def normalize_runtime_route_bindings(values: Any) -> Dict[str, Dict[str, Any]]:
    if not isinstance(values, dict):
        return {}
    out: Dict[str, Dict[str, Any]] = {}
    for route_id, raw in values.items():
        if not isinstance(route_id, str) or not route_id.strip():
            continue
        if not isinstance(raw, dict):
            continue
        operation_id = raw.get("operationId")
        if not isinstance(operation_id, str) or not operation_id.strip():
            continue
        out[route_id.strip()] = {
            "operationId": operation_id.strip(),
            "requiredMorphisms": as_sorted_strings(raw.get("requiredMorphisms", ())),
        }
    return out


def check_spec_index_capability_map(
    spec_map: Dict[str, str],
    executable_capabilities: Sequence[str],
    conditional_docs_map: Dict[str, str],
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    executable_set = set(executable_capabilities)
    unknown_caps = sorted(set(spec_map.values()) - executable_set)
    if unknown_caps:
        reasons.append("spec-index references capabilities not present in CAPABILITY-REGISTRY")
    conditional_mismatches: List[Dict[str, str]] = []
    missing_conditional_docs: List[str] = []
    for doc_ref, capability_id in conditional_docs_map.items():
        mapped = spec_map.get(doc_ref)
        if mapped is None:
            missing_conditional_docs.append(doc_ref)
            continue
        if mapped != capability_id:
            conditional_mismatches.append(
                {"docRef": doc_ref, "expected": capability_id, "actual": mapped}
            )
    if missing_conditional_docs or conditional_mismatches:
        reasons.append(
            "SPEC-INDEX §5.4 conditional capability docs diverge from COHERENCE-CONTRACT"
        )
    details = {
        "reasons": reasons,
        "specIndexCapabilityDocMap": spec_map,
        "conditionalCapabilityDocs": conditional_docs_map,
        "unknownCapabilities": unknown_caps,
        "missingConditionalDocs": sorted(missing_conditional_docs),
        "conditionalMismatches": conditional_mismatches,
    }
    return bool(reasons), details


def check_profile_overlay_claims(
    registry_overlay_claims: Sequence[str],
    conformance_overlay_claims: Sequence[str],
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    registry_set = set(registry_overlay_claims)
    conformance_set = set(conformance_overlay_claims)
    missing_in_conformance = sorted(registry_set - conformance_set)
    missing_in_registry = sorted(conformance_set - registry_set)
    if missing_in_conformance or missing_in_registry:
        reasons.append("CONFORMANCE §2.4 profile-overlay claims diverge from CAPABILITY-REGISTRY")
    details = {
        "reasons": reasons,
        "registryProfileOverlayClaims": sorted(registry_set),
        "conformanceProfileOverlayClaims": sorted(conformance_set),
        "missingInConformance": missing_in_conformance,
        "missingInRegistry": missing_in_registry,
    }
    return bool(reasons), details


def check_control_plane_lane_bindings(
    loaded_control_plane_contract: Dict[str, Any],
    control_plane_module: Any,
    gate_chain_details: Dict[str, Any],
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []

    lane_registry = gate_chain_details.get("laneRegistry")
    if not isinstance(lane_registry, dict):
        lane_registry = {}
        reasons.append("coherence witness missing gate_chain_parity laneRegistry details")

    contract_evidence_lanes = loaded_control_plane_contract.get("evidenceLanes", {})
    contract_lane_artifact_kinds = normalize_lane_artifact_kinds(
        loaded_control_plane_contract.get("laneArtifactKinds", {})
    )
    contract_checker_core = as_sorted_strings(
        loaded_control_plane_contract.get("laneOwnership", {}).get(
            "checkerCoreOnlyObligations", ()
        )
    )
    contract_required_route = loaded_control_plane_contract.get("laneOwnership", {}).get(
        "requiredCrossLaneWitnessRoute"
    )
    contract_lane_failure_classes = as_sorted_strings(
        loaded_control_plane_contract.get("laneFailureClasses", ())
    )
    contract_schema_lifecycle = loaded_control_plane_contract.get("schemaLifecycle", {})
    if not isinstance(contract_schema_lifecycle, dict):
        contract_schema_lifecycle = {}
    contract_schema_governance = contract_schema_lifecycle.get("governance", {})
    if not isinstance(contract_schema_governance, dict):
        contract_schema_governance = {}
    contract_governance_mode = str(contract_schema_governance.get("mode", ""))
    contract_governance_decision_ref = str(contract_schema_governance.get("decisionRef", ""))
    contract_governance_owner = str(contract_schema_governance.get("owner", ""))
    contract_rollover_cadence_months = contract_schema_governance.get(
        "rolloverCadenceMonths"
    )
    contract_freeze_reason = contract_schema_governance.get("freezeReason")
    contract_harness_retry = loaded_control_plane_contract.get("harnessRetry", {})
    if not isinstance(contract_harness_retry, dict):
        contract_harness_retry = {}
    contract_harness_policy_kind = str(contract_harness_retry.get("policyKind", ""))
    contract_harness_policy_path = str(contract_harness_retry.get("policyPath", ""))
    contract_harness_escalation_actions = as_sorted_strings(
        contract_harness_retry.get("escalationActions", ())
    )
    contract_harness_active_issue_env_keys = as_sorted_strings(
        contract_harness_retry.get("activeIssueEnvKeys", ())
    )
    contract_harness_issues_path_env_key = str(
        contract_harness_retry.get("issuesPathEnvKey", "")
    )
    contract_harness_session_path_env_key = str(
        contract_harness_retry.get("sessionPathEnvKey", "")
    )
    contract_harness_session_path_default = str(
        contract_harness_retry.get("sessionPathDefault", "")
    )
    contract_harness_session_issue_field = str(
        contract_harness_retry.get("sessionIssueField", "")
    )
    contract_stage1_parity = loaded_control_plane_contract.get("evidenceStage1Parity", {})
    if not isinstance(contract_stage1_parity, dict):
        contract_stage1_parity = {}
    contract_stage1_parity_profile_kind = str(contract_stage1_parity.get("profileKind", ""))
    contract_stage1_parity_classes_obj = contract_stage1_parity.get("failureClasses", {})
    if not isinstance(contract_stage1_parity_classes_obj, dict):
        contract_stage1_parity_classes_obj = {}
    contract_stage1_parity_failure_classes = as_sorted_strings(
        contract_stage1_parity_classes_obj.values()
    )

    contract_stage1_rollback = loaded_control_plane_contract.get("evidenceStage1Rollback", {})
    if not isinstance(contract_stage1_rollback, dict):
        contract_stage1_rollback = {}
    contract_stage1_rollback_profile_kind = str(contract_stage1_rollback.get("profileKind", ""))
    contract_stage1_rollback_witness_kind = str(contract_stage1_rollback.get("witnessKind", ""))
    contract_stage1_rollback_trigger_failure_classes = as_sorted_strings(
        contract_stage1_rollback.get("triggerFailureClasses", ())
    )
    contract_stage1_rollback_classes_obj = contract_stage1_rollback.get("failureClasses", {})
    if not isinstance(contract_stage1_rollback_classes_obj, dict):
        contract_stage1_rollback_classes_obj = {}
    contract_stage1_rollback_failure_classes = as_sorted_strings(
        contract_stage1_rollback_classes_obj.values()
    )
    contract_stage2_authority = loaded_control_plane_contract.get("evidenceStage2Authority", {})
    if not isinstance(contract_stage2_authority, dict):
        contract_stage2_authority = {}
    contract_stage2_profile_kind = str(contract_stage2_authority.get("profileKind", ""))
    contract_stage2_active_stage = str(contract_stage2_authority.get("activeStage", ""))
    contract_stage2_alias = contract_stage2_authority.get("compatibilityAlias", {})
    if not isinstance(contract_stage2_alias, dict):
        contract_stage2_alias = {}
    contract_stage2_alias_role = str(contract_stage2_alias.get("role", ""))
    contract_stage2_alias_support_until_epoch = str(
        contract_stage2_alias.get("supportUntilEpoch", "")
    )
    contract_stage2_classes_obj = contract_stage2_authority.get("failureClasses", {})
    if not isinstance(contract_stage2_classes_obj, dict):
        contract_stage2_classes_obj = {}
    contract_stage2_failure_classes = as_sorted_strings(contract_stage2_classes_obj.values())
    contract_stage2_bidir_route = contract_stage2_authority.get("bidirEvidenceRoute", {})
    if not isinstance(contract_stage2_bidir_route, dict):
        contract_stage2_bidir_route = {}
    contract_stage2_bidir_required_obligations = as_sorted_strings(
        contract_stage2_bidir_route.get("requiredObligations", ())
    )
    contract_stage2_bidir_classes_obj = contract_stage2_bidir_route.get("failureClasses", {})
    if not isinstance(contract_stage2_bidir_classes_obj, dict):
        contract_stage2_bidir_classes_obj = {}
    contract_stage2_bidir_failure_classes = as_sorted_strings(
        contract_stage2_bidir_classes_obj.values()
    )

    checker_expected_core = as_sorted_strings(
        lane_registry.get("expectedCheckerCoreOnlyObligations", ())
    )
    checker_required_route = lane_registry.get("requiredCrossLaneWitnessRoute")
    checker_required_failures = as_sorted_strings(
        lane_registry.get("requiredLaneFailureClasses", ())
    )
    checker_stage1_parity = gate_chain_details.get("stage1Parity")
    if not isinstance(checker_stage1_parity, dict):
        checker_stage1_parity = {}
        reasons.append("coherence witness missing gate_chain_parity stage1Parity details")
    checker_stage1_parity_required_classes_obj = checker_stage1_parity.get(
        "requiredFailureClasses", {}
    )
    if not isinstance(checker_stage1_parity_required_classes_obj, dict):
        checker_stage1_parity_required_classes_obj = {}
    checker_stage1_parity_required_classes = as_sorted_strings(
        checker_stage1_parity_required_classes_obj.values()
    )

    checker_stage1_rollback = gate_chain_details.get("stage1Rollback")
    if not isinstance(checker_stage1_rollback, dict):
        checker_stage1_rollback = {}
        reasons.append("coherence witness missing gate_chain_parity stage1Rollback details")
    checker_stage1_rollback_required_trigger_classes = as_sorted_strings(
        checker_stage1_rollback.get("requiredTriggerFailureClasses", ())
    )
    checker_stage1_rollback_required_classes_obj = checker_stage1_rollback.get(
        "requiredFailureClasses", {}
    )
    if not isinstance(checker_stage1_rollback_required_classes_obj, dict):
        checker_stage1_rollback_required_classes_obj = {}
    checker_stage1_rollback_required_classes = as_sorted_strings(
        checker_stage1_rollback_required_classes_obj.values()
    )
    checker_stage2_authority = gate_chain_details.get("stage2Authority")
    if not isinstance(checker_stage2_authority, dict):
        checker_stage2_authority = {}
        if contract_stage2_authority:
            reasons.append("coherence witness missing gate_chain_parity stage2Authority details")
    checker_stage2_required_classes_obj = checker_stage2_authority.get(
        "requiredFailureClasses", {}
    )
    if not isinstance(checker_stage2_required_classes_obj, dict):
        checker_stage2_required_classes_obj = {}
    checker_stage2_required_classes = as_sorted_strings(
        checker_stage2_required_classes_obj.values()
    )
    checker_stage2_bidir_route = checker_stage2_authority.get("bidirEvidenceRoute", {})
    if not isinstance(checker_stage2_bidir_route, dict):
        checker_stage2_bidir_route = {}
    checker_stage2_bidir_required_obligations = as_sorted_strings(
        checker_stage2_bidir_route.get("requiredObligations", ())
    )
    if not checker_stage2_bidir_required_obligations:
        checker_stage2_kernel_sentinel = checker_stage2_authority.get(
            "kernelComplianceSentinel", {}
        )
        if isinstance(checker_stage2_kernel_sentinel, dict):
            checker_stage2_bidir_required_obligations = as_sorted_strings(
                checker_stage2_kernel_sentinel.get("requiredObligations", ())
            )
    checker_stage2_bidir_required_classes_obj = checker_stage2_authority.get(
        "requiredBidirEvidenceFailureClasses", {}
    )
    if not isinstance(checker_stage2_bidir_required_classes_obj, dict):
        checker_stage2_bidir_required_classes_obj = {}
    if not checker_stage2_bidir_required_classes_obj:
        kernel_classes_fallback = checker_stage2_authority.get(
            "requiredKernelComplianceFailureClasses", {}
        )
        if isinstance(kernel_classes_fallback, dict):
            checker_stage2_bidir_required_classes_obj = kernel_classes_fallback
    checker_stage2_bidir_required_classes = as_sorted_strings(
        checker_stage2_bidir_required_classes_obj.values()
    )
    checker_lane_values = lane_registry.get("evidenceLanes")
    if isinstance(checker_lane_values, dict) and checker_lane_values != contract_evidence_lanes:
        reasons.append("coherence checker lane IDs differ from CONTROL-PLANE-CONTRACT evidenceLanes")
    checker_kinds = normalize_lane_artifact_kinds(lane_registry.get("laneArtifactKinds", {}))
    if checker_kinds and checker_kinds != contract_lane_artifact_kinds:
        reasons.append("coherence checker laneArtifactKinds differ from CONTROL-PLANE-CONTRACT")
    if checker_expected_core and checker_expected_core != contract_checker_core:
        reasons.append(
            "checker expected checker-core-only obligations differ from CONTROL-PLANE-CONTRACT laneOwnership"
        )
    if checker_required_route and checker_required_route != contract_required_route:
        reasons.append(
            "checker required cross-lane witness route differs from CONTROL-PLANE-CONTRACT laneOwnership"
        )
    if checker_required_failures and not set(checker_required_failures).issubset(
        set(contract_lane_failure_classes)
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT laneFailureClasses missing checker-required failure classes"
        )
    if checker_stage1_parity_required_classes and (
        checker_stage1_parity_required_classes != contract_stage1_parity_failure_classes
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage1Parity.failureClasses differ from checker-required classes"
        )
    if checker_stage1_rollback_required_classes and (
        checker_stage1_rollback_required_classes != contract_stage1_rollback_failure_classes
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.failureClasses differ from checker-required classes"
        )
    if checker_stage1_rollback_required_trigger_classes and not set(
        checker_stage1_rollback_required_trigger_classes
    ).issubset(set(contract_stage1_rollback_trigger_failure_classes)):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage1Rollback.triggerFailureClasses missing checker-required trigger classes"
        )
    if checker_stage2_required_classes and (
        checker_stage2_required_classes != contract_stage2_failure_classes
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.failureClasses differ from checker-required classes"
        )
    if checker_stage2_bidir_required_obligations and (
        checker_stage2_bidir_required_obligations
        != contract_stage2_bidir_required_obligations
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.bidirEvidenceRoute.requiredObligations differ from checker-observed values"
        )
    if checker_stage2_bidir_required_classes and (
        checker_stage2_bidir_required_classes
        != contract_stage2_bidir_failure_classes
    ):
        reasons.append(
            "CONTROL-PLANE-CONTRACT evidenceStage2Authority.bidirEvidenceRoute.failureClasses differ from checker-required classes"
        )

    loader_evidence_lanes = dict(getattr(control_plane_module, "EVIDENCE_LANES", {}))
    loader_lane_artifact_kinds = normalize_lane_artifact_kinds(
        getattr(control_plane_module, "LANE_ARTIFACT_KINDS", {})
    )
    loader_checker_core = as_sorted_strings(
        getattr(control_plane_module, "CHECKER_CORE_ONLY_OBLIGATIONS", ())
    )
    loader_required_route = getattr(
        control_plane_module, "REQUIRED_CROSS_LANE_WITNESS_ROUTE", None
    )
    loader_lane_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "LANE_FAILURE_CLASSES", ())
    )
    loader_governance_mode = str(
        getattr(control_plane_module, "SCHEMA_LIFECYCLE_GOVERNANCE_MODE", "")
    )
    loader_governance_decision_ref = str(
        getattr(control_plane_module, "SCHEMA_LIFECYCLE_GOVERNANCE_DECISION_REF", "")
    )
    loader_governance_owner = str(
        getattr(control_plane_module, "SCHEMA_LIFECYCLE_GOVERNANCE_OWNER", "")
    )
    loader_rollover_cadence_months = getattr(
        control_plane_module, "SCHEMA_LIFECYCLE_ROLLOVER_CADENCE_MONTHS", None
    )
    loader_freeze_reason = getattr(
        control_plane_module, "SCHEMA_LIFECYCLE_FREEZE_REASON", None
    )
    loader_harness_policy_kind = str(
        getattr(control_plane_module, "HARNESS_RETRY_POLICY_KIND", "")
    )
    loader_harness_policy_path = str(
        getattr(control_plane_module, "HARNESS_RETRY_POLICY_PATH", "")
    )
    loader_harness_escalation_actions = as_sorted_strings(
        getattr(control_plane_module, "HARNESS_ESCALATION_ACTIONS", ())
    )
    loader_harness_active_issue_env_keys = as_sorted_strings(
        getattr(control_plane_module, "HARNESS_ACTIVE_ISSUE_ENV_KEYS", ())
    )
    loader_harness_issues_path_env_key = str(
        getattr(control_plane_module, "HARNESS_ISSUES_PATH_ENV_KEY", "")
    )
    loader_harness_session_path_env_key = str(
        getattr(control_plane_module, "HARNESS_SESSION_PATH_ENV_KEY", "")
    )
    loader_harness_session_path_default = str(
        getattr(control_plane_module, "HARNESS_SESSION_PATH_DEFAULT", "")
    )
    loader_harness_session_issue_field = str(
        getattr(control_plane_module, "HARNESS_SESSION_ISSUE_FIELD", "")
    )
    loader_stage1_parity_profile_kind = str(
        getattr(control_plane_module, "EVIDENCE_STAGE1_PARITY_PROFILE_KIND", "")
    )
    loader_stage1_parity_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "EVIDENCE_STAGE1_PARITY_FAILURE_CLASSES", ())
    )
    loader_stage1_rollback_profile_kind = str(
        getattr(control_plane_module, "EVIDENCE_STAGE1_ROLLBACK_PROFILE_KIND", "")
    )
    loader_stage1_rollback_witness_kind = str(
        getattr(control_plane_module, "EVIDENCE_STAGE1_ROLLBACK_WITNESS_KIND", "")
    )
    loader_stage1_rollback_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "EVIDENCE_STAGE1_ROLLBACK_FAILURE_CLASSES", ())
    )
    loader_stage2_profile_kind = str(
        getattr(control_plane_module, "EVIDENCE_STAGE2_AUTHORITY_PROFILE_KIND", "")
    )
    loader_stage2_active_stage = str(
        getattr(control_plane_module, "EVIDENCE_STAGE2_AUTHORITY_ACTIVE_STAGE", "")
    )
    loader_stage2_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "EVIDENCE_STAGE2_AUTHORITY_FAILURE_CLASSES", ())
    )
    loader_stage2_alias_role = str(
        getattr(control_plane_module, "EVIDENCE_STAGE2_ALIAS_ROLE", "")
    )
    loader_stage2_alias_support_until_epoch = str(
        getattr(control_plane_module, "EVIDENCE_STAGE2_ALIAS_SUPPORT_UNTIL_EPOCH", "")
    )
    loader_stage2_bidir_required_obligations = as_sorted_strings(
        getattr(control_plane_module, "EVIDENCE_STAGE2_BIDIR_REQUIRED_OBLIGATIONS", ())
    )
    loader_stage2_bidir_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "EVIDENCE_STAGE2_BIDIR_FAILURE_CLASSES", ())
    )

    if loader_evidence_lanes != contract_evidence_lanes:
        reasons.append("control_plane_contract.py EVIDENCE_LANES drift from contract payload")
    if loader_lane_artifact_kinds != contract_lane_artifact_kinds:
        reasons.append("control_plane_contract.py LANE_ARTIFACT_KINDS drift from contract payload")
    if loader_checker_core != contract_checker_core:
        reasons.append(
            "control_plane_contract.py CHECKER_CORE_ONLY_OBLIGATIONS drift from contract payload"
        )
    if loader_required_route != contract_required_route:
        reasons.append(
            "control_plane_contract.py REQUIRED_CROSS_LANE_WITNESS_ROUTE drift from contract payload"
        )
    if loader_lane_failure_classes != contract_lane_failure_classes:
        reasons.append(
            "control_plane_contract.py LANE_FAILURE_CLASSES drift from contract payload"
        )
    if loader_governance_mode != contract_governance_mode:
        reasons.append(
            "control_plane_contract.py SCHEMA_LIFECYCLE_GOVERNANCE_MODE drift from contract payload"
        )
    if loader_governance_decision_ref != contract_governance_decision_ref:
        reasons.append(
            "control_plane_contract.py SCHEMA_LIFECYCLE_GOVERNANCE_DECISION_REF drift from contract payload"
        )
    if loader_governance_owner != contract_governance_owner:
        reasons.append(
            "control_plane_contract.py SCHEMA_LIFECYCLE_GOVERNANCE_OWNER drift from contract payload"
        )
    if loader_rollover_cadence_months != contract_rollover_cadence_months:
        reasons.append(
            "control_plane_contract.py SCHEMA_LIFECYCLE_ROLLOVER_CADENCE_MONTHS drift from contract payload"
        )
    if loader_freeze_reason != contract_freeze_reason:
        reasons.append(
            "control_plane_contract.py SCHEMA_LIFECYCLE_FREEZE_REASON drift from contract payload"
        )
    if loader_harness_policy_kind != contract_harness_policy_kind:
        reasons.append(
            "control_plane_contract.py HARNESS_RETRY_POLICY_KIND drift from contract payload"
        )
    if loader_harness_policy_path != contract_harness_policy_path:
        reasons.append(
            "control_plane_contract.py HARNESS_RETRY_POLICY_PATH drift from contract payload"
        )
    if loader_harness_escalation_actions != contract_harness_escalation_actions:
        reasons.append(
            "control_plane_contract.py HARNESS_ESCALATION_ACTIONS drift from contract payload"
        )
    if (
        loader_harness_active_issue_env_keys
        != contract_harness_active_issue_env_keys
    ):
        reasons.append(
            "control_plane_contract.py HARNESS_ACTIVE_ISSUE_ENV_KEYS drift from contract payload"
        )
    if loader_harness_issues_path_env_key != contract_harness_issues_path_env_key:
        reasons.append(
            "control_plane_contract.py HARNESS_ISSUES_PATH_ENV_KEY drift from contract payload"
        )
    if loader_harness_session_path_env_key != contract_harness_session_path_env_key:
        reasons.append(
            "control_plane_contract.py HARNESS_SESSION_PATH_ENV_KEY drift from contract payload"
        )
    if loader_harness_session_path_default != contract_harness_session_path_default:
        reasons.append(
            "control_plane_contract.py HARNESS_SESSION_PATH_DEFAULT drift from contract payload"
        )
    if loader_harness_session_issue_field != contract_harness_session_issue_field:
        reasons.append(
            "control_plane_contract.py HARNESS_SESSION_ISSUE_FIELD drift from contract payload"
        )
    if loader_stage1_parity_profile_kind != contract_stage1_parity_profile_kind:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE1_PARITY_PROFILE_KIND drift from contract payload"
        )
    if loader_stage1_parity_failure_classes != contract_stage1_parity_failure_classes:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE1_PARITY_FAILURE_CLASSES drift from contract payload"
        )
    if loader_stage1_rollback_profile_kind != contract_stage1_rollback_profile_kind:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE1_ROLLBACK_PROFILE_KIND drift from contract payload"
        )
    if loader_stage1_rollback_witness_kind != contract_stage1_rollback_witness_kind:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE1_ROLLBACK_WITNESS_KIND drift from contract payload"
        )
    if loader_stage1_rollback_failure_classes != contract_stage1_rollback_failure_classes:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE1_ROLLBACK_FAILURE_CLASSES drift from contract payload"
        )
    if contract_stage2_profile_kind and loader_stage2_profile_kind != contract_stage2_profile_kind:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_AUTHORITY_PROFILE_KIND drift from contract payload"
        )
    if contract_stage2_active_stage and loader_stage2_active_stage != contract_stage2_active_stage:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_AUTHORITY_ACTIVE_STAGE drift from contract payload"
        )
    if contract_stage2_failure_classes and loader_stage2_failure_classes != contract_stage2_failure_classes:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_AUTHORITY_FAILURE_CLASSES drift from contract payload"
        )
    if contract_stage2_alias_role and loader_stage2_alias_role != contract_stage2_alias_role:
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_ALIAS_ROLE drift from contract payload"
        )
    if (
        contract_stage2_alias_support_until_epoch
        and loader_stage2_alias_support_until_epoch != contract_stage2_alias_support_until_epoch
    ):
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_ALIAS_SUPPORT_UNTIL_EPOCH drift from contract payload"
        )
    if (
        contract_stage2_bidir_required_obligations
        and loader_stage2_bidir_required_obligations != contract_stage2_bidir_required_obligations
    ):
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_BIDIR_REQUIRED_OBLIGATIONS drift from contract payload"
        )
    if (
        contract_stage2_bidir_failure_classes
        and loader_stage2_bidir_failure_classes != contract_stage2_bidir_failure_classes
    ):
        reasons.append(
            "control_plane_contract.py EVIDENCE_STAGE2_BIDIR_FAILURE_CLASSES drift from contract payload"
        )

    details = {
        "reasons": reasons,
        "contract": {
            "evidenceLanes": contract_evidence_lanes,
            "laneArtifactKinds": contract_lane_artifact_kinds,
            "checkerCoreOnlyObligations": contract_checker_core,
            "requiredCrossLaneWitnessRoute": contract_required_route,
            "laneFailureClasses": contract_lane_failure_classes,
            "schemaLifecycleGovernance": {
                "mode": contract_governance_mode,
                "decisionRef": contract_governance_decision_ref,
                "owner": contract_governance_owner,
                "rolloverCadenceMonths": contract_rollover_cadence_months,
                "freezeReason": contract_freeze_reason,
            },
            "harnessRetry": {
                "policyKind": contract_harness_policy_kind,
                "policyPath": contract_harness_policy_path,
                "escalationActions": contract_harness_escalation_actions,
                "activeIssueEnvKeys": contract_harness_active_issue_env_keys,
                "issuesPathEnvKey": contract_harness_issues_path_env_key,
                "sessionPathEnvKey": contract_harness_session_path_env_key,
                "sessionPathDefault": contract_harness_session_path_default,
                "sessionIssueField": contract_harness_session_issue_field,
            },
            "stage1": {
                "parityProfileKind": contract_stage1_parity_profile_kind,
                "parityFailureClasses": contract_stage1_parity_failure_classes,
                "rollbackProfileKind": contract_stage1_rollback_profile_kind,
                "rollbackWitnessKind": contract_stage1_rollback_witness_kind,
                "rollbackTriggerFailureClasses": contract_stage1_rollback_trigger_failure_classes,
                "rollbackFailureClasses": contract_stage1_rollback_failure_classes,
            },
            "stage2": {
                "authorityProfileKind": contract_stage2_profile_kind,
                "activeStage": contract_stage2_active_stage,
                "aliasRole": contract_stage2_alias_role,
                "aliasSupportUntilEpoch": contract_stage2_alias_support_until_epoch,
                "authorityFailureClasses": contract_stage2_failure_classes,
                "bidirRequiredObligations": contract_stage2_bidir_required_obligations,
                "bidirFailureClasses": contract_stage2_bidir_failure_classes,
            },
        },
        "checker": {
            "evidenceLanes": checker_lane_values,
            "laneArtifactKinds": checker_kinds,
            "expectedCheckerCoreOnlyObligations": checker_expected_core,
            "requiredCrossLaneWitnessRoute": checker_required_route,
            "requiredLaneFailureClasses": checker_required_failures,
            "stage1": {
                "parityRequiredFailureClasses": checker_stage1_parity_required_classes,
                "rollbackRequiredTriggerFailureClasses": checker_stage1_rollback_required_trigger_classes,
                "rollbackRequiredFailureClasses": checker_stage1_rollback_required_classes,
            },
            "stage2": {
                "authorityRequiredFailureClasses": checker_stage2_required_classes,
                "bidirRequiredObligations": checker_stage2_bidir_required_obligations,
                "bidirRequiredFailureClasses": checker_stage2_bidir_required_classes,
            },
        },
        "loader": {
            "evidenceLanes": loader_evidence_lanes,
            "laneArtifactKinds": loader_lane_artifact_kinds,
            "checkerCoreOnlyObligations": loader_checker_core,
            "requiredCrossLaneWitnessRoute": loader_required_route,
            "laneFailureClasses": loader_lane_failure_classes,
            "schemaLifecycleGovernance": {
                "mode": loader_governance_mode,
                "decisionRef": loader_governance_decision_ref,
                "owner": loader_governance_owner,
                "rolloverCadenceMonths": loader_rollover_cadence_months,
                "freezeReason": loader_freeze_reason,
            },
            "harnessRetry": {
                "policyKind": loader_harness_policy_kind,
                "policyPath": loader_harness_policy_path,
                "escalationActions": loader_harness_escalation_actions,
                "activeIssueEnvKeys": loader_harness_active_issue_env_keys,
                "issuesPathEnvKey": loader_harness_issues_path_env_key,
                "sessionPathEnvKey": loader_harness_session_path_env_key,
                "sessionPathDefault": loader_harness_session_path_default,
                "sessionIssueField": loader_harness_session_issue_field,
            },
            "stage1": {
                "parityProfileKind": loader_stage1_parity_profile_kind,
                "parityFailureClasses": loader_stage1_parity_failure_classes,
                "rollbackProfileKind": loader_stage1_rollback_profile_kind,
                "rollbackWitnessKind": loader_stage1_rollback_witness_kind,
                "rollbackFailureClasses": loader_stage1_rollback_failure_classes,
            },
            "stage2": {
                "authorityProfileKind": loader_stage2_profile_kind,
                "activeStage": loader_stage2_active_stage,
                "aliasRole": loader_stage2_alias_role,
                "aliasSupportUntilEpoch": loader_stage2_alias_support_until_epoch,
                "authorityFailureClasses": loader_stage2_failure_classes,
                "bidirRequiredObligations": loader_stage2_bidir_required_obligations,
                "bidirFailureClasses": loader_stage2_bidir_failure_classes,
            },
        },
    }
    return bool(reasons), details


def check_runtime_route_bindings(
    loaded_control_plane_contract: Dict[str, Any],
    control_plane_module: Any,
    doctrine_operations: Dict[str, Dict[str, Any]],
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []

    contract_runtime_routes = normalize_runtime_route_bindings(
        loaded_control_plane_contract.get("runtimeRouteBindings", {}).get(
            "requiredOperationRoutes", {}
        )
    )
    if not contract_runtime_routes:
        reasons.append(
            "CONTROL-PLANE-CONTRACT missing runtimeRouteBindings.requiredOperationRoutes"
        )

    contract_runtime_failure_classes = as_sorted_strings(
        loaded_control_plane_contract.get("runtimeRouteBindings", {})
        .get("failureClasses", {})
        .values()
    )
    if not contract_runtime_failure_classes:
        reasons.append("CONTROL-PLANE-CONTRACT missing runtimeRouteBindings.failureClasses")

    loader_runtime_routes = normalize_runtime_route_bindings(
        getattr(control_plane_module, "RUNTIME_ROUTE_BINDINGS", {})
    )
    loader_runtime_failure_classes = as_sorted_strings(
        getattr(control_plane_module, "RUNTIME_ROUTE_FAILURE_CLASSES", ())
    )
    if loader_runtime_routes != contract_runtime_routes:
        reasons.append(
            "control_plane_contract.py RUNTIME_ROUTE_BINDINGS drift from contract payload"
        )
    if loader_runtime_failure_classes != contract_runtime_failure_classes:
        reasons.append(
            "control_plane_contract.py RUNTIME_ROUTE_FAILURE_CLASSES drift from contract payload"
        )

    missing_operation_routes: List[Dict[str, str]] = []
    missing_morphisms: List[Dict[str, Any]] = []
    observed_registry_routes: Dict[str, Dict[str, Any]] = {}
    for route_id, route in contract_runtime_routes.items():
        operation_id = route["operationId"]
        required_morphisms = as_sorted_strings(route.get("requiredMorphisms", ()))
        operation_row = doctrine_operations.get(operation_id)
        if operation_row is None:
            missing_operation_routes.append(
                {
                    "routeId": route_id,
                    "operationId": operation_id,
                }
            )
            continue
        actual_morphisms = as_sorted_strings(operation_row.get("morphisms", ()))
        observed_registry_routes[route_id] = {
            "operationId": operation_id,
            "path": operation_row.get("path", ""),
            "actualMorphisms": actual_morphisms,
        }
        route_missing_morphisms = sorted(set(required_morphisms) - set(actual_morphisms))
        if route_missing_morphisms:
            missing_morphisms.append(
                {
                    "routeId": route_id,
                    "operationId": operation_id,
                    "missingMorphisms": route_missing_morphisms,
                    "requiredMorphisms": required_morphisms,
                    "actualMorphisms": actual_morphisms,
                }
            )

    if missing_operation_routes or missing_morphisms:
        reasons.append(
            "DOCTRINE-OP-REGISTRY missing required runtime-route bindings or morphisms"
        )

    details = {
        "reasons": reasons,
        "contractRuntimeRouteBindings": contract_runtime_routes,
        "contractRuntimeRouteFailureClasses": contract_runtime_failure_classes,
        "loaderRuntimeRouteBindings": loader_runtime_routes,
        "loaderRuntimeRouteFailureClasses": loader_runtime_failure_classes,
        "observedDoctrineRegistryRoutes": observed_registry_routes,
        "missingOperationRoutes": missing_operation_routes,
        "missingRequiredMorphisms": missing_morphisms,
    }
    return bool(reasons), details


def check_control_plane_kcir_mappings(
    loaded_control_plane_contract: Dict[str, Any],
    control_plane_module: Any,
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    contract_mappings = loaded_control_plane_contract.get("controlPlaneKcirMappings", {})
    if not isinstance(contract_mappings, dict):
        contract_mappings = {}
        reasons.append("CONTROL-PLANE-CONTRACT missing controlPlaneKcirMappings")

    contract_profile_id = str(contract_mappings.get("profileId", "")).strip()
    contract_mapping_table = normalize_kcir_mapping_table(
        contract_mappings.get("mappingTable", {})
    )
    contract_legacy_policy = normalize_kcir_legacy_policy(
        contract_mappings.get("compatibilityPolicy", {}).get(
            "legacyNonKcirEncodings", {}
        )
        if isinstance(contract_mappings.get("compatibilityPolicy", {}), dict)
        else {}
    )

    loader_profile_id = str(
        getattr(control_plane_module, "CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID", "")
    ).strip()
    loader_mapping_table = normalize_kcir_mapping_table(
        getattr(control_plane_module, "CONTROL_PLANE_KCIR_MAPPING_TABLE", {})
    )
    loader_legacy_policy = normalize_kcir_legacy_policy(
        getattr(control_plane_module, "CONTROL_PLANE_KCIR_LEGACY_POLICY", {})
    )

    if loader_profile_id != contract_profile_id:
        reasons.append(
            "control_plane_contract.py CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID drift from contract payload"
        )

    missing_rows_in_loader = sorted(set(contract_mapping_table) - set(loader_mapping_table))
    missing_rows_in_contract = sorted(
        set(loader_mapping_table) - set(contract_mapping_table)
    )
    row_drifts: List[Dict[str, Any]] = []
    for row_id in sorted(set(contract_mapping_table) & set(loader_mapping_table)):
        contract_row = contract_mapping_table[row_id]
        loader_row = loader_mapping_table[row_id]
        drift_fields: List[str] = []
        for field in ("sourceKind", "targetDomain", "targetKind", "identityFields"):
            if contract_row.get(field) != loader_row.get(field):
                drift_fields.append(field)
        if drift_fields:
            row_drifts.append(
                {
                    "rowId": row_id,
                    "driftFields": drift_fields,
                    "contract": contract_row,
                    "loader": loader_row,
                }
            )

    if missing_rows_in_loader or missing_rows_in_contract or row_drifts:
        reasons.append(
            "control_plane_contract.py CONTROL_PLANE_KCIR_MAPPING_TABLE drift from contract payload"
        )

    legacy_policy_drift_fields = [
        field
        for field in ("mode", "authorityMode", "supportUntilEpoch", "failureClass")
        if contract_legacy_policy.get(field, "") != loader_legacy_policy.get(field, "")
    ]
    if legacy_policy_drift_fields:
        reasons.append(
            "control_plane_contract.py CONTROL_PLANE_KCIR_LEGACY_POLICY drift from contract payload"
        )

    details = {
        "reasons": reasons,
        "contractProfileId": contract_profile_id,
        "loaderProfileId": loader_profile_id,
        "contractMappingTable": contract_mapping_table,
        "loaderMappingTable": loader_mapping_table,
        "missingRowsInLoader": missing_rows_in_loader,
        "missingRowsInContract": missing_rows_in_contract,
        "rowDrifts": row_drifts,
        "contractLegacyPolicy": contract_legacy_policy,
        "loaderLegacyPolicy": loader_legacy_policy,
        "legacyPolicyDriftFields": legacy_policy_drift_fields,
    }
    return bool(reasons), details


def check_coherence_required_obligations(
    coherence_contract: Dict[str, Any],
    scope_noncontradiction_details: Dict[str, Any],
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    contract_required_obligations = as_sorted_strings(
        parse_required_obligation_ids(coherence_contract)
    )
    contract_required_bidir = as_sorted_strings(
        parse_required_bidir_obligations(coherence_contract)
    )
    contract_registry_kind = (
        coherence_contract.get("surfaces", {}).get("obligationRegistryKind")
        if isinstance(coherence_contract.get("surfaces"), dict)
        else None
    )

    checker_required_obligations = as_sorted_strings(
        scope_noncontradiction_details.get("requiredCoherenceObligations", ())
    )
    checker_required_bidir = as_sorted_strings(
        scope_noncontradiction_details.get("requiredBidirObligations", ())
    )
    checker_registry_kind = scope_noncontradiction_details.get("obligationRegistryKind")

    if contract_required_obligations != checker_required_obligations:
        reasons.append("coherence required obligation set drifts between contract and checker")
    if contract_required_bidir != checker_required_bidir:
        reasons.append("requiredBidirObligations drifts between contract and checker")
    if contract_registry_kind != checker_registry_kind:
        reasons.append("obligation registry kind drifts between contract and checker")

    details = {
        "reasons": reasons,
        "contractRequiredObligations": contract_required_obligations,
        "checkerRequiredObligations": checker_required_obligations,
        "contractRequiredBidirObligations": contract_required_bidir,
        "checkerRequiredBidirObligations": checker_required_bidir,
        "contractObligationRegistryKind": contract_registry_kind,
        "checkerObligationRegistryKind": checker_registry_kind,
    }
    return bool(reasons), details


def check_sigpi_notation(repo_root: Path) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    alias_hits: List[str] = []
    canonical_sigpi_docs: List[str] = []
    canonical_latex_docs: List[str] = []

    for rel in SIGPI_NORMATIVE_DOCS:
        path = repo_root / rel
        text = path.read_text(encoding="utf-8")
        if _SIGPI_ALIAS_RE.search(text):
            alias_hits.append(rel)
        if "SigPi" in text:
            canonical_sigpi_docs.append(rel)
        if "sig\\Pi" in text:
            canonical_latex_docs.append(rel)

    if alias_hits:
        reasons.append("normative docs still use Sig/Pi alias")
    if not canonical_sigpi_docs:
        reasons.append("normative docs missing canonical SigPi spelling")
    if not canonical_latex_docs:
        reasons.append("normative docs missing canonical sig\\\\Pi notation")

    details = {
        "reasons": reasons,
        "checkedDocs": list(SIGPI_NORMATIVE_DOCS),
        "aliasHits": sorted(alias_hits),
        "canonicalSigPiDocs": sorted(canonical_sigpi_docs),
        "canonicalLatexDocs": sorted(canonical_latex_docs),
    }
    return bool(reasons), details


def check_cache_input_closure(
    repo_root: Path,
    fixture_suites_module: Any,
) -> Tuple[bool, Dict[str, Any]]:
    reasons: List[str] = []
    closure_paths = {
        path.resolve()
        for path in fixture_suites_module.load_coherence_contract_input_paths()
    }
    required_paths = [(repo_root / rel).resolve() for rel in CACHE_CLOSURE_REQUIRED_PATHS]
    missing = [
        rel
        for rel, abs_path in zip(CACHE_CLOSURE_REQUIRED_PATHS, required_paths)
        if abs_path not in closure_paths
    ]
    if missing:
        reasons.append("coherence-contract cache input closure missing required loader inputs")

    details = {
        "reasons": reasons,
        "requiredPaths": list(CACHE_CLOSURE_REQUIRED_PATHS),
        "missingPaths": sorted(missing),
        "closureSize": len(closure_paths),
    }
    return bool(reasons), details


def _extract_frontmatter_status(path: Path) -> str | None:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return None
    try:
        _, rest = text.split("---\n", 1)
        frontmatter, _ = rest.split("---\n", 1)
    except ValueError:
        return None
    for raw in frontmatter.splitlines():
        line = raw.strip()
        if line.startswith("status:"):
            _, value = line.split(":", 1)
            return value.strip()
    return None


def count_promoted_draft_specs(draft_dir: Path) -> int:
    count = 0
    for path in sorted(draft_dir.iterdir()):
        if path.is_dir():
            continue
        if path.name == "README.md":
            continue
        if path.suffix == ".md" and _extract_frontmatter_status(path) == "draft":
            count += 1
        elif path.suffix == ".json":
            count += 1
    return count


def count_traceability_rows(matrix_path: Path) -> int:
    lines = matrix_path.read_text(encoding="utf-8").splitlines()
    in_matrix = False
    count = 0
    for line in lines:
        if line.startswith("## 3. Traceability Matrix"):
            in_matrix = True
            continue
        if in_matrix and line.startswith("## "):
            break
        if not in_matrix:
            continue
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        if stripped.startswith("| Draft spec"):
            continue
        if re.match(r"^\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|$", stripped):
            continue
        parts = [cell.strip() for cell in stripped.strip("|").split("|")]
        if len(parts) != 4:
            continue
        if not CODE_REF_RE.search(parts[0]):
            continue
        count += 1
    return count


def parse_optional_string_list(value: Any, label: str) -> List[str]:
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, row in enumerate(value):
        if not isinstance(row, str) or not row.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(row.strip())
    if len(set(out)) != len(out):
        raise ValueError(f"{label} must not contain duplicates")
    return out


def parse_topology_threshold(raw: Any, label: str) -> Dict[str, int | None]:
    if not isinstance(raw, dict):
        raise ValueError(f"{label} must be an object")
    out: Dict[str, int | None] = {
        "warnAbove": None,
        "failAbove": None,
        "warnBelow": None,
        "failBelow": None,
    }
    for key in list(out.keys()):
        value = raw.get(key)
        if value is None:
            continue
        if not isinstance(value, int):
            raise ValueError(f"{label}.{key} must be an integer")
        out[key] = value
    if out["warnAbove"] is not None and out["failAbove"] is not None:
        if out["warnAbove"] > out["failAbove"]:
            raise ValueError(f"{label}: warnAbove must be <= failAbove")
    if out["warnBelow"] is not None and out["failBelow"] is not None:
        if out["warnBelow"] < out["failBelow"]:
            raise ValueError(f"{label}: warnBelow must be >= failBelow")
    if all(value is None for value in out.values()):
        raise ValueError(f"{label} must declare at least one threshold bound")
    return out


def load_topology_budget_contract(path: Path) -> Dict[str, Any]:
    payload = load_json(path)
    if payload.get("schema") != TOPOLOGY_BUDGET_SCHEMA:
        raise ValueError(f"{path}: schema must be {TOPOLOGY_BUDGET_SCHEMA}")
    if payload.get("budgetKind") != TOPOLOGY_BUDGET_KIND:
        raise ValueError(f"{path}: budgetKind must be {TOPOLOGY_BUDGET_KIND!r}")

    metrics_raw = payload.get("metrics")
    if not isinstance(metrics_raw, dict) or not metrics_raw:
        raise ValueError(f"{path}: metrics must be a non-empty object")

    metrics: Dict[str, Dict[str, int | None]] = {}
    for metric_id, threshold_raw in metrics_raw.items():
        if not isinstance(metric_id, str) or not metric_id.strip():
            raise ValueError(f"{path}: metric IDs must be non-empty strings")
        key = metric_id.strip()
        if key in metrics:
            raise ValueError(f"{path}: duplicate metric ID {key!r}")
        metrics[key] = parse_topology_threshold(threshold_raw, f"{path}:metrics.{key}")

    return {
        "metrics": metrics,
        "deprecatedDesignFragments": parse_optional_string_list(
            payload.get("deprecatedDesignFragments"), "deprecatedDesignFragments"
        ),
        "doctrineSiteAuthorityInputs": parse_optional_string_list(
            payload.get("doctrineSiteAuthorityInputs"), "doctrineSiteAuthorityInputs"
        ),
        "doctrineSiteGeneratedViews": parse_optional_string_list(
            payload.get("doctrineSiteGeneratedViews"), "doctrineSiteGeneratedViews"
        ),
    }


def evaluate_topology_threshold(
    value: int, threshold: Dict[str, int | None]
) -> Tuple[str, List[str]]:
    messages: List[str] = []
    fail_above = threshold.get("failAbove")
    fail_below = threshold.get("failBelow")
    warn_above = threshold.get("warnAbove")
    warn_below = threshold.get("warnBelow")

    if fail_above is not None and value > fail_above:
        messages.append(f"value {value} exceeds failAbove {fail_above}")
    if fail_below is not None and value < fail_below:
        messages.append(f"value {value} is below failBelow {fail_below}")
    if messages:
        return "fail", messages

    if warn_above is not None and value > warn_above:
        messages.append(f"value {value} exceeds warnAbove {warn_above}")
    if warn_below is not None and value < warn_below:
        messages.append(f"value {value} is below warnBelow {warn_below}")
    if messages:
        return "warn", messages
    return "ok", []


def collect_topology_metrics(repo_root: Path, contract: Dict[str, Any]) -> Dict[str, int]:
    draft_dir = repo_root / "specs" / "premath" / "draft"
    design_dir = repo_root / "docs" / "design"
    traceability_path = draft_dir / "SPEC-TRACEABILITY.md"
    doctrine_site_path = draft_dir / "DOCTRINE-SITE.json"
    doctrine_site = load_json(doctrine_site_path)
    doctrine_edges = doctrine_site.get("edges")
    if not isinstance(doctrine_edges, list):
        raise ValueError(f"{doctrine_site_path}: edges must be a list")

    authority_inputs = contract.get("doctrineSiteAuthorityInputs", [])
    generated_views = contract.get("doctrineSiteGeneratedViews", [])
    deprecated = contract.get("deprecatedDesignFragments", [])
    if not isinstance(authority_inputs, list):
        authority_inputs = []
    if not isinstance(generated_views, list):
        generated_views = []
    if not isinstance(deprecated, list):
        deprecated = []

    return {
        "draftSpecNodes": count_promoted_draft_specs(draft_dir),
        "specTraceabilityRows": count_traceability_rows(traceability_path),
        "designDocNodes": len(
            [
                path
                for path in design_dir.glob("*.md")
                if path.name != "README.md"
            ]
        ),
        "doctrineSiteEdgeCount": len(doctrine_edges),
        "doctrineSiteAuthorityInputCount": len(
            [rel for rel in authority_inputs if (repo_root / rel).exists()]
        ),
        "doctrineSiteGeneratedViewCount": len(
            [rel for rel in generated_views if (repo_root / rel).exists()]
        ),
        "deprecatedDesignFragmentCount": len(
            [rel for rel in deprecated if (repo_root / rel).exists()]
        ),
    }


def check_topology_budget(
    repo_root: Path,
    topology_budget_path: Path,
) -> Tuple[bool, bool, Dict[str, Any]]:
    reasons: List[str] = []
    warnings: List[str] = []

    contract = load_topology_budget_contract(topology_budget_path)
    thresholds = contract["metrics"]
    metrics = collect_topology_metrics(repo_root, contract)

    details_metrics: Dict[str, Any] = {}
    unknown_threshold_metrics = sorted(set(thresholds.keys()) - set(metrics.keys()))
    for metric_id in unknown_threshold_metrics:
        reasons.append(f"topology metric {metric_id!r} has no evaluator")

    unbudgeted_metrics = sorted(set(metrics.keys()) - set(thresholds.keys()))

    for metric_id in sorted(set(metrics.keys()) & set(thresholds.keys())):
        value = metrics[metric_id]
        threshold = thresholds[metric_id]
        status, messages = evaluate_topology_threshold(value, threshold)
        details_metrics[metric_id] = {
            "value": value,
            "status": status,
            "threshold": threshold,
            "messages": messages,
        }
        if status == "fail":
            for message in messages:
                reasons.append(f"{metric_id}: {message}")
        elif status == "warn":
            for message in messages:
                warnings.append(f"{metric_id}: {message}")

    details = {
        "reasons": reasons,
        "warnings": warnings,
        "budgetPath": str(topology_budget_path),
        "metrics": details_metrics,
        "unbudgetedMetrics": unbudgeted_metrics,
    }
    return bool(reasons), bool(warnings), details


def build_drift_budget_payload(
    repo_root: Path,
    coherence_witness: Dict[str, Any],
    coherence_contract: Dict[str, Any],
    control_plane_contract: Dict[str, Any],
    control_plane_module: Any,
    fixture_suites_module: Any,
    topology_budget_path: Path,
) -> Dict[str, Any]:
    capability_registry_path = (
        repo_root / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
    )
    doctrine_op_registry_path = (
        repo_root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
    )
    spec_index_path = repo_root / "specs" / "premath" / "draft" / "SPEC-INDEX.md"
    conformance_path = repo_root / "specs" / "premath" / "draft" / "CONFORMANCE.md"

    spec_map = parse_spec_index_capability_doc_map(spec_index_path)
    registry_contract = parse_capability_registry(capability_registry_path)
    doctrine_operations = parse_doctrine_operation_registry(doctrine_op_registry_path)
    executable_capabilities = registry_contract.executable_capabilities
    registry_overlay_claims = registry_contract.profile_overlay_claims
    conformance_overlay_claims = parse_conformance_profile_overlay_claims(conformance_path)
    conditional_docs = parse_conditional_capability_docs(coherence_contract)

    scope_details = obligation_details(coherence_witness, "scope_noncontradiction")
    gate_chain_details = obligation_details(coherence_witness, "gate_chain_parity")

    checks: List[Tuple[str, bool, bool, Dict[str, Any]]] = []
    profile_failed, profile_details = check_profile_overlay_claims(
        registry_overlay_claims, conformance_overlay_claims
    )
    checks.append(
        (
            DRIFT_CLASS_PROFILE_OVERLAYS,
            profile_failed,
            False,
            profile_details,
        )
    )
    spec_failed, spec_details = check_spec_index_capability_map(
        spec_map, executable_capabilities, conditional_docs
    )
    checks.append(
        (
            DRIFT_CLASS_SPEC_INDEX,
            spec_failed,
            False,
            spec_details,
        )
    )
    lane_failed, lane_details = check_control_plane_lane_bindings(
        control_plane_contract, control_plane_module, gate_chain_details
    )
    checks.append(
        (
            DRIFT_CLASS_LANE_BINDINGS,
            lane_failed,
            False,
            lane_details,
        )
    )
    kcir_mapping_failed, kcir_mapping_details = check_control_plane_kcir_mappings(
        control_plane_contract,
        control_plane_module,
    )
    checks.append(
        (
            DRIFT_CLASS_KCIR_MAPPINGS,
            kcir_mapping_failed,
            False,
            kcir_mapping_details,
        )
    )
    runtime_route_failed, runtime_route_details = check_runtime_route_bindings(
        control_plane_contract,
        control_plane_module,
        doctrine_operations,
    )
    checks.append(
        (
            DRIFT_CLASS_RUNTIME_ROUTE_BINDINGS,
            runtime_route_failed,
            False,
            runtime_route_details,
        )
    )
    required_failed, required_details = check_coherence_required_obligations(
        coherence_contract, scope_details
    )
    checks.append(
        (
            DRIFT_CLASS_REQUIRED_OBLIGATIONS,
            required_failed,
            False,
            required_details,
        )
    )
    sigpi_failed, sigpi_details = check_sigpi_notation(repo_root)
    checks.append((DRIFT_CLASS_SIGPI_NOTATION, sigpi_failed, False, sigpi_details))
    closure_failed, closure_details = check_cache_input_closure(
        repo_root, fixture_suites_module
    )
    checks.append(
        (
            DRIFT_CLASS_CACHE_CLOSURE,
            closure_failed,
            False,
            closure_details,
        )
    )
    topology_failed, topology_warned, topology_details = check_topology_budget(
        repo_root, topology_budget_path
    )
    checks.append(
        (
            DRIFT_CLASS_TOPOLOGY_BUDGET,
            topology_failed,
            topology_warned,
            topology_details,
        )
    )

    drift_classes = sorted([class_id for class_id, failed, _, _ in checks if failed])
    warning_classes = sorted(
        [
            WARN_CLASS_TOPOLOGY_BUDGET if class_id == DRIFT_CLASS_TOPOLOGY_BUDGET else class_id
            for class_id, failed, warned, _ in checks
            if (not failed and warned)
        ]
    )
    details = {class_id: detail for class_id, _, _, detail in checks}
    payload = {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": "rejected" if drift_classes else "accepted",
        "driftClasses": drift_classes,
        "warningClasses": warning_classes,
        "summary": {
            "checkCount": len(checks),
            "driftCount": len(drift_classes),
            "driftDetected": bool(drift_classes),
            "warningCount": len(warning_classes),
            "warningDetected": bool(warning_classes),
        },
        "details": details,
    }
    return payload


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()

    coherence_contract_path = root / "specs" / "premath" / "draft" / "COHERENCE-CONTRACT.json"
    control_plane_contract_path = (
        root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
    )
    topology_budget_path = (
        args.topology_budget.resolve()
        if args.topology_budget is not None
        else root / "specs" / "process" / "TOPOLOGY-BUDGET.json"
    )
    control_plane_module_path = root / "tools" / "ci" / "control_plane_contract.py"
    fixture_suites_module_path = root / "tools" / "conformance" / "run_fixture_suites.py"

    try:
        control_plane_module = import_module_from_path(
            "premath_control_plane_contract", control_plane_module_path
        )
        fixture_suites_module = import_module_from_path(
            "premath_run_fixture_suites", fixture_suites_module_path
        )
        coherence_contract = load_json(coherence_contract_path)
        control_plane_contract = control_plane_module.load_control_plane_contract(
            control_plane_contract_path
        )
        if args.coherence_json is not None:
            coherence_witness = load_json(args.coherence_json.resolve())
        else:
            coherence_witness = run_coherence_check(root, coherence_contract_path)

        payload = build_drift_budget_payload(
            root,
            coherence_witness,
            coherence_contract,
            control_plane_contract,
            control_plane_module,
            fixture_suites_module,
            topology_budget_path,
        )
    except Exception as exc:  # pragma: no cover - fail-closed CLI guard
        print(f"[drift-budget-check] FAIL ({exc})")
        return 1

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        if payload["result"] == "accepted":
            if payload["summary"]["warningCount"] > 0:
                print(
                    "[drift-budget-check] WARN "
                    f"(checks={payload['summary']['checkCount']}, drift=0, "
                    f"warnings={payload['warningClasses']})"
                )
            else:
                print(
                    "[drift-budget-check] OK "
                    f"(checks={payload['summary']['checkCount']}, drift=0)"
                )
        else:
            print(
                "[drift-budget-check] FAIL "
                f"(driftClasses={payload['driftClasses']})"
            )
    return 0 if payload["result"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
