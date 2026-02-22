#!/usr/bin/env python3
"""Shared typed control-plane contract loader."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, Tuple


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
