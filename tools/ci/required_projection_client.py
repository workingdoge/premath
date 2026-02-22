#!/usr/bin/env python3
"""Shared client for core `premath required-projection` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class RequiredProjectionError(ValueError):
    """Required-projection failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-projection payload must be an object")

    if payload.get("schema") != 1:
        raise ValueError("required-projection payload schema must be 1")

    for key in ("projectionPolicy", "projectionDigest"):
        value = payload.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"required-projection payload missing {key}")

    for key in ("changedPaths", "requiredChecks", "reasons"):
        if not isinstance(payload.get(key), list):
            raise ValueError(f"required-projection payload {key} must be a list")

    if not isinstance(payload.get("docsOnly"), bool):
        raise ValueError("required-projection payload docsOnly must be a boolean")
    return payload


def run_required_projection(root: Path, projection_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-projection",
            input_flag="--input",
            request_payload=projection_input,
            validate_payload=_validate_payload,
            default_failure_class="required_projection_invalid",
            default_failure_message="required_projection_invalid: required-projection failed",
            invalid_json_message="required-projection returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredProjectionError(exc.failure_class, exc.reason) from exc
