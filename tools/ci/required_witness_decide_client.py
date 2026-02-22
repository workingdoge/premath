#!/usr/bin/env python3
"""Shared client for core `premath required-witness-decide` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class RequiredWitnessDecideError(ValueError):
    """Required-witness-decide failure with deterministic failure class."""

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
    return "required_witness_decide_invalid: required-witness-decide failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise RequiredWitnessDecideError(
            "required_witness_decide_invalid",
            "required-witness-decide payload must be an object",
        )
    if payload.get("decisionKind") != "ci.required.decision.v1":
        raise RequiredWitnessDecideError(
            "required_witness_decide_invalid",
            "required-witness-decide payload decisionKind must be ci.required.decision.v1",
        )
    decision = payload.get("decision")
    if decision not in {"accept", "reject"}:
        raise RequiredWitnessDecideError(
            "required_witness_decide_invalid",
            "required-witness-decide payload decision must be accept|reject",
        )
    reason_class = payload.get("reasonClass")
    if not isinstance(reason_class, str) or not reason_class.strip():
        raise RequiredWitnessDecideError(
            "required_witness_decide_invalid",
            "required-witness-decide payload reasonClass must be a non-empty string",
        )
    if not isinstance(payload.get("errors"), list):
        raise RequiredWitnessDecideError(
            "required_witness_decide_invalid",
            "required-witness-decide payload errors must be a list",
        )
    return payload


def run_required_witness_decide(root: Path, decide_input: Dict[str, Any]) -> Dict[str, Any]:
    def run_cmd(cli_prefix: List[str], input_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            "required-witness-decide",
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
        json.dump(decide_input, input_file, indent=2, ensure_ascii=False)
        input_file.write("\n")
        input_path = Path(input_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        completed = run_cmd(cli_prefix, input_path)

        # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'required-witness-decide'" in stderr:
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise RequiredWitnessDecideError(failure_class, reason)
            raise RequiredWitnessDecideError("required_witness_decide_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RequiredWitnessDecideError(
                "required_witness_decide_invalid",
                "required-witness-decide returned invalid JSON",
            ) from exc

        try:
            return _validate_payload(payload)
        except RequiredWitnessDecideError as exc:
            # If a stale local binary emits an older payload shape, retry through cargo.
            if cli_prefix and Path(cli_prefix[0]).name == "premath":
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)
                if completed.returncode != 0:
                    message = _extract_failure_message(completed)
                    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
                    if match:
                        failure_class = match.group("class")
                        reason = match.group("reason").strip() or message
                        raise RequiredWitnessDecideError(failure_class, reason) from exc
                    raise RequiredWitnessDecideError("required_witness_decide_invalid", message) from exc
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise RequiredWitnessDecideError(
                        "required_witness_decide_invalid",
                        "required-witness-decide returned invalid JSON",
                    ) from json_exc
                return _validate_payload(payload)
            raise
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
