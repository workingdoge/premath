#!/usr/bin/env python3
"""Control-plane KCIR mapping gate client over core CLI surfaces."""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Tuple

from control_plane_contract import (
    CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID,
    CONTROL_PLANE_KCIR_MAPPING_TABLE,
)
from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


KCIR_MAPPING_CONTRACT_VIOLATION = "kcir_mapping_contract_violation"
KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION = (
    "kcir_mapping_legacy_encoding_authority_violation"
)


@dataclass(frozen=True)
class MappingGateReport:
    """Deterministic mapping-gate evaluation report."""

    profile_id: str
    declared_rows: Tuple[str, ...]
    checked_rows: Tuple[str, ...]
    failure_classes: Tuple[str, ...]


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _ordered_failure_classes(values: Tuple[str, ...]) -> Tuple[str, ...]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        if not isinstance(value, str):
            continue
        trimmed = value.strip()
        if not trimmed or trimmed in seen:
            continue
        seen.add(trimmed)
        ordered.append(trimmed)
    return tuple(ordered)


def _default_declared_rows() -> Tuple[str, ...]:
    return tuple(sorted(CONTROL_PLANE_KCIR_MAPPING_TABLE))


def _default_instruction_rows() -> Tuple[str, ...]:
    return (
        "instructionEnvelope",
        "coherenceObligations",
        "doctrineRouteBinding",
        "proposalPayload",
    )


def _default_required_rows() -> Tuple[str, ...]:
    return (
        "coherenceCheckPayload",
        "requiredDecisionInput",
        "coherenceObligations",
        "doctrineRouteBinding",
    )


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("kcir mapping payload must be an object")

    profile_id = payload.get("profileId")
    if not isinstance(profile_id, str):
        raise ValueError("kcir mapping payload profileId must be a string")

    declared_rows = payload.get("declaredRows")
    checked_rows = payload.get("checkedRows")
    failure_classes = payload.get("failureClasses")

    for label, value in (
        ("declaredRows", declared_rows),
        ("checkedRows", checked_rows),
        ("failureClasses", failure_classes),
    ):
        if not isinstance(value, list):
            raise ValueError(f"kcir mapping payload {label} must be a list")
        for idx, item in enumerate(value):
            if not isinstance(item, str) or not item.strip():
                raise ValueError(
                    f"kcir mapping payload {label}[{idx}] must be a non-empty string"
                )

    return payload


def _report_from_payload(payload: Dict[str, Any]) -> MappingGateReport:
    profile_id = payload.get("profileId")
    if not isinstance(profile_id, str) or not profile_id.strip():
        profile_id = CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID

    declared_rows_raw = payload.get("declaredRows", [])
    checked_rows_raw = payload.get("checkedRows", [])
    failures_raw = payload.get("failureClasses", [])

    declared_rows = tuple(
        item.strip()
        for item in declared_rows_raw
        if isinstance(item, str) and item.strip()
    )
    checked_rows = tuple(
        item.strip()
        for item in checked_rows_raw
        if isinstance(item, str) and item.strip()
    )
    failures = tuple(
        item.strip() for item in failures_raw if isinstance(item, str) and item.strip()
    )

    return MappingGateReport(
        profile_id=profile_id,
        declared_rows=declared_rows,
        checked_rows=checked_rows,
        failure_classes=_ordered_failure_classes(failures),
    )


def _error_report(*, checked_rows: Tuple[str, ...]) -> MappingGateReport:
    return MappingGateReport(
        profile_id=CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID,
        declared_rows=_default_declared_rows(),
        checked_rows=checked_rows,
        failure_classes=(KCIR_MAPPING_CONTRACT_VIOLATION,),
    )


def evaluate_instruction_mapping(
    root: Path,
    *,
    instruction_path: Path,
    instruction_id: str,
    strict: bool,
) -> MappingGateReport:
    request_payload: Dict[str, Any] = {
        "repoRoot": str(root),
        "scope": "instruction",
        "instructionPath": str(instruction_path),
        "instructionId": instruction_id,
        "strict": bool(strict),
    }
    try:
        payload = run_core_json_command(
            root,
            subcommand="kcir-mapping-check",
            input_flag="--input",
            request_payload=request_payload,
            validate_payload=_validate_payload,
            default_failure_class=KCIR_MAPPING_CONTRACT_VIOLATION,
            default_failure_message="kcir mapping command failed",
            invalid_json_message="kcir mapping command returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError:
        return _error_report(checked_rows=_default_instruction_rows())

    return _report_from_payload(payload)


def evaluate_required_mapping(
    root: Path,
    *,
    strict: bool,
) -> MappingGateReport:
    request_payload: Dict[str, Any] = {
        "repoRoot": str(root),
        "scope": "required",
        "strict": bool(strict),
    }
    try:
        payload = run_core_json_command(
            root,
            subcommand="kcir-mapping-check",
            input_flag="--input",
            request_payload=request_payload,
            validate_payload=_validate_payload,
            default_failure_class=KCIR_MAPPING_CONTRACT_VIOLATION,
            default_failure_message="kcir mapping command failed",
            invalid_json_message="kcir mapping command returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError:
        return _error_report(checked_rows=_default_required_rows())

    return _report_from_payload(payload)


def render_mapping_summary_lines(report: MappingGateReport) -> list[str]:
    declared_count = len(set(report.declared_rows))
    checked_set = set(report.checked_rows)
    checked_count = len(checked_set)
    missing_rows = sorted(set(report.declared_rows) - checked_set)
    rows = ", ".join(report.checked_rows) if report.checked_rows else "(none)"
    missing = ", ".join(missing_rows) if missing_rows else "(none)"
    failures = ", ".join(report.failure_classes) if report.failure_classes else "(none)"
    return [
        f"- KCIR mapping profile: `{report.profile_id or '(missing)'}`",
        f"- KCIR mapping coverage: `{checked_count}/{declared_count}`",
        f"- KCIR mapping rows: `{rows}`",
        f"- KCIR mapping rows missing: `{missing}`",
        f"- KCIR mapping failures: `{failures}`",
    ]
