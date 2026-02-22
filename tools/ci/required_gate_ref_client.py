#!/usr/bin/env python3
"""Shared client for core `premath required-gate-ref` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class RequiredGateRefError(ValueError):
    """Required-gate-ref failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-gate-ref payload must be an object")
    ref = payload.get("gateWitnessRef")
    if not isinstance(ref, dict):
        raise ValueError("required-gate-ref payload gateWitnessRef must be an object")
    for key in ("checkId", "artifactRelPath", "sha256", "source"):
        value = ref.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"required-gate-ref payload gateWitnessRef.{key} must be a non-empty string")
    if not isinstance(ref.get("failureClasses"), list):
        raise ValueError("required-gate-ref payload gateWitnessRef.failureClasses must be a list")
    gate_payload = payload.get("gatePayload")
    if gate_payload is not None and not isinstance(gate_payload, dict):
        raise ValueError("required-gate-ref payload gatePayload must be object|null")
    return payload


def run_required_gate_ref(root: Path, request_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-gate-ref",
            input_flag="--input",
            request_payload=request_input,
            validate_payload=_validate_payload,
            default_failure_class="required_gate_ref_invalid",
            default_failure_message="required_gate_ref_invalid: required-gate-ref failed",
            invalid_json_message="required-gate-ref returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredGateRefError(exc.failure_class, exc.reason) from exc
