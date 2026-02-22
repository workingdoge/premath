#!/usr/bin/env python3
"""Shared client for core `premath required-witness-decide` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)
from control_plane_contract import resolve_schema_kind


class RequiredWitnessDecideError(ValueError):
    """Required-witness-decide failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-witness-decide payload must be an object")
    payload["decisionKind"] = resolve_schema_kind(
        "requiredDecisionKind",
        payload.get("decisionKind"),
        label="required-witness-decide payload decisionKind",
    )
    decision = payload.get("decision")
    if decision not in {"accept", "reject"}:
        raise ValueError("required-witness-decide payload decision must be accept|reject")
    reason_class = payload.get("reasonClass")
    if not isinstance(reason_class, str) or not reason_class.strip():
        raise ValueError("required-witness-decide payload reasonClass must be a non-empty string")
    if not isinstance(payload.get("errors"), list):
        raise ValueError("required-witness-decide payload errors must be a list")
    typed_fields = (
        "typedCoreProjectionDigest",
        "authorityPayloadDigest",
        "normalizerId",
        "policyDigest",
    )
    for key in typed_fields:
        value = payload.get(key)
        if value is None:
            if decision == "accept":
                raise ValueError(
                    f"required-witness-decide payload missing {key} for accept decision"
                )
            continue
        if not isinstance(value, str) or not value.strip():
            raise ValueError(
                f"required-witness-decide payload {key} must be a non-empty string when present"
            )
    return payload


def run_required_witness_decide(root: Path, decide_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-witness-decide",
            input_flag="--input",
            request_payload=decide_input,
            validate_payload=_validate_payload,
            default_failure_class="required_witness_decide_invalid",
            default_failure_message="required_witness_decide_invalid: required-witness-decide failed",
            invalid_json_message="required-witness-decide returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredWitnessDecideError(exc.failure_class, exc.reason) from exc
