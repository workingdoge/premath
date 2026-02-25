#!/usr/bin/env python3
"""Shared core-command client helpers for conformance runners."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
from typing import Any, Dict, List, Tuple

ROOT = Path(__file__).resolve().parents[2]

WORLD_REGISTRY_CHECK_COMMAND_PREFIX: Tuple[str, ...] = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "world-registry-check",
)
RUNTIME_ORCHESTRATION_CHECK_COMMAND_PREFIX: Tuple[str, ...] = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "runtime-orchestration-check",
)


def _validate_command_prefix(cmd: List[str], prefix: Tuple[str, ...], label: str) -> None:
    if tuple(cmd[: len(prefix)]) != prefix:
        raise ValueError(
            f"{label} command surface drift: expected prefix {list(prefix)!r}, got {cmd!r}"
        )


def validate_world_registry_check_command(cmd: List[str]) -> None:
    _validate_command_prefix(cmd, WORLD_REGISTRY_CHECK_COMMAND_PREFIX, "world-registry-check")


def validate_runtime_orchestration_check_command(cmd: List[str]) -> None:
    _validate_command_prefix(
        cmd,
        RUNTIME_ORCHESTRATION_CHECK_COMMAND_PREFIX,
        "runtime-orchestration-check",
    )


def resolve_world_registry_check_command() -> List[str]:
    override = os.environ.get("PREMATH_WORLD_REGISTRY_CHECK_CMD", "").strip()
    if override:
        command = shlex.split(override)
    else:
        command = list(WORLD_REGISTRY_CHECK_COMMAND_PREFIX)
    validate_world_registry_check_command(command)
    return command


def resolve_runtime_orchestration_check_command() -> List[str]:
    override = os.environ.get("PREMATH_RUNTIME_ORCHESTRATION_CHECK_CMD", "").strip()
    if override:
        command = shlex.split(override)
    else:
        command = list(RUNTIME_ORCHESTRATION_CHECK_COMMAND_PREFIX)
    validate_runtime_orchestration_check_command(command)
    return command


def _run_checked_json_command(cmd: List[str], *, allowed_exit_codes: Tuple[int, ...], label: str) -> Dict[str, Any]:
    completed = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode not in set(allowed_exit_codes):
        raise ValueError(
            f"kernel {label} command failed: "
            f"exit={completed.returncode}, stderr={completed.stderr.strip()!r}"
        )
    stdout = completed.stdout.strip()
    if not stdout:
        raise ValueError(f"kernel {label} produced empty stdout")
    payload = json.loads(stdout)
    if not isinstance(payload, dict):
        raise ValueError(f"kernel {label} payload must be an object")
    return payload


def run_world_registry_check(
    *,
    doctrine_site_input: Dict[str, Any],
    doctrine_operation_registry: Dict[str, Any],
    control_plane_contract: Dict[str, Any] | None = None,
    required_route_families: Tuple[str, ...] | None = None,
    required_route_bindings: Dict[str, List[str]] | None = None,
) -> Dict[str, Any]:
    command = resolve_world_registry_check_command()
    with tempfile.TemporaryDirectory(prefix="premath-world-registry-check-") as tmp:
        tmp_root = Path(tmp)
        site_input_path = tmp_root / "doctrine_site_input.json"
        operations_path = tmp_root / "doctrine_operation_registry.json"
        site_input_path.write_text(
            json.dumps(doctrine_site_input, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        operations_path.write_text(
            json.dumps(doctrine_operation_registry, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        cmd = [
            *command,
            "--site-input",
            str(site_input_path),
            "--operations",
            str(operations_path),
            "--json",
        ]
        if control_plane_contract is not None:
            control_plane_contract_path = tmp_root / "control_plane_contract.json"
            control_plane_contract_path.write_text(
                json.dumps(control_plane_contract, indent=2, sort_keys=True),
                encoding="utf-8",
            )
            cmd.extend(
                [
                    "--control-plane-contract",
                    str(control_plane_contract_path),
                ]
            )
        if required_route_families:
            for family in required_route_families:
                cmd.extend(["--required-route-family", family])
        if required_route_bindings:
            for family in sorted(required_route_bindings):
                for operation_id in sorted(set(required_route_bindings[family])):
                    cmd.extend(
                        [
                            "--required-route-binding",
                            f"{family}={operation_id}",
                        ]
                    )
        return _run_checked_json_command(
            cmd,
            allowed_exit_codes=(0,),
            label="world-registry-check",
        )


def run_runtime_orchestration_check(
    *,
    control_plane_contract: Dict[str, Any],
    operation_registry: Dict[str, Any],
    harness_runtime_text: str,
    doctrine_site_input: Dict[str, Any] | None = None,
) -> Dict[str, Any]:
    command = resolve_runtime_orchestration_check_command()
    with tempfile.TemporaryDirectory(prefix="premath-runtime-orchestration-check-") as tmp:
        tmp_root = Path(tmp)
        control_plane_path = tmp_root / "control_plane_contract.json"
        op_registry_path = tmp_root / "doctrine_operation_registry.json"
        harness_runtime_path = tmp_root / "harness_runtime.md"
        control_plane_path.write_text(
            json.dumps(control_plane_contract, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        op_registry_path.write_text(
            json.dumps(operation_registry, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        harness_runtime_path.write_text(harness_runtime_text, encoding="utf-8")

        cmd = [
            *command,
            "--control-plane-contract",
            str(control_plane_path),
            "--doctrine-op-registry",
            str(op_registry_path),
            "--harness-runtime",
            str(harness_runtime_path),
            "--json",
        ]
        if doctrine_site_input is not None:
            doctrine_site_input_path = tmp_root / "doctrine_site_input.json"
            doctrine_site_input_path.write_text(
                json.dumps(doctrine_site_input, indent=2, sort_keys=True),
                encoding="utf-8",
            )
            cmd.extend(["--doctrine-site-input", str(doctrine_site_input_path)])
        return _run_checked_json_command(
            cmd,
            allowed_exit_codes=(0, 1),
            label="runtime-orchestration-check",
        )
