#!/usr/bin/env python3
"""Shared client for core `premath required-witness` execution."""

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


class RequiredWitnessError(ValueError):
    """Required-witness failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-witness payload must be an object")

    if payload.get("ciSchema") != 1:
        raise ValueError("required-witness payload ciSchema must be 1")
    payload["witnessKind"] = resolve_schema_kind(
        "requiredWitnessKind",
        payload.get("witnessKind"),
        label="required-witness payload witnessKind",
    )
    payload["projectionPolicy"] = resolve_schema_kind(
        "requiredProjectionPolicy",
        payload.get("projectionPolicy"),
        label="required-witness payload projectionPolicy",
    )

    required_string_fields = (
        "projectionPolicy",
        "projectionDigest",
        "verdictClass",
        "deltaSource",
        "policyDigest",
        "squeakSiteProfile",
        "runStartedAt",
        "runFinishedAt",
    )
    for key in required_string_fields:
        value = payload.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"required-witness payload missing {key}")

    required_list_fields = (
        "changedPaths",
        "requiredChecks",
        "executedChecks",
        "results",
        "gateWitnessRefs",
        "operationalFailureClasses",
        "semanticFailureClasses",
        "failureClasses",
        "reasons",
    )
    for key in required_list_fields:
        if not isinstance(payload.get(key), list):
            raise ValueError(f"required-witness payload {key} must be a list")

    if not isinstance(payload.get("docsOnly"), bool):
        raise ValueError("required-witness payload docsOnly must be a boolean")
    run_duration_ms = payload.get("runDurationMs")
    if not isinstance(run_duration_ms, int) or run_duration_ms < 0:
        raise ValueError("required-witness payload runDurationMs must be a non-negative integer")

    return payload


def run_required_witness(root: Path, runtime_payload: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-witness",
            input_flag="--runtime",
            request_payload=runtime_payload,
            validate_payload=_validate_payload,
            default_failure_class="required_witness_runtime_invalid",
            default_failure_message="required_witness_runtime_invalid: required-witness failed",
            invalid_json_message="required-witness returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredWitnessError(exc.failure_class, exc.reason) from exc
