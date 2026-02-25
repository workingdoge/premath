#!/usr/bin/env python3
"""Deterministic governance promotion-gate client over core CLI surfaces."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Tuple

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


GOVERNANCE_PROFILE_CLAIM_ID = "profile.doctrine_inf_governance.v0"


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _is_truthy_env(var_name: str) -> bool:
    raw = os.environ.get(var_name, "").strip().lower()
    return raw in {"1", "true", "yes", "on"}


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("governance gate payload must be an object")
    failure_classes = payload.get("failureClasses")
    if not isinstance(failure_classes, list):
        raise ValueError("governance gate payload failureClasses must be a list")
    for idx, item in enumerate(failure_classes):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(
                f"governance gate payload failureClasses[{idx}] must be a non-empty string"
            )
    return payload


def governance_failure_classes(repo_root: Path) -> Tuple[str, ...]:
    """Return deterministic governance-gate failure classes for CI pipeline routing.

    Gate semantics are claim-gated by `profile.doctrine_inf_governance.v0` from
    CAPABILITY-REGISTRY. If promotion intent is not asserted, returns empty.

    Environment controls:
    - PREMATH_GOVERNANCE_PROMOTION_REQUIRED=1|true|yes|on: fail closed with
      `governance.eval_lineage_missing` when evidence is absent/invalid.
    - PREMATH_GOVERNANCE_PROMOTION_EVIDENCE=<path>: override evidence location.
    """

    evidence_path = os.environ.get("PREMATH_GOVERNANCE_PROMOTION_EVIDENCE", "").strip()
    request_payload: Dict[str, Any] = {
        "repoRoot": str(repo_root),
        "promotionRequired": _is_truthy_env("PREMATH_GOVERNANCE_PROMOTION_REQUIRED"),
    }
    if evidence_path:
        request_payload["promotionEvidencePath"] = evidence_path

    try:
        payload = run_core_json_command(
            repo_root,
            subcommand="governance-promotion-check",
            input_flag="--input",
            request_payload=request_payload,
            validate_payload=_validate_payload,
            default_failure_class="governance.eval_lineage_missing",
            default_failure_message="governance gate command failed",
            invalid_json_message="governance gate command returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError:
        return ("governance.eval_lineage_missing",)

    failure_classes = payload.get("failureClasses", [])
    if not isinstance(failure_classes, list):
        return ("governance.eval_lineage_missing",)

    out: list[str] = []
    seen: set[str] = set()
    for item in failure_classes:
        if not isinstance(item, str):
            continue
        trimmed = item.strip()
        if not trimmed or trimmed in seen:
            continue
        seen.add(trimmed)
        out.append(trimmed)
    return tuple(out)
