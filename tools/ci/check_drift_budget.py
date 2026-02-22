#!/usr/bin/env python3
"""Fail-closed drift-budget sentinel across docs/contracts/checkers."""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple


SCHEMA = 1
CHECK_KIND = "ci.drift_budget.v1"

DRIFT_CLASS_SPEC_INDEX = "spec_index_capability_map_drift"
DRIFT_CLASS_LANE_BINDINGS = "control_plane_lane_binding_drift"
DRIFT_CLASS_REQUIRED_OBLIGATIONS = "coherence_required_obligation_drift"
DRIFT_CLASS_SIGPI_NOTATION = "sigpi_notation_drift"
DRIFT_CLASS_CACHE_CLOSURE = "coherence_cache_input_closure_drift"

_DOC_MAP_RE = re.compile(r"- `([^`]+)`\s+\(for `([^`]+)`\)")
_SIGPI_ALIAS_RE = re.compile(r"\bSig/Pi\b", re.IGNORECASE)

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


def parse_capability_registry(registry_path: Path) -> List[str]:
    payload = load_json(registry_path)
    capabilities = payload.get("executableCapabilities")
    if not isinstance(capabilities, list) or not capabilities:
        raise ValueError(f"{registry_path}: executableCapabilities must be a non-empty list")
    out: List[str] = []
    for idx, value in enumerate(capabilities):
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{registry_path}: executableCapabilities[{idx}] must be non-empty")
        out.append(value.strip())
    return out


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

    checker_expected_core = as_sorted_strings(
        lane_registry.get("expectedCheckerCoreOnlyObligations", ())
    )
    checker_required_route = lane_registry.get("requiredCrossLaneWitnessRoute")
    checker_required_failures = as_sorted_strings(
        lane_registry.get("requiredLaneFailureClasses", ())
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
        },
        "checker": {
            "evidenceLanes": checker_lane_values,
            "laneArtifactKinds": checker_kinds,
            "expectedCheckerCoreOnlyObligations": checker_expected_core,
            "requiredCrossLaneWitnessRoute": checker_required_route,
            "requiredLaneFailureClasses": checker_required_failures,
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
        },
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


def build_drift_budget_payload(
    repo_root: Path,
    coherence_witness: Dict[str, Any],
    coherence_contract: Dict[str, Any],
    control_plane_contract: Dict[str, Any],
    control_plane_module: Any,
    fixture_suites_module: Any,
) -> Dict[str, Any]:
    capability_registry_path = (
        repo_root / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
    )
    spec_index_path = repo_root / "specs" / "premath" / "draft" / "SPEC-INDEX.md"

    spec_map = parse_spec_index_capability_doc_map(spec_index_path)
    executable_capabilities = parse_capability_registry(capability_registry_path)
    conditional_docs = parse_conditional_capability_docs(coherence_contract)

    scope_details = obligation_details(coherence_witness, "scope_noncontradiction")
    gate_chain_details = obligation_details(coherence_witness, "gate_chain_parity")

    checks: List[Tuple[str, bool, Dict[str, Any]]] = []
    checks.append(
        (
            DRIFT_CLASS_SPEC_INDEX,
            *check_spec_index_capability_map(spec_map, executable_capabilities, conditional_docs),
        )
    )
    checks.append(
        (
            DRIFT_CLASS_LANE_BINDINGS,
            *check_control_plane_lane_bindings(
                control_plane_contract, control_plane_module, gate_chain_details
            ),
        )
    )
    checks.append(
        (
            DRIFT_CLASS_REQUIRED_OBLIGATIONS,
            *check_coherence_required_obligations(coherence_contract, scope_details),
        )
    )
    checks.append((DRIFT_CLASS_SIGPI_NOTATION, *check_sigpi_notation(repo_root)))
    checks.append(
        (
            DRIFT_CLASS_CACHE_CLOSURE,
            *check_cache_input_closure(repo_root, fixture_suites_module),
        )
    )

    drift_classes = sorted([class_id for class_id, failed, _ in checks if failed])
    details = {
        class_id: detail
        for class_id, _, detail in checks
    }
    payload = {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": "rejected" if drift_classes else "accepted",
        "driftClasses": drift_classes,
        "summary": {
            "checkCount": len(checks),
            "driftCount": len(drift_classes),
            "driftDetected": bool(drift_classes),
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
        )
    except Exception as exc:  # pragma: no cover - fail-closed CLI guard
        print(f"[drift-budget-check] FAIL ({exc})")
        return 1

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        if payload["result"] == "accepted":
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
