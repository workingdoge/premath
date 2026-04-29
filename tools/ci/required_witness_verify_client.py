#!/usr/bin/env python3
"""Shared client for core `premath required-witness-verify` execution."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
)


class RequiredWitnessVerifyError(ValueError):
    """Required-witness-verify failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("required-witness-verify payload must be an object")
    errors = payload.get("errors")
    derived = payload.get("derived")
    if not isinstance(errors, list):
        raise ValueError("required-witness-verify payload errors must be a list")
    if not isinstance(derived, dict):
        raise ValueError("required-witness-verify payload derived must be an object")
    return payload


def run_required_witness_verify(root: Path, verify_input: Dict[str, Any]) -> Dict[str, Any]:
    try:
        return run_core_json_command(
            root,
            subcommand="required-witness-verify",
            input_flag="--input",
            request_payload=verify_input,
            validate_payload=_validate_payload,
            default_failure_class="required_witness_verify_invalid",
            default_failure_message="required_witness_verify_invalid: required-witness-verify failed",
            invalid_json_message="required-witness-verify returned invalid JSON",
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise RequiredWitnessVerifyError(exc.failure_class, exc.reason) from exc


def _normalize_rel_path(path: str) -> str:
    normalized = path.strip().replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def _load_gate_witness_payloads_from_fs(
    witness: Dict[str, Any],
    witness_root: Optional[Path],
) -> Optional[Dict[str, Any]]:
    refs = witness.get("gateWitnessRefs")
    if not isinstance(refs, list) or witness_root is None:
        return None

    payloads: Dict[str, Any] = {}
    for ref in refs:
        if not isinstance(ref, dict):
            continue
        artifact_rel_path_raw = ref.get("artifactRelPath")
        if not isinstance(artifact_rel_path_raw, str) or not artifact_rel_path_raw.strip():
            continue
        artifact_rel_path = _normalize_rel_path(artifact_rel_path_raw)
        if (
            artifact_rel_path.startswith("/")
            or artifact_rel_path.startswith("../")
            or "/../" in artifact_rel_path
            or artifact_rel_path == ".."
        ):
            continue
        target = witness_root / artifact_rel_path
        try:
            payload = json.loads(target.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        payloads[artifact_rel_path] = payload
    return payloads if payloads else None


def verify_required_witness_payload(
    witness: Dict[str, Any],
    changed_paths: Sequence[str],
    witness_root: Optional[Path] = None,
    gate_witness_payloads: Optional[Mapping[str, Dict[str, Any]]] = None,
    native_required_checks: Optional[Sequence[str]] = None,
) -> Tuple[List[str], Dict[str, Any]]:
    """Verify a ci.required witness against deterministic projection semantics."""

    root = Path(__file__).resolve().parents[2]
    request: Dict[str, Any] = {
        "witness": witness,
        "changedPaths": list(changed_paths),
        "nativeRequiredChecks": list(native_required_checks or []),
    }

    if gate_witness_payloads is not None:
        request["gateWitnessPayloads"] = dict(gate_witness_payloads)
    else:
        loaded = _load_gate_witness_payloads_from_fs(witness, witness_root)
        if loaded is not None:
            request["gateWitnessPayloads"] = loaded

    if witness_root is not None:
        request["witnessRoot"] = str(witness_root)

    try:
        result = run_required_witness_verify(root, request)
    except RequiredWitnessVerifyError as exc:
        return [f"{exc.failure_class}: {exc.reason}"], {}

    errors_raw = result.get("errors")
    derived_raw = result.get("derived")
    errors: List[str] = []
    if isinstance(errors_raw, list):
        for item in errors_raw:
            if isinstance(item, str) and item.strip():
                errors.append(item.strip())
    derived = derived_raw if isinstance(derived_raw, dict) else {}
    return errors, derived
