#!/usr/bin/env python3
"""Shared client for core `premath instruction-check` execution."""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Any, Dict, List

from core_cli_client import (
    CoreCliClientError,
    resolve_premath_cli as _resolve_premath_cli,
    run_core_json_command,
    run_core_json_command_from_path,
)
from control_plane_contract import resolve_schema_kind


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
    return _resolve_premath_cli(root)


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("instruction-check payload must be an object")
    required_string_fields = ("intent", "normalizerId", "policyDigest", "instructionDigest")
    for key in required_string_fields:
        if not isinstance(payload.get(key), str) or not payload.get(key):
            raise ValueError(f"instruction-check payload missing {key}")
    if "scope" not in payload:
        raise ValueError("instruction-check payload missing scope")
    requested_checks = payload.get("requestedChecks")
    if not isinstance(requested_checks, list):
        raise ValueError("instruction-check payload requestedChecks must be a list")
    classification = payload.get("instructionClassification")
    if not isinstance(classification, dict):
        raise ValueError("instruction-check payload instructionClassification must be an object")
    state = classification.get("state")
    if state == "typed":
        if not isinstance(classification.get("kind"), str) or not classification.get("kind"):
            raise ValueError(
                "instruction-check payload instructionClassification.kind must be a non-empty string"
            )
    elif state == "unknown":
        if not isinstance(classification.get("reason"), str) or not classification.get("reason"):
            raise ValueError(
                "instruction-check payload instructionClassification.reason must be a non-empty string"
            )
    else:
        raise ValueError("instruction-check payload instructionClassification.state must be typed|unknown")
    execution_decision = payload.get("executionDecision")
    if not isinstance(execution_decision, dict):
        raise ValueError("instruction-check payload executionDecision must be an object")
    decision_state = execution_decision.get("state")
    if decision_state == "execute":
        pass
    elif decision_state == "reject":
        if not isinstance(execution_decision.get("source"), str) or not execution_decision.get("source"):
            raise ValueError(
                "instruction-check payload executionDecision.source must be a non-empty string"
            )
        if not isinstance(execution_decision.get("reason"), str) or not execution_decision.get("reason"):
            raise ValueError(
                "instruction-check payload executionDecision.reason must be a non-empty string"
            )
        for key in ("operationalFailureClasses", "semanticFailureClasses"):
            values = execution_decision.get(key)
            if not isinstance(values, list) or not all(
                isinstance(item, str) and item for item in values
            ):
                raise ValueError(
                    f"instruction-check payload executionDecision.{key} must be a list of non-empty strings"
                )
    else:
        raise ValueError("instruction-check payload executionDecision.state must be execute|reject")
    typing_policy = payload.get("typingPolicy")
    if not isinstance(typing_policy, dict):
        raise ValueError("instruction-check payload typingPolicy must be an object")
    capability_claims = payload.get("capabilityClaims")
    if not isinstance(capability_claims, list):
        raise ValueError("instruction-check payload capabilityClaims must be a list")
    proposal = payload.get("proposal")
    if proposal is not None and not isinstance(proposal, dict):
        raise ValueError("instruction-check payload proposal must be an object when provided")
    return payload


