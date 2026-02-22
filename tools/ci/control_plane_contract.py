#!/usr/bin/env python3
"""Shared typed control-plane contract loader."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, Optional, Tuple


CONTROL_PLANE_CONTRACT_KIND = "premath.control_plane.contract.v1"
CONTROL_PLANE_CONTRACT_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "CONTROL-PLANE-CONTRACT.json"
)
_EPOCH_RE = re.compile(r"^\d{4}-(0[1-9]|1[0-2])$")
_REQUIRED_SCHEMA_KIND_FAMILIES = (
    "controlPlaneContractKind",
    "requiredWitnessKind",
    "requiredDecisionKind",
    "instructionWitnessKind",
    "instructionPolicyKind",
    "requiredProjectionPolicy",
    "requiredDeltaKind",
)
_MAX_ALIAS_RUNWAY_MONTHS = 12
_SCHEMA_LIFECYCLE_GOVERNANCE_MODES = ("rollover", "freeze")


def _require_non_empty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value.strip()


def _require_object(value: Any, label: str) -> Dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    return value


def _require_string_list(value: Any, label: str) -> Tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label} must be a non-empty list")
    out = []
    for idx, item in enumerate(value):
        out.append(_require_non_empty_string(item, f"{label}[{idx}]"))
    if len(set(out)) != len(out):
        raise ValueError(f"{label} must not contain duplicates")
    return tuple(out)


def _require_optional_string_list(value: Any, label: str) -> Tuple[str, ...]:
    if value is None:
        return tuple()
    return _require_string_list(value, label)


def _require_epoch(value: Any, label: str) -> str:
    epoch = _require_non_empty_string(value, label)
    if _EPOCH_RE.fullmatch(epoch) is None:
        raise ValueError(f"{label} must use YYYY-MM with zero-padded month")
    return epoch


def _require_positive_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{label} must be an integer")
    if value < 1:
        raise ValueError(f"{label} must be >= 1")
    return value


def _epoch_to_month_index(epoch: str) -> int:
    year_str, month_str = epoch.split("-", 1)
    return int(year_str) * 12 + int(month_str)


def _validate_schema_lifecycle_epoch_discipline(
    active_epoch: str,
    kind_families: Dict[str, Dict[str, Any]],
) -> Dict[str, Any]:
    alias_support_epochs = []
    for family in kind_families.values():
        aliases = family.get("compatibilityAliases", {})
        if not isinstance(aliases, dict):
            continue
        for alias_row in aliases.values():
            if not isinstance(alias_row, dict):
                continue
            support_until_epoch = alias_row.get("supportUntilEpoch")
            if isinstance(support_until_epoch, str) and support_until_epoch:
                alias_support_epochs.append(support_until_epoch)

    if not alias_support_epochs:
        return {
            "rolloverEpoch": None,
            "aliasRunwayMonths": 0,
            "maxAliasRunwayMonths": _MAX_ALIAS_RUNWAY_MONTHS,
        }

    unique_support_epochs = sorted(set(alias_support_epochs))
    if len(unique_support_epochs) != 1:
        raise ValueError(
            "schemaLifecycle rollover policy requires one shared supportUntilEpoch "
            f"across all compatibility aliases (got {unique_support_epochs})"
        )

    rollover_epoch = unique_support_epochs[0]
    runway_months = _epoch_to_month_index(rollover_epoch) - _epoch_to_month_index(
        active_epoch
    )
    if runway_months < 1:
        raise ValueError(
            "schemaLifecycle rollover policy requires supportUntilEpoch to be after "
            f"activeEpoch (activeEpoch={active_epoch!r}, rolloverEpoch={rollover_epoch!r})"
        )
    if runway_months > _MAX_ALIAS_RUNWAY_MONTHS:
        raise ValueError(
            "schemaLifecycle rollover policy exceeds max runway "
            f"({_MAX_ALIAS_RUNWAY_MONTHS} months): "
            f"activeEpoch={active_epoch!r}, rolloverEpoch={rollover_epoch!r}"
        )

    return {
        "rolloverEpoch": rollover_epoch,
        "aliasRunwayMonths": runway_months,
        "maxAliasRunwayMonths": _MAX_ALIAS_RUNWAY_MONTHS,
    }


def _validate_schema_lifecycle_governance(
    governance: Dict[str, Any],
    *,
    schema_epoch_discipline: Dict[str, Any],
) -> Dict[str, Any]:
    mode = _require_non_empty_string(governance.get("mode"), "schemaLifecycle.governance.mode")
    if mode not in _SCHEMA_LIFECYCLE_GOVERNANCE_MODES:
        raise ValueError(
            "schemaLifecycle.governance.mode must be one of: "
            + ", ".join(_SCHEMA_LIFECYCLE_GOVERNANCE_MODES)
        )
    decision_ref = _require_non_empty_string(
        governance.get("decisionRef"),
        "schemaLifecycle.governance.decisionRef",
    )
    owner = _require_non_empty_string(
        governance.get("owner"),
        "schemaLifecycle.governance.owner",
    )

    rollover_cadence_raw = governance.get("rolloverCadenceMonths")
    freeze_reason_raw = governance.get("freezeReason")

    rollover_epoch = schema_epoch_discipline.get("rolloverEpoch")
    alias_runway_months = int(schema_epoch_discipline.get("aliasRunwayMonths", 0))

    if mode == "rollover":
        rollover_cadence_months = _require_positive_int(
            rollover_cadence_raw,
            "schemaLifecycle.governance.rolloverCadenceMonths",
        )
        if rollover_cadence_months > _MAX_ALIAS_RUNWAY_MONTHS:
            raise ValueError(
                "schemaLifecycle.governance.rolloverCadenceMonths must be <= "
                f"{_MAX_ALIAS_RUNWAY_MONTHS}"
            )
        if rollover_epoch is None:
            raise ValueError(
                "schemaLifecycle.governance.mode=rollover requires at least one "
                "compatibility alias with supportUntilEpoch"
            )
        if alias_runway_months > rollover_cadence_months:
            raise ValueError(
                "schemaLifecycle.governance.rolloverCadenceMonths must be >= alias runway "
                f"(runway={alias_runway_months}, cadence={rollover_cadence_months})"
            )
        if freeze_reason_raw is not None:
            raise ValueError(
                "schemaLifecycle.governance.freezeReason is only allowed when mode=freeze"
            )
        return {
            "mode": mode,
            "decisionRef": decision_ref,
            "owner": owner,
            "rolloverCadenceMonths": rollover_cadence_months,
            "freezeReason": None,
        }

    if rollover_cadence_raw is not None:
        raise ValueError(
            "schemaLifecycle.governance.rolloverCadenceMonths is only allowed when mode=rollover"
        )
    freeze_reason = _require_non_empty_string(
        freeze_reason_raw,
        "schemaLifecycle.governance.freezeReason",
    )
    if rollover_epoch is not None or alias_runway_months != 0:
        raise ValueError(
            "schemaLifecycle.governance.mode=freeze requires no active compatibility aliases"
        )
    return {
        "mode": mode,
        "decisionRef": decision_ref,
        "owner": owner,
        "rolloverCadenceMonths": None,
        "freezeReason": freeze_reason,
    }


def _require_schema_kind_family(value: Any, label: str) -> Dict[str, Any]:
    family = _require_object(value, label)
    canonical_kind = _require_non_empty_string(
        family.get("canonicalKind"), f"{label}.canonicalKind"
    )
    aliases_raw = family.get("compatibilityAliases", [])
    if not isinstance(aliases_raw, list):
        raise ValueError(f"{label}.compatibilityAliases must be a list")
    aliases: Dict[str, Dict[str, str]] = {}
    for idx, alias_row_raw in enumerate(aliases_raw):
        alias_row = _require_object(
            alias_row_raw, f"{label}.compatibilityAliases[{idx}]"
        )
        alias_kind = _require_non_empty_string(
            alias_row.get("aliasKind"),
            f"{label}.compatibilityAliases[{idx}].aliasKind",
        )
        support_until_epoch = _require_epoch(
            alias_row.get("supportUntilEpoch"),
            f"{label}.compatibilityAliases[{idx}].supportUntilEpoch",
        )
        replacement_kind = _require_non_empty_string(
            alias_row.get("replacementKind"),
            f"{label}.compatibilityAliases[{idx}].replacementKind",
        )
        if alias_kind == canonical_kind:
            raise ValueError(
                f"{label}.compatibilityAliases[{idx}].aliasKind must differ from canonicalKind"
            )
        if replacement_kind != canonical_kind:
            raise ValueError(
                f"{label}.compatibilityAliases[{idx}].replacementKind must match canonicalKind"
            )
        if alias_kind in aliases:
            raise ValueError(f"{label}.compatibilityAliases aliasKind values must be unique")
        aliases[alias_kind] = {
            "supportUntilEpoch": support_until_epoch,
            "replacementKind": replacement_kind,
        }
    return {
        "canonicalKind": canonical_kind,
        "compatibilityAliases": aliases,
    }


def _resolve_kind_in_family(
    family_id: str,
    *,
    family: Dict[str, Any],
    kind: str,
    active_epoch: str,
    label: str,
) -> str:
    canonical_kind = family["canonicalKind"]
    if kind == canonical_kind:
        return canonical_kind
    aliases = family.get("compatibilityAliases", {})
    alias_row = aliases.get(kind)
    if alias_row is None:
        raise ValueError(
            f"{label} kind {kind!r} is not supported for schemaLifecycle.kindFamilies.{family_id} "
            f"(canonicalKind={canonical_kind!r})"
        )
    support_until_epoch = alias_row["supportUntilEpoch"]
    if active_epoch > support_until_epoch:
        raise ValueError(
            f"{label} kind {kind!r} expired at supportUntilEpoch={support_until_epoch!r} "
            f"for schemaLifecycle.kindFamilies.{family_id} (activeEpoch={active_epoch!r}, "
            f"canonicalKind={canonical_kind!r})"
        )
    return canonical_kind


def load_control_plane_contract(path: Path = CONTROL_PLANE_CONTRACT_PATH) -> Dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"failed to read control-plane contract {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json in control-plane contract {path}: {exc}") from exc
    root = _require_object(payload, "control-plane contract root")

    schema = root.get("schema")
    if schema != 1:
        raise ValueError("control-plane contract schema must be 1")

    schema_lifecycle = _require_object(root.get("schemaLifecycle"), "schemaLifecycle")
    active_epoch = _require_epoch(
        schema_lifecycle.get("activeEpoch"), "schemaLifecycle.activeEpoch"
    )
    schema_lifecycle_governance_obj = _require_object(
        schema_lifecycle.get("governance"),
        "schemaLifecycle.governance",
    )
    kind_families_raw = _require_object(
        schema_lifecycle.get("kindFamilies"), "schemaLifecycle.kindFamilies"
    )
    unknown_kind_families = sorted(
        set(kind_families_raw) - set(_REQUIRED_SCHEMA_KIND_FAMILIES)
    )
    if unknown_kind_families:
        raise ValueError(
            "schemaLifecycle.kindFamilies includes unknown families: "
            + ", ".join(unknown_kind_families)
        )
    kind_families: Dict[str, Dict[str, Any]] = {}
    for family_id in _REQUIRED_SCHEMA_KIND_FAMILIES:
        kind_families[family_id] = _require_schema_kind_family(
            kind_families_raw.get(family_id),
            f"schemaLifecycle.kindFamilies.{family_id}",
        )
    schema_epoch_discipline = _validate_schema_lifecycle_epoch_discipline(
        active_epoch, kind_families
    )
    schema_lifecycle_governance = _validate_schema_lifecycle_governance(
        schema_lifecycle_governance_obj,
        schema_epoch_discipline=schema_epoch_discipline,
    )

    contract_kind_declared = _require_non_empty_string(root.get("contractKind"), "contractKind")
    contract_kind = _resolve_kind_in_family(
        "controlPlaneContractKind",
        family=kind_families["controlPlaneContractKind"],
        kind=contract_kind_declared,
        active_epoch=active_epoch,
        label="contractKind",
    )
    if contract_kind != CONTROL_PLANE_CONTRACT_KIND:
        raise ValueError(
            f"control-plane contract kind must resolve to {CONTROL_PLANE_CONTRACT_KIND!r}"
        )

    evidence_lanes: Dict[str, str] = {}
    evidence_lanes_raw = root.get("evidenceLanes")
    if evidence_lanes_raw is not None:
        evidence_lanes_obj = _require_object(evidence_lanes_raw, "evidenceLanes")
        required_lane_keys = (
            "semanticDoctrine",
            "strictChecker",
            "witnessCommutation",
            "runtimeTransport",
        )
        for key in required_lane_keys:
            evidence_lanes[key] = _require_non_empty_string(
                evidence_lanes_obj.get(key), f"evidenceLanes.{key}"
            )
        if len(set(evidence_lanes.values())) != len(evidence_lanes):
            raise ValueError("evidenceLanes values must not contain duplicates")

    lane_artifact_kinds: Dict[str, Tuple[str, ...]] = {}
    lane_artifact_kinds_raw = root.get("laneArtifactKinds")
    if lane_artifact_kinds_raw is not None:
        lane_artifact_kinds_obj = _require_object(
            lane_artifact_kinds_raw, "laneArtifactKinds"
        )
        for lane_id, kinds_raw in lane_artifact_kinds_obj.items():
            lane_id_norm = _require_non_empty_string(
                lane_id, "laneArtifactKinds.<laneId>"
            )
            lane_artifact_kinds[lane_id_norm] = _require_string_list(
                kinds_raw, f"laneArtifactKinds.{lane_id_norm}"
            )
        if evidence_lanes and not set(lane_artifact_kinds).issubset(
            set(evidence_lanes.values())
        ):
            raise ValueError(
                "laneArtifactKinds keys must be subset of evidenceLanes values"
            )

    checker_core_only_obligations: Tuple[str, ...] = tuple()
    required_cross_lane_witness_route: Optional[str] = None
    lane_ownership_raw = root.get("laneOwnership")
    if lane_ownership_raw is not None:
        lane_ownership = _require_object(lane_ownership_raw, "laneOwnership")
        checker_core_only_obligations = _require_optional_string_list(
            lane_ownership.get("checkerCoreOnlyObligations"),
            "laneOwnership.checkerCoreOnlyObligations",
        )
        required_route_obj = lane_ownership.get("requiredCrossLaneWitnessRoute")
        if required_route_obj is not None:
            required_route = _require_object(
                required_route_obj, "laneOwnership.requiredCrossLaneWitnessRoute"
            )
            required_cross_lane_witness_route = _require_non_empty_string(
                required_route.get("pullbackBaseChange"),
                "laneOwnership.requiredCrossLaneWitnessRoute.pullbackBaseChange",
            )

    lane_failure_classes = _require_optional_string_list(
        root.get("laneFailureClasses"), "laneFailureClasses"
    )

    harness_retry_obj = _require_object(
        root.get("harnessRetry"),
        "harnessRetry",
    )
    harness_retry_policy_kind = _require_non_empty_string(
        harness_retry_obj.get("policyKind"),
        "harnessRetry.policyKind",
    )
    harness_retry_policy_path = _require_non_empty_string(
        harness_retry_obj.get("policyPath"),
        "harnessRetry.policyPath",
    )
    harness_retry_escalation_actions = _require_string_list(
        harness_retry_obj.get("escalationActions"),
        "harnessRetry.escalationActions",
    )
    harness_retry_active_issue_env_keys = _require_string_list(
        harness_retry_obj.get("activeIssueEnvKeys"),
        "harnessRetry.activeIssueEnvKeys",
    )
    harness_retry_issues_path_env_key = _require_non_empty_string(
        harness_retry_obj.get("issuesPathEnvKey"),
        "harnessRetry.issuesPathEnvKey",
    )
    harness_retry_session_path_env_key = _require_non_empty_string(
        harness_retry_obj.get("sessionPathEnvKey"),
        "harnessRetry.sessionPathEnvKey",
    )
    harness_retry_session_path_default = _require_non_empty_string(
        harness_retry_obj.get("sessionPathDefault"),
        "harnessRetry.sessionPathDefault",
    )
    harness_retry_session_issue_field = _require_non_empty_string(
        harness_retry_obj.get("sessionIssueField"),
        "harnessRetry.sessionIssueField",
    )

    required_gate_projection = _require_object(
        root.get("requiredGateProjection"), "requiredGateProjection"
    )
    projection_policy = _resolve_kind_in_family(
        "requiredProjectionPolicy",
        family=kind_families["requiredProjectionPolicy"],
        kind=_require_non_empty_string(
            required_gate_projection.get("projectionPolicy"),
            "requiredGateProjection.projectionPolicy",
        ),
        active_epoch=active_epoch,
        label="requiredGateProjection.projectionPolicy",
    )
    check_ids_raw = _require_object(
        required_gate_projection.get("checkIds"),
        "requiredGateProjection.checkIds",
    )
    required_check_id_keys = (
        "baseline",
        "build",
        "test",
        "testToy",
        "testKcirToy",
        "conformanceCheck",
        "conformanceRun",
        "doctrineCheck",
    )
    check_ids: Dict[str, str] = {}
    for key in required_check_id_keys:
        check_ids[key] = _require_non_empty_string(
            check_ids_raw.get(key), f"requiredGateProjection.checkIds.{key}"
        )
    if len(set(check_ids.values())) != len(check_ids):
        raise ValueError("requiredGateProjection.checkIds must not contain duplicate values")
    check_order = _require_string_list(
        required_gate_projection.get("checkOrder"),
        "requiredGateProjection.checkOrder",
    )
    if set(check_order) != set(check_ids.values()):
        raise ValueError(
            "requiredGateProjection.checkOrder must cover exactly requiredGateProjection.checkIds values"
        )

    required_witness = _require_object(root.get("requiredWitness"), "requiredWitness")
    required_witness_kind = _resolve_kind_in_family(
        "requiredWitnessKind",
        family=kind_families["requiredWitnessKind"],
        kind=_require_non_empty_string(
            required_witness.get("witnessKind"),
            "requiredWitness.witnessKind",
        ),
        active_epoch=active_epoch,
        label="requiredWitness.witnessKind",
    )
    required_decision_kind = _resolve_kind_in_family(
        "requiredDecisionKind",
        family=kind_families["requiredDecisionKind"],
        kind=_require_non_empty_string(
            required_witness.get("decisionKind"),
            "requiredWitness.decisionKind",
        ),
        active_epoch=active_epoch,
        label="requiredWitness.decisionKind",
    )

    instruction_witness = _require_object(
        root.get("instructionWitness"),
        "instructionWitness",
    )
    instruction_witness_kind = _resolve_kind_in_family(
        "instructionWitnessKind",
        family=kind_families["instructionWitnessKind"],
        kind=_require_non_empty_string(
            instruction_witness.get("witnessKind"),
            "instructionWitness.witnessKind",
        ),
        active_epoch=active_epoch,
        label="instructionWitness.witnessKind",
    )
    instruction_policy_kind = _resolve_kind_in_family(
        "instructionPolicyKind",
        family=kind_families["instructionPolicyKind"],
        kind=_require_non_empty_string(
            instruction_witness.get("policyKind"),
            "instructionWitness.policyKind",
        ),
        active_epoch=active_epoch,
        label="instructionWitness.policyKind",
    )
    instruction_policy_digest_prefix = _require_non_empty_string(
        instruction_witness.get("policyDigestPrefix"),
        "instructionWitness.policyDigestPrefix",
    )

    return {
        "schema": schema,
        "contractKind": contract_kind,
        "schemaLifecycle": {
            "activeEpoch": active_epoch,
            "governance": schema_lifecycle_governance,
            "kindFamilies": kind_families,
            "epochDiscipline": schema_epoch_discipline,
        },
        "evidenceLanes": evidence_lanes,
        "laneArtifactKinds": lane_artifact_kinds,
        "laneOwnership": {
            "checkerCoreOnlyObligations": checker_core_only_obligations,
            "requiredCrossLaneWitnessRoute": required_cross_lane_witness_route,
        },
        "laneFailureClasses": lane_failure_classes,
        "harnessRetry": {
            "policyKind": harness_retry_policy_kind,
            "policyPath": harness_retry_policy_path,
            "escalationActions": harness_retry_escalation_actions,
            "activeIssueEnvKeys": harness_retry_active_issue_env_keys,
            "issuesPathEnvKey": harness_retry_issues_path_env_key,
            "sessionPathEnvKey": harness_retry_session_path_env_key,
            "sessionPathDefault": harness_retry_session_path_default,
            "sessionIssueField": harness_retry_session_issue_field,
        },
        "requiredGateProjection": {
            "projectionPolicy": projection_policy,
            "checkIds": check_ids,
            "checkOrder": check_order,
        },
        "requiredWitness": {
            "witnessKind": required_witness_kind,
            "decisionKind": required_decision_kind,
        },
        "instructionWitness": {
            "witnessKind": instruction_witness_kind,
            "policyKind": instruction_policy_kind,
            "policyDigestPrefix": instruction_policy_digest_prefix,
        },
    }


_CONTRACT = load_control_plane_contract()
SCHEMA_LIFECYCLE_ACTIVE_EPOCH: str = _CONTRACT["schemaLifecycle"]["activeEpoch"]
SCHEMA_LIFECYCLE_GOVERNANCE_MODE: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("mode", "")
SCHEMA_LIFECYCLE_GOVERNANCE_DECISION_REF: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("decisionRef", "")
SCHEMA_LIFECYCLE_GOVERNANCE_OWNER: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("owner", "")
SCHEMA_LIFECYCLE_ROLLOVER_CADENCE_MONTHS: int | None = _CONTRACT[
    "schemaLifecycle"
].get("governance", {}).get("rolloverCadenceMonths")
SCHEMA_LIFECYCLE_FREEZE_REASON: str | None = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("freezeReason")
SCHEMA_LIFECYCLE_EPOCH_DISCIPLINE: Dict[str, Any] = dict(
    _CONTRACT["schemaLifecycle"].get("epochDiscipline", {})
)
SCHEMA_LIFECYCLE_ROLLOVER_EPOCH: str | None = SCHEMA_LIFECYCLE_EPOCH_DISCIPLINE.get(
    "rolloverEpoch"
)
SCHEMA_KIND_FAMILIES: Dict[str, Dict[str, Any]] = dict(
    _CONTRACT["schemaLifecycle"]["kindFamilies"]
)


def canonical_schema_kind(family_id: str) -> str:
    family = SCHEMA_KIND_FAMILIES.get(family_id)
    if not isinstance(family, dict):
        raise ValueError(
            f"unknown schemaLifecycle kind family: {family_id!r}"
        )
    canonical_kind = family.get("canonicalKind")
    if not isinstance(canonical_kind, str) or not canonical_kind:
        raise ValueError(
            f"schemaLifecycle.kindFamilies.{family_id} missing canonicalKind"
        )
    return canonical_kind


def resolve_schema_kind(
    family_id: str,
    kind: Any,
    *,
    active_epoch: str | None = None,
    label: str | None = None,
) -> str:
    family = SCHEMA_KIND_FAMILIES.get(family_id)
    if not isinstance(family, dict):
        raise ValueError(
            f"unknown schemaLifecycle kind family: {family_id!r}"
        )
    effective_epoch = _require_epoch(
        active_epoch if active_epoch is not None else SCHEMA_LIFECYCLE_ACTIVE_EPOCH,
        "schemaLifecycle.activeEpoch",
    )
    kind_label = label or f"schemaLifecycle.kindFamilies.{family_id}"
    kind_value = _require_non_empty_string(kind, kind_label)
    return _resolve_kind_in_family(
        family_id,
        family=family,
        kind=kind_value,
        active_epoch=effective_epoch,
        label=kind_label,
    )


REQUIRED_PROJECTION_POLICY: str = canonical_schema_kind("requiredProjectionPolicy")
REQUIRED_CHECK_IDS: Dict[str, str] = dict(_CONTRACT["requiredGateProjection"]["checkIds"])
REQUIRED_CHECK_ORDER: Tuple[str, ...] = tuple(
    _CONTRACT["requiredGateProjection"]["checkOrder"]
)

REQUIRED_WITNESS_KIND: str = canonical_schema_kind("requiredWitnessKind")
REQUIRED_DECISION_KIND: str = canonical_schema_kind("requiredDecisionKind")
REQUIRED_DELTA_KIND: str = canonical_schema_kind("requiredDeltaKind")

INSTRUCTION_WITNESS_KIND: str = canonical_schema_kind("instructionWitnessKind")
INSTRUCTION_POLICY_KIND: str = canonical_schema_kind("instructionPolicyKind")
INSTRUCTION_POLICY_DIGEST_PREFIX: str = _CONTRACT["instructionWitness"][
    "policyDigestPrefix"
]

EVIDENCE_LANES: Dict[str, str] = dict(_CONTRACT.get("evidenceLanes", {}))
LANE_ARTIFACT_KINDS: Dict[str, Tuple[str, ...]] = dict(
    _CONTRACT.get("laneArtifactKinds", {})
)
CHECKER_CORE_ONLY_OBLIGATIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("laneOwnership", {}).get("checkerCoreOnlyObligations", ())
)
REQUIRED_CROSS_LANE_WITNESS_ROUTE: Optional[str] = _CONTRACT.get(
    "laneOwnership", {}
).get("requiredCrossLaneWitnessRoute")
LANE_FAILURE_CLASSES: Tuple[str, ...] = tuple(_CONTRACT.get("laneFailureClasses", ()))

HARNESS_RETRY_POLICY_KIND: str = _CONTRACT.get("harnessRetry", {}).get(
    "policyKind",
    "",
)
HARNESS_RETRY_POLICY_PATH: str = _CONTRACT.get("harnessRetry", {}).get(
    "policyPath",
    "",
)
HARNESS_ESCALATION_ACTIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("harnessRetry", {}).get("escalationActions", ())
)
HARNESS_ACTIVE_ISSUE_ENV_KEYS: Tuple[str, ...] = tuple(
    _CONTRACT.get("harnessRetry", {}).get("activeIssueEnvKeys", ())
)
HARNESS_ISSUES_PATH_ENV_KEY: str = _CONTRACT.get("harnessRetry", {}).get(
    "issuesPathEnvKey",
    "",
)
HARNESS_SESSION_PATH_ENV_KEY: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionPathEnvKey",
    "",
)
HARNESS_SESSION_PATH_DEFAULT: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionPathDefault",
    "",
)
HARNESS_SESSION_ISSUE_FIELD: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionIssueField",
    "",
)
