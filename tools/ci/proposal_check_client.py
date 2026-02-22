#!/usr/bin/env python3
"""Shared client for core `premath proposal-check` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class ProposalCheckError(ValueError):
    """Proposal-check failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("proposal-check payload must be an object")

    canonical = payload.get("canonical")
    digest = payload.get("digest")
    kcir_ref = payload.get("kcirRef")
    obligations = payload.get("obligations", [])
    discharge = payload.get("discharge")

    if not isinstance(canonical, dict):
        raise ValueError("proposal-check canonical payload must be an object")
    if not isinstance(digest, str) or not digest:
        raise ValueError("proposal-check digest is missing")
    if not isinstance(kcir_ref, str) or not kcir_ref:
        raise ValueError("proposal-check kcirRef is missing")
    if not isinstance(obligations, list):
        raise ValueError("proposal-check obligations must be a list")
    if not isinstance(discharge, dict):
        raise ValueError("proposal-check discharge payload must be an object")

    return {
        "canonical": canonical,
        "digest": digest,
        "kcirRef": kcir_ref,
        "obligations": obligations,
        "discharge": discharge,
    }


def _map_validation_failure_class(default_failure_class: str, reason: str) -> str:
    if default_failure_class != "proposal_invalid_shape":
        return default_failure_class
    if "digest is missing" in reason:
        return "proposal_nondeterministic"
    if "kcirRef is missing" in reason:
        return "proposal_kcir_ref_mismatch"
    if "obligations must be a list" in reason or "discharge payload" in reason:
        return "proposal_invalid_step"
    return default_failure_class


def run_proposal_check(root: Path, proposal: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="proposal-check",
            input_flag="--proposal",
            request_payload=proposal,
            validate_payload=_validate_payload,
            default_failure_class="proposal_invalid_shape",
            default_failure_message="proposal_invalid_shape: proposal-check failed",
            invalid_json_message="proposal-check returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        failure_class = _map_validation_failure_class(exc.failure_class, exc.reason)
        raise ProposalCheckError(failure_class, exc.reason) from exc
