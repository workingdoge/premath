#!/usr/bin/env python3
"""Shared client for core `premath required-delta` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)

class RequiredDeltaError(ValueError):
    """Required-delta failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-delta payload must be an object")

    if payload.get("schema") != 1:
        raise ValueError("required-delta payload schema must be 1")

    for key in ("deltaKind", "source", "toRef"):
        value = payload.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"required-delta payload missing {key}")

    changed_paths = payload.get("changedPaths")
    if not isinstance(changed_paths, list):
        raise ValueError("required-delta payload changedPaths must be a list")

    from_ref = payload.get("fromRef")
    if from_ref is not None and not isinstance(from_ref, str):
        raise ValueError("required-delta payload fromRef must be string|null")
    return payload


def run_required_delta(root: Path, delta_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-delta",
            input_flag="--input",
            request_payload=delta_input,
            validate_payload=_validate_payload,
            default_failure_class="required_delta_invalid",
            default_failure_message="required_delta_invalid: required-delta failed",
            invalid_json_message="required-delta returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredDeltaError(exc.failure_class, exc.reason) from exc
