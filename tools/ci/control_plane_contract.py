#!/usr/bin/env python3
"""Shared typed control-plane contract loader."""

from __future__ import annotations

import json
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

    contract_kind = _require_non_empty_string(root.get("contractKind"), "contractKind")
    if contract_kind != CONTROL_PLANE_CONTRACT_KIND:
        raise ValueError(
            f"control-plane contract kind must be {CONTROL_PLANE_CONTRACT_KIND!r}"
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

    required_gate_projection = _require_object(
        root.get("requiredGateProjection"), "requiredGateProjection"
    )
    projection_policy = _require_non_empty_string(
        required_gate_projection.get("projectionPolicy"),
        "requiredGateProjection.projectionPolicy",
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
    required_witness_kind = _require_non_empty_string(
        required_witness.get("witnessKind"),
        "requiredWitness.witnessKind",
    )
    required_decision_kind = _require_non_empty_string(
        required_witness.get("decisionKind"),
        "requiredWitness.decisionKind",
    )

    instruction_witness = _require_object(
        root.get("instructionWitness"),
        "instructionWitness",
    )
    instruction_witness_kind = _require_non_empty_string(
        instruction_witness.get("witnessKind"),
        "instructionWitness.witnessKind",
    )
    instruction_policy_kind = _require_non_empty_string(
        instruction_witness.get("policyKind"),
        "instructionWitness.policyKind",
    )
    instruction_policy_digest_prefix = _require_non_empty_string(
        instruction_witness.get("policyDigestPrefix"),
        "instructionWitness.policyDigestPrefix",
    )

    return {
        "schema": schema,
        "contractKind": contract_kind,
        "evidenceLanes": evidence_lanes,
        "laneArtifactKinds": lane_artifact_kinds,
        "laneOwnership": {
            "checkerCoreOnlyObligations": checker_core_only_obligations,
            "requiredCrossLaneWitnessRoute": required_cross_lane_witness_route,
        },
        "laneFailureClasses": lane_failure_classes,
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

REQUIRED_PROJECTION_POLICY: str = _CONTRACT["requiredGateProjection"]["projectionPolicy"]
REQUIRED_CHECK_IDS: Dict[str, str] = dict(_CONTRACT["requiredGateProjection"]["checkIds"])
REQUIRED_CHECK_ORDER: Tuple[str, ...] = tuple(
    _CONTRACT["requiredGateProjection"]["checkOrder"]
)

REQUIRED_WITNESS_KIND: str = _CONTRACT["requiredWitness"]["witnessKind"]
REQUIRED_DECISION_KIND: str = _CONTRACT["requiredWitness"]["decisionKind"]

INSTRUCTION_WITNESS_KIND: str = _CONTRACT["instructionWitness"]["witnessKind"]
INSTRUCTION_POLICY_KIND: str = _CONTRACT["instructionWitness"]["policyKind"]
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
