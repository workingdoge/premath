#!/usr/bin/env python3
"""Shared client for core `premath required-delta` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class RequiredDeltaError(ValueError):
    """Required-delta failure with deterministic failure class."""

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
    return "required_delta_invalid: required-delta failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise RequiredDeltaError(
            "required_delta_invalid",
            "required-delta payload must be an object",
        )

    if payload.get("schema") != 1:
        raise RequiredDeltaError(
            "required_delta_invalid",
            "required-delta payload schema must be 1",
        )

    for key in ("deltaKind", "source", "toRef"):
        value = payload.get(key)
        if not isinstance(value, str) or not value.strip():
            raise RequiredDeltaError(
                "required_delta_invalid",
                f"required-delta payload missing {key}",
            )

    changed_paths = payload.get("changedPaths")
    if not isinstance(changed_paths, list):
        raise RequiredDeltaError(
            "required_delta_invalid",
            "required-delta payload changedPaths must be a list",
        )

    from_ref = payload.get("fromRef")
    if from_ref is not None and not isinstance(from_ref, str):
        raise RequiredDeltaError(
            "required_delta_invalid",
            "required-delta payload fromRef must be string|null",
        )
    return payload


def run_required_delta(root: Path, delta_input: Dict[str, Any]) -> Dict[str, Any]:
    def run_cmd(cli_prefix: List[str], input_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            "required-delta",
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
        json.dump(delta_input, input_file, indent=2, ensure_ascii=False)
        input_file.write("\n")
        input_path = Path(input_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        completed = run_cmd(cli_prefix, input_path)

        # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'required-delta'" in stderr:
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise RequiredDeltaError(failure_class, reason)
            raise RequiredDeltaError("required_delta_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise RequiredDeltaError(
                "required_delta_invalid",
                "required-delta returned invalid JSON",
            ) from exc

        try:
            return _validate_payload(payload)
        except RequiredDeltaError as exc:
            # If a stale local binary emits an older payload shape, retry through cargo.
            if cli_prefix and Path(cli_prefix[0]).name == "premath":
                completed = run_cmd(["cargo", "run", "--package", "premath-cli", "--"], input_path)
                if completed.returncode != 0:
                    message = _extract_failure_message(completed)
                    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
                    if match:
                        failure_class = match.group("class")
                        reason = match.group("reason").strip() or message
                        raise RequiredDeltaError(failure_class, reason) from exc
                    raise RequiredDeltaError("required_delta_invalid", message) from exc
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise RequiredDeltaError(
                        "required_delta_invalid",
                        "required-delta returned invalid JSON",
                    ) from json_exc
                return _validate_payload(payload)
            raise
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
