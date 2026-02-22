#!/usr/bin/env python3
"""Shared client for core `premath required-decision-verify` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class RequiredDecisionVerifyError(ValueError):
    """Required-decision-verify failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-decision-verify payload must be an object")
    errors = payload.get("errors")
    if not isinstance(errors, list):
        raise ValueError("required-decision-verify payload errors must be a list")
    derived = payload.get("derived")
    if not isinstance(derived, dict):
        raise ValueError("required-decision-verify payload derived must be an object")

    decision = derived.get("decision")
    if errors == [] and decision == "accept":
        for key in (
            "typedCoreProjectionDigest",
            "authorityPayloadDigest",
            "normalizerId",
            "policyDigest",
        ):
            value = derived.get(key)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(
                    f"required-decision-verify payload missing {key} for accept decision"
                )
    return payload


def run_required_decision_verify(root: Path, verify_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-decision-verify",
            input_flag="--input",
            request_payload=verify_input,
            validate_payload=_validate_payload,
            default_failure_class="required_decision_verify_invalid",
            default_failure_message="required_decision_verify_invalid: required-decision-verify failed",
            invalid_json_message="required-decision-verify returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredDecisionVerifyError(exc.failure_class, exc.reason) from exc
