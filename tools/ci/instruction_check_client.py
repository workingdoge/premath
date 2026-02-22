#!/usr/bin/env python3
"""Shared client for core `premath instruction-check` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class InstructionCheckError(ValueError):
    """Instruction-check failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


class InstructionWitnessError(ValueError):
    """Instruction-witness failure with deterministic failure class."""

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
    return "instruction_envelope_invalid: instruction-check failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload must be an object",
        )
    required_string_fields = ("intent", "normalizerId", "policyDigest")
    for key in required_string_fields:
        if not isinstance(payload.get(key), str) or not payload.get(key):
            raise InstructionCheckError(
                "instruction_envelope_invalid_shape",
                f"instruction-check payload missing {key}",
            )
    if "scope" not in payload:
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload missing scope",
        )
    requested_checks = payload.get("requestedChecks")
    if not isinstance(requested_checks, list):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload requestedChecks must be a list",
        )
    classification = payload.get("instructionClassification")
    if not isinstance(classification, dict):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload instructionClassification must be an object",
        )
    state = classification.get("state")
    if state == "typed":
        if not isinstance(classification.get("kind"), str) or not classification.get("kind"):
            raise InstructionCheckError(
                "instruction_envelope_invalid_shape",
                "instruction-check payload instructionClassification.kind must be a non-empty string",
            )
    elif state == "unknown":
        if not isinstance(classification.get("reason"), str) or not classification.get("reason"):
            raise InstructionCheckError(
                "instruction_envelope_invalid_shape",
                "instruction-check payload instructionClassification.reason must be a non-empty string",
            )
    else:
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload instructionClassification.state must be typed|unknown",
        )
    execution_decision = payload.get("executionDecision")
    if not isinstance(execution_decision, dict):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload executionDecision must be an object",
        )
    decision_state = execution_decision.get("state")
    if decision_state == "execute":
        pass
    elif decision_state == "reject":
        if not isinstance(execution_decision.get("source"), str) or not execution_decision.get(
            "source"
        ):
            raise InstructionCheckError(
                "instruction_envelope_invalid_shape",
                "instruction-check payload executionDecision.source must be a non-empty string",
            )
        if not isinstance(execution_decision.get("reason"), str) or not execution_decision.get(
            "reason"
        ):
            raise InstructionCheckError(
                "instruction_envelope_invalid_shape",
                "instruction-check payload executionDecision.reason must be a non-empty string",
            )
        for key in ("operationalFailureClasses", "semanticFailureClasses"):
            values = execution_decision.get(key)
            if not isinstance(values, list) or not all(
                isinstance(item, str) and item for item in values
            ):
                raise InstructionCheckError(
                    "instruction_envelope_invalid_shape",
                    f"instruction-check payload executionDecision.{key} must be a list of non-empty strings",
                )
    else:
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload executionDecision.state must be execute|reject",
        )
    typing_policy = payload.get("typingPolicy")
    if not isinstance(typing_policy, dict):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload typingPolicy must be an object",
        )
    capability_claims = payload.get("capabilityClaims")
    if not isinstance(capability_claims, list):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload capabilityClaims must be a list",
        )
    proposal = payload.get("proposal")
    if proposal is not None and not isinstance(proposal, dict):
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check payload proposal must be an object when provided",
        )
    return payload


def run_instruction_check(root: Path, instruction_path: Path) -> Dict[str, Any]:
    cli_prefix = resolve_premath_cli(root)
    cmd = [
        *cli_prefix,
        "instruction-check",
        "--instruction",
        str(instruction_path),
        "--repo-root",
        str(root),
        "--json",
    ]
    completed = subprocess.run(
        cmd,
        cwd=root,
        capture_output=True,
        text=True,
    )

    # If a stale local `target/debug/premath` lacks this subcommand, retry through cargo.
    if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
        stderr = completed.stderr + "\n" + completed.stdout
        if "unrecognized subcommand 'instruction-check'" in stderr:
            cmd = [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "instruction-check",
                "--instruction",
                str(instruction_path),
                "--repo-root",
                str(root),
                "--json",
            ]
            completed = subprocess.run(
                cmd,
                cwd=root,
                capture_output=True,
                text=True,
            )

    if completed.returncode != 0:
        message = _extract_failure_message(completed)
        match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
        if match:
            failure_class = match.group("class")
            reason = match.group("reason").strip() or message
            raise InstructionCheckError(failure_class, reason)
        raise InstructionCheckError("instruction_envelope_invalid", message)

    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise InstructionCheckError(
            "instruction_envelope_invalid_shape",
            "instruction-check returned invalid JSON",
        ) from exc
    return _validate_payload(payload)


def _validate_witness_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "instruction-witness payload must be an object",
        )
    required_string_fields = (
        "instructionId",
        "instructionRef",
        "instructionDigest",
        "witnessKind",
        "verdictClass",
        "normalizerId",
        "policyDigest",
    )
    for key in required_string_fields:
        if not isinstance(payload.get(key), str) or not payload.get(key):
            raise InstructionWitnessError(
                "instruction_runtime_invalid",
                f"instruction-witness payload missing {key}",
            )
    if not isinstance(payload.get("results"), list):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "instruction-witness payload results must be a list",
        )
    if not isinstance(payload.get("failureClasses"), list):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "instruction-witness payload failureClasses must be a list",
        )
    if not isinstance(payload.get("operationalFailureClasses"), list):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "instruction-witness payload operationalFailureClasses must be a list",
        )
    if not isinstance(payload.get("semanticFailureClasses"), list):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "instruction-witness payload semanticFailureClasses must be a list",
        )
    return payload


def run_instruction_witness(
    root: Path, instruction_path: Path, runtime_payload: Dict[str, Any]
) -> Dict[str, Any]:
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False, encoding="utf-8"
    ) as runtime_file:
        json.dump(runtime_payload, runtime_file, indent=2, ensure_ascii=False)
        runtime_file.write("\n")
        runtime_path = Path(runtime_file.name)

    try:
        cli_prefix = resolve_premath_cli(root)
        cmd = [
            *cli_prefix,
            "instruction-witness",
            "--instruction",
            str(instruction_path),
            "--runtime",
            str(runtime_path),
            "--repo-root",
            str(root),
            "--json",
        ]
        completed = subprocess.run(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
        )

        # If a stale local binary lacks this subcommand, retry through cargo.
        if completed.returncode != 0 and cli_prefix and Path(cli_prefix[0]).name == "premath":
            stderr = completed.stderr + "\n" + completed.stdout
            if "unrecognized subcommand 'instruction-witness'" in stderr:
                cmd = [
                    "cargo",
                    "run",
                    "--package",
                    "premath-cli",
                    "--",
                    "instruction-witness",
                    "--instruction",
                    str(instruction_path),
                    "--runtime",
                    str(runtime_path),
                    "--repo-root",
                    str(root),
                    "--json",
                ]
                completed = subprocess.run(
                    cmd,
                    cwd=root,
                    capture_output=True,
                    text=True,
                )

        if completed.returncode != 0:
            message = _extract_failure_message(completed)
            match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
            if match:
                failure_class = match.group("class")
                reason = match.group("reason").strip() or message
                raise InstructionWitnessError(failure_class, reason)
            raise InstructionWitnessError("instruction_runtime_invalid", message)

        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise InstructionWitnessError(
                "instruction_runtime_invalid",
                "instruction-witness returned invalid JSON",
            ) from exc
        return _validate_witness_payload(payload)
    finally:
        try:
            runtime_path.unlink()
        except FileNotFoundError:
            pass
