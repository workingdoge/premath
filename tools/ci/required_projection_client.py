#!/usr/bin/env python3
"""Shared client for core `premath required-projection` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class RequiredProjectionError(ValueError):
    """Required-projection failure with deterministic failure class."""

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
    return "required_projection_invalid: required-projection failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise RequiredProjectionError(
            "required_projection_invalid",
            "required-projection payload must be an object",
        )

    if payload.get("schema") != 1:
        raise RequiredProjectionError(
            "required_projection_invalid",
            "required-projection payload schema must be 1",
        )

    for key in ("projectionPolicy", "projectionDigest"):
        value = payload.get(key)
        if not isinstance(value, str) or not value.strip():
            raise RequiredProjectionError(
                "required_projection_invalid",
                f"required-projection payload missing {key}",
            )

    for key in ("changedPaths", "requiredChecks", "reasons"):
        if not isinstance(payload.get(key), list):
            raise RequiredProjectionError(
                "required_projection_invalid",
                f"required-projection payload {key} must be a list",
            )

    if not isinstance(payload.get("docsOnly"), bool):
        raise RequiredProjectionError(
            "required_projection_invalid",
            "required-projection payload docsOnly must be a boolean",
        )
    return payload


def run_required_projection(root: Path, projection_input: Dict[str, Any]) -> Dict[str, Any]:
    def run_cmd(cli_prefix: List[str], input_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            "required-projection",
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
        json.dump(projection_input, input_file, indent=2, ensure_ascii=False)
        input_file.write("\n")
        input_path = Path(input_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        completed = run_cmd(cli_prefix, input_path)

        # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'required-projection'" in stderr:
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise RequiredProjectionError(failure_class, reason)
            raise RequiredProjectionError("required_projection_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RequiredProjectionError(
                "required_projection_invalid",
                "required-projection returned invalid JSON",
            ) from exc

        try:
            return _validate_payload(payload)
        except RequiredProjectionError as exc:
            # If a stale local binary emits an older payload shape, retry through cargo.
            if cli_prefix and Path(cli_prefix[0]).name == "premath":
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)
                if completed.returncode != 0:
                    message = _extract_failure_message(completed)
                    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
                    if match:
                        failure_class = match.group("class")
                        reason = match.group("reason").strip() or message
                        raise RequiredProjectionError(failure_class, reason) from exc
                    raise RequiredProjectionError("required_projection_invalid", message) from exc
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise RequiredProjectionError(
                        "required_projection_invalid",
                        "required-projection returned invalid JSON",
                    ) from json_exc
                return _validate_payload(payload)
            raise
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
