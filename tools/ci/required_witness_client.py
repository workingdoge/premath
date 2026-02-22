#!/usr/bin/env python3
"""Shared client for core `premath required-witness` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class RequiredWitnessError(ValueError):
    """Required-witness failure with deterministic failure class."""

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
    return "required_witness_runtime_invalid: required-witness failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise RequiredWitnessError(
            "required_witness_runtime_invalid",
            "required-witness payload must be an object",
        )

    if payload.get("ciSchema") != 1:
        raise RequiredWitnessError(
            "required_witness_runtime_invalid",
            "required-witness payload ciSchema must be 1",
        )
    if payload.get("witnessKind") != "ci.required.v1":
        raise RequiredWitnessError(
            "required_witness_runtime_invalid",
            "required-witness payload witnessKind must be ci.required.v1",
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
            raise RequiredWitnessError(
                "required_witness_runtime_invalid",
                f"required-witness payload missing {key}",
            )

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
            raise RequiredWitnessError(
                "required_witness_runtime_invalid",
                f"required-witness payload {key} must be a list",
            )

    if not isinstance(payload.get("docsOnly"), bool):
        raise RequiredWitnessError(
            "required_witness_runtime_invalid",
            "required-witness payload docsOnly must be a boolean",
        )
    run_duration_ms = payload.get("runDurationMs")
    if not isinstance(run_duration_ms, int) or run_duration_ms < 0:
        raise RequiredWitnessError(
            "required_witness_runtime_invalid",
            "required-witness payload runDurationMs must be a non-negative integer",
        )

    return payload


def run_required_witness(root: Path, runtime_payload: Dict[str, Any]) -> Dict[str, Any]:
    def run_cmd(cli_prefix: List[str], runtime_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            "required-witness",
            "--runtime",
            str(runtime_path),
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
    ) as runtime_file:
        json.dump(runtime_payload, runtime_file, indent=2, ensure_ascii=False)
        runtime_file.write("\n")
        runtime_path = Path(runtime_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        completed = run_cmd(cli_prefix, runtime_path)

        # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'required-witness'" in stderr:
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], runtime_path)

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise RequiredWitnessError(failure_class, reason)
            raise RequiredWitnessError("required_witness_runtime_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RequiredWitnessError(
                "required_witness_runtime_invalid",
                "required-witness returned invalid JSON",
            ) from exc

        try:
            return _validate_payload(payload)
        except RequiredWitnessError as exc:
            # If a stale local binary emits an older payload shape, retry through cargo.
            if cli_prefix and Path(cli_prefix[0]).name == "premath":
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], runtime_path)
                if completed.returncode != 0:
                    message = _extract_failure_message(completed)
                    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
                    if match:
                        failure_class = match.group("class")
                        reason = match.group("reason").strip() or message
                        raise RequiredWitnessError(failure_class, reason) from exc
                    raise RequiredWitnessError("required_witness_runtime_invalid", message) from exc
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise RequiredWitnessError(
                        "required_witness_runtime_invalid",
                        "required-witness returned invalid JSON",
                    ) from json_exc
                return _validate_payload(payload)
            raise
    finally:
        try:
            runtime_path.unlink()
        except FileNotFoundError:
            pass
