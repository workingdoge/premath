#!/usr/bin/env python3
"""Shared transport helper for core `premath <subcommand>` JSON clients."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Callable, Dict, List


class CoreCliClientError(ValueError):
    """Core command execution failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    premath_bin = root / "target" / "debug" / "premath"
    if premath_bin.exists() and os.access(premath_bin, os.X_OK):
        return [str(premath_bin)]
    return ["cargo", "run", "--package", "premath-cli", "--"]


def _is_local_premath_binary(cli_prefix: List[str]) -> bool:
    return bool(cli_prefix) and Path(cli_prefix[0]).name == "premath"


def _extract_failure_message(
    completed: subprocess.CompletedProcess[str],
    default_message: str,
) -> str:
    stderr_lines = [line.strip() for line in completed.stderr.splitlines() if line.strip()]
    stdout_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]

    for line in reversed(stderr_lines):
        if re.match(r"^[a-z0-9_]+:\s+.+$", line):
            return line
    if stderr_lines:
        return stderr_lines[-1]
    if stdout_lines:
        return stdout_lines[-1]
    return default_message


def _raise_from_completed(
    completed: subprocess.CompletedProcess[str],
    *,
    default_failure_class: str,
    default_failure_message: str,
) -> None:
    message = _extract_failure_message(completed, default_failure_message)
    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
    if match:
        failure_class = match.group("class")
        reason = match.group("reason").strip() or message
        raise CoreCliClientError(failure_class, reason)
    raise CoreCliClientError(default_failure_class, message)


def run_core_json_command(
    root: Path,
    *,
    subcommand: str,
    input_flag: str,
    request_payload: Dict[str, Any],
    validate_payload: Callable[[Any], Dict[str, Any]],
    default_failure_class: str,
    default_failure_message: str,
    invalid_json_message: str,
    resolve_cli: Callable[[Path], List[str]] = resolve_premath_cli,
    run_process: Callable[..., subprocess.CompletedProcess[str]] = subprocess.run,
) -> Dict[str, Any]:
    """Run one core command with JSON input/output and deterministic retries."""

    def run_cmd(cli_prefix: List[str], input_path: Path) -> subprocess.CompletedProcess[str]:
        cmd = [
            *cli_prefix,
            subcommand,
            input_flag,
            str(input_path),
            "--json",
        ]
        return run_process(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
        )

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8"
    ) as input_file:
        json.dump(request_payload, input_file, indent=2, ensure_ascii=False)
        input_file.write("\n")
        input_path = Path(input_file.name)

    try:
        cli_prefix = resolve_cli(root)
        completed = run_cmd(cli_prefix, input_path)

        if completed.returncode != 0 and _is_local_premath_binary(cli_prefix):
            stderr = completed.stderr + "\n" + completed.stdout
            if f"unrecognized subcommand '{subcommand}'" in stderr:
                completed = run_cmd(
                    ["cargo", "run", "--package", "premath-cli", "--"],
                    input_path,
                )

        if completed.returncode != 0:
            _raise_from_completed(
                completed,
                default_failure_class=default_failure_class,
                default_failure_message=default_failure_message,
            )

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise CoreCliClientError(default_failure_class, invalid_json_message) from exc

        try:
            return validate_payload(payload)
        except ValueError as exc:
            if _is_local_premath_binary(cli_prefix):
                completed = run_cmd(
                    ["cargo", "run", "--package", "premath-cli", "--"],
                    input_path,
                )
                if completed.returncode != 0:
                    _raise_from_completed(
                        completed,
                        default_failure_class=default_failure_class,
                        default_failure_message=default_failure_message,
                    )
                try:
                    payload = json.loads(completed.stdout)
                except json.JSONDecodeError as json_exc:
                    raise CoreCliClientError(default_failure_class, invalid_json_message) from json_exc
                return validate_payload(payload)
            raise CoreCliClientError(default_failure_class, str(exc)) from exc
    finally:
        try:
            input_path.unlink()
        except FileNotFoundError:
            pass
