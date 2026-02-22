#!/usr/bin/env python3
"""Shared client for core `premath required-gate-ref` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class RequiredGateRefError(ValueError):
    """Required-gate-ref failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    premath_bin = root / "target" / "debug" / "premath"
    if premath_bin.exists() and os.access(premath_bin, os.X_OK):
        return [str(premath_bin)]
    return ["cargo", "run", "--package", "premath-cli", "--"]


def _extract_failure_message(completed: subprocess.CompletedProcess[str]) -> str:
    stderr_lines = [line.strip() for line in completed.stderr.splitlines() if line.strip()]
    stdout_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]

    for line in reversed(stderr_lines):
        if re.match(r"^[a-z0-9_]+:\s+.+$", line):
            return line
    if stderr_lines:
        return stderr_lines[-1]
    if stdout_lines:
        return stdout_lines[-1]
    return "required_gate_ref_invalid: required-gate-ref failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise RequiredGateRefError(
            "required_gate_ref_invalid",
            "required-gate-ref payload must be an object",
        )
    ref = payload.get("gateWitnessRef")
    if not isinstance(ref, dict):
        raise RequiredGateRefError(
            "required_gate_ref_invalid",
            "required-gate-ref payload gateWitnessRef must be an object",
        )
    for key in ("checkId", "artifactRelPath", "sha256", "source"):
        value = ref.get(key)
        if not isinstance(value, str) or not value.strip():
            raise RequiredGateRefError(
                "required_gate_ref_invalid",
                f"required-gate-ref payload gateWitnessRef.{key} must be a non-empty string",
            )
    if not isinstance(ref.get("failureClasses"), list):
        raise RequiredGateRefError(
            "required_gate_ref_invalid",
            "required-gate-ref payload gateWitnessRef.failureClasses must be a list",
        )
    gate_payload = payload.get("gatePayload")
    if gate_payload is not None and not isinstance(gate_payload, dict):
        raise RequiredGateRefError(
            "required_gate_ref_invalid",
            "required-gate-ref payload gatePayload must be object|null",
        )
    return payload


def run_required_gate_ref(root: Path, request_input: Dict[str, Any]) -> Dict[str, Any]:
    def run_cmd(cli_prefix: List[str], input_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            "required-gate-ref",
            "--input",
            str(input_path),
            "--json",
        ]
        return subprocess.run(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
        )

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8"
    ) as input_file:
        json.dump(request_input, input_file, indent=2, ensure_ascii=False)
        input_file.write("\n")
        input_path = Path(input_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        completed = run_cmd(cli_prefix, input_path)

        # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'required-gate-ref'" in stderr:
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise RequiredGateRefError(failure_class, reason)
            raise RequiredGateRefError("required_gate_ref_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RequiredGateRefError(
                "required_gate_ref_invalid",
                "required-gate-ref returned invalid JSON",
            ) from exc

        try:
            return _validate_payload(payload)
        except RequiredGateRefError as exc:
            # If a stale local binary emits an older payload shape, retry through cargo.
            if cli_prefix and Path(cli_prefix[0]).name == "premath":
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)
                if completed.returncode != 0:
                    message = _extract_failure_message(completed)
                    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
                    if match:
                        failure_class = match.group("class")
                        reason = match.group("reason").strip() or message
                        raise RequiredGateRefError(failure_class, reason) from exc
                    raise RequiredGateRefError("required_gate_ref_invalid", message) from exc
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise RequiredGateRefError(
                        "required_gate_ref_invalid",
                        "required-gate-ref returned invalid JSON",
                    ) from json_exc
                return _validate_payload(payload)
            raise
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