def run_instruction_check(root: Path, instruction_path: Path) -> Dict[str, Any]:
    try:
        return run_core_json_command_from_path(
            root,
            subcommand="instruction-check",
            input_flag="--instruction",
            input_path=instruction_path,
            validate_payload=_validate_payload,
            default_failure_class="instruction_envelope_invalid",
            default_failure_message="instruction_envelope_invalid: instruction-check failed",
            invalid_json_message="instruction-check returned invalid JSON",
            invalid_json_failure_class="instruction_envelope_invalid_shape",
            validation_failure_class="instruction_envelope_invalid_shape",
            extra_args=["--repo-root", str(root)],
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise InstructionCheckError(exc.failure_class, exc.reason) from exc


def _validate_witness_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError("instruction-witness payload must be an object")
    required_string_fields = (
        "instructionId",
        "instructionRef",
        "instructionDigest",
        "verdictClass",
    )
    for key in required_string_fields:
        if not isinstance(payload.get(key), str) or not payload.get(key):
            raise ValueError(f"instruction-witness payload missing {key}")
    payload["witnessKind"] = resolve_schema_kind(
        "instructionWitnessKind",
        payload.get("witnessKind"),
        label="instruction-witness payload witnessKind",
    )
    for key in ("normalizerId", "policyDigest"):
        value = payload.get(key)
        if value is not None and (not isinstance(value, str) or not value):
            raise ValueError(
                f"instruction-witness payload {key} must be a non-empty string or null"
            )
    authority_payload_digest = payload.get("authorityPayloadDigest")
    if not isinstance(authority_payload_digest, str) or not authority_payload_digest:
        raise ValueError(
            "instruction-witness payload authorityPayloadDigest must be a non-empty string"
        )
    typed_core_projection_digest = payload.get("typedCoreProjectionDigest")
    if typed_core_projection_digest is not None and (
        not isinstance(typed_core_projection_digest, str) or not typed_core_projection_digest
    ):
        raise ValueError(
            "instruction-witness payload typedCoreProjectionDigest must be a non-empty string when present"
        )
    if payload.get("normalizerId") is not None and payload.get("policyDigest") is not None:
        if not isinstance(typed_core_projection_digest, str) or not typed_core_projection_digest:
            raise ValueError(
                "instruction-witness payload missing typedCoreProjectionDigest for bound authority"
            )
    if not isinstance(payload.get("results"), list):
        raise ValueError("instruction-witness payload results must be a list")
    if not isinstance(payload.get("failureClasses"), list):
        raise ValueError("instruction-witness payload failureClasses must be a list")
    if not isinstance(payload.get("operationalFailureClasses"), list):
        raise ValueError("instruction-witness payload operationalFailureClasses must be a list")
    if not isinstance(payload.get("semanticFailureClasses"), list):
        raise ValueError("instruction-witness payload semanticFailureClasses must be a list")
    reject_stage = payload.get("rejectStage")
    reject_reason = payload.get("rejectReason")
    if reject_stage is not None:
        if not isinstance(reject_stage, str) or not reject_stage:
            raise ValueError(
                "instruction-witness payload rejectStage must be a non-empty string when present"
            )
        if not isinstance(reject_reason, str) or not reject_reason:
            raise ValueError(
                "instruction-witness payload rejectReason must be a non-empty string when rejectStage is present"
            )
    elif reject_reason is not None:
        raise ValueError("instruction-witness payload rejectReason requires rejectStage")
    return payload


def run_instruction_witness(
    root: Path,
    instruction_path: Path,
    runtime_payload: Dict[str, Any],
    pre_execution_failure_class: str | None = None,
    pre_execution_reason: str | None = None,
) -> Dict[str, Any]:
    if (pre_execution_failure_class is None) ^ (pre_execution_reason is None):
        raise InstructionWitnessError(
            "instruction_runtime_invalid",
            "pre_execution_failure_class and pre_execution_reason must be provided together",
        )

    extra_args: List[str] = [
        "--instruction",
        str(instruction_path),
        "--repo-root",
        str(root),
    ]
    if pre_execution_failure_class is not None:
        extra_args.extend(
            [
                "--pre-execution-failure-class",
                pre_execution_failure_class,
                "--pre-execution-reason",
                pre_execution_reason or "",
            ]
        )

    try:
        return run_core_json_command(
            root,
            subcommand="instruction-witness",
            input_flag="--runtime",
            request_payload=runtime_payload,
            validate_payload=_validate_witness_payload,
            default_failure_class="instruction_runtime_invalid",
            default_failure_message="instruction_runtime_invalid: instruction-witness failed",
            invalid_json_message="instruction-witness returned invalid JSON",
            validation_failure_class="instruction_runtime_invalid",
            extra_args=extra_args,
            resolve_cli=resolve_premath_cli,
            run_process=subprocess.run,
        )
    except CoreCliClientError as exc:
        raise InstructionWitnessError(exc.failure_class, exc.reason) from exc
