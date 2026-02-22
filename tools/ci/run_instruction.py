#!/usr/bin/env python3
"""Run instruction envelopes through the CI gate and emit a CI witness artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List

from control_plane_contract import INSTRUCTION_WITNESS_KIND
from instruction_check_client import (
    InstructionCheckError,
    InstructionWitnessError,
    run_instruction_check,
    run_instruction_witness,
)


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Run an instruction envelope through gate checks and emit "
            "artifacts/ciwitness/<instruction-id>.json"
        )
    )
    parser.add_argument(
        "instruction",
        type=Path,
        help="Path to instruction envelope (recommended: instructions/<ts>-<id>.json)",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=root / "artifacts" / "ciwitness",
        help=(
            "Output directory for CI witness artifacts "
            f"(default: {root / 'artifacts' / 'ciwitness'})"
        ),
    )
    parser.add_argument(
        "--allow-failure",
        action="store_true",
        help="Exit 0 even when one or more checks fail.",
    )
    return parser.parse_args()


def load_instruction(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError("instruction envelope root must be an object")
    return data


def instruction_id_from_path(path: Path) -> str:
    if path.suffix != ".json":
        raise ValueError("instruction envelope must be a .json file")
    instruction_id = path.stem
    if "-" not in instruction_id:
        raise ValueError(
            "instruction filename must follow <ts>-<id>.json so artifact IDs are stable"
        )
    return instruction_id


def fallback_instruction_id_from_path(path: Path) -> str:
    stem = path.stem
    if stem:
        return stem
    name = path.name
    if name:
        return name
    return "instruction-invalid"


def classify_invalid_envelope(exc: Exception) -> Dict[str, str]:
    if isinstance(exc, InstructionCheckError):
        return {
            "failureClass": exc.failure_class,
            "reason": exc.reason,
        }
    message = str(exc).strip()
    return {
        "failureClass": "instruction_envelope_invalid",
        "reason": message or "invalid instruction envelope",
    }


def _normalized_instruction_digest(path: Path, envelope: Dict[str, Any] | None) -> str:
    if envelope is not None:
        return "instr1_" + stable_hash(envelope)
    return "instr1_" + hashlib.sha256(path.read_bytes()).hexdigest()


def run_check(root: Path, check_id: str) -> Dict[str, Any]:
    cmd = ["sh", str(root / "tools" / "ci" / "run_gate.sh"), check_id]
    started = time.perf_counter()
    completed = subprocess.run(cmd, cwd=root)
    duration_ms = int((time.perf_counter() - started) * 1000)
    rc = int(completed.returncode)
    return {
        "checkId": check_id,
        "status": "passed" if rc == 0 else "failed",
        "exitCode": rc,
        "durationMs": duration_ms,
    }


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]

    instruction_path = args.instruction
    if not instruction_path.is_absolute():
        instruction_path = (root / instruction_path).resolve()

    if not instruction_path.exists() or not instruction_path.is_file():
        print(f"[error] instruction file not found: {instruction_path}", file=sys.stderr)
        return 2

    started_at = datetime.now(timezone.utc)
    raw_envelope: Dict[str, Any] | None = None

    try:
        checked = run_instruction_check(root, instruction_path)
        instruction_id = instruction_id_from_path(instruction_path)
        requested_checks = checked["requestedChecks"]
        execution_decision = checked["executionDecision"]
        raw_envelope = load_instruction(instruction_path)
    except (InstructionCheckError, ValueError, json.JSONDecodeError) as exc:
        try:
            raw_envelope = load_instruction(instruction_path)
        except Exception:  # noqa: BLE001
            raw_envelope = None
        invalid = classify_invalid_envelope(exc)
        fallback_instruction_id = fallback_instruction_id_from_path(instruction_path)
        instruction_digest = _normalized_instruction_digest(instruction_path, raw_envelope)
        rel_instruction_ref = (
            str(instruction_path.relative_to(root))
            if instruction_path.is_relative_to(root)
            else str(instruction_path)
        )
        squeak_site_profile = os.environ.get(
            "PREMATH_SQUEAK_SITE_PROFILE",
            os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
        )
        finished_at = datetime.now(timezone.utc)
        runtime_payload = {
            "instructionId": fallback_instruction_id,
            "instructionRef": rel_instruction_ref,
            "instructionDigest": instruction_digest,
            "squeakSiteProfile": squeak_site_profile,
            "runStartedAt": started_at.isoformat(),
            "runFinishedAt": finished_at.isoformat(),
            "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
            "results": [],
        }
        try:
            witness = run_instruction_witness(
                root,
                instruction_path,
                runtime_payload,
                pre_execution_failure_class=invalid["failureClass"],
                pre_execution_reason=invalid["reason"],
            )
        except InstructionWitnessError as witness_error:
            print(
                (
                    "[error] instruction witness build failed: "
                    f"{witness_error.failure_class}: {witness_error.reason}"
                ),
                file=sys.stderr,
            )
            return 2
        out_dir = args.out_dir
        if not out_dir.is_absolute():
            out_dir = (root / out_dir).resolve()
        out_dir.mkdir(parents=True, exist_ok=True)
        reject_path = out_dir / f"{fallback_instruction_id}.json"
        with reject_path.open("w", encoding="utf-8") as f:
            json.dump(witness, f, indent=2, ensure_ascii=False)
            f.write("\n")
        print(
            (
                "[error] invalid instruction envelope: "
                f"{invalid['failureClass']}: {invalid['reason']}"
            ),
            file=sys.stderr,
        )
        print(f"[instruction] reject witness written: {reject_path}", file=sys.stderr)
        return 2

    instruction_digest = _normalized_instruction_digest(instruction_path, raw_envelope)
    rel_instruction_ref = str(instruction_path.relative_to(root)) if instruction_path.is_relative_to(root) else str(instruction_path)

    results: List[Dict[str, Any]] = []
    decision_state = execution_decision.get("state")
    if decision_state == "reject":
        source = execution_decision.get("source", "unknown")
        reason = execution_decision.get("reason", "unknown")
        semantic_failure_classes = [
            item
            for item in execution_decision.get("semanticFailureClasses", [])
            if isinstance(item, str) and item
        ]
        if source == "instruction_classification":
            print(
                f"[instruction] classification rejected: unknown(reason={reason}) "
                f"without allowUnknown policy",
                file=sys.stderr,
            )
        elif source == "proposal_discharge":
            print(
                "[instruction] proposal discharge rejected before execution "
                f"(failureClasses={semantic_failure_classes})",
                file=sys.stderr,
            )
        else:
            print(
                "[instruction] execution decision rejected before execution "
                f"(source={source}, reason={reason})",
                file=sys.stderr,
            )
    elif decision_state == "execute":
        for check_id in requested_checks:
            print(f"[instruction] running check: {check_id}")
            results.append(run_check(root, check_id))
    else:
        raise ValueError(
            "instruction-check payload executionDecision.state must be execute|reject"
        )

    squeak_site_profile = os.environ.get(
        "PREMATH_SQUEAK_SITE_PROFILE",
        os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
    )

    finished_at = datetime.now(timezone.utc)
    runtime_payload = {
        "instructionId": instruction_id,
        "instructionRef": rel_instruction_ref,
        "instructionDigest": instruction_digest,
        "squeakSiteProfile": squeak_site_profile,
        "runStartedAt": started_at.isoformat(),
        "runFinishedAt": finished_at.isoformat(),
        "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
        "results": results,
    }
    try:
        witness = run_instruction_witness(root, instruction_path, runtime_payload)
    except InstructionWitnessError as exc:
        print(
            f"[error] instruction witness build failed: {exc.failure_class}: {exc.reason}",
            file=sys.stderr,
        )
        return 2

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{instruction_id}.json"
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(witness, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"[instruction] witness written: {out_path}")

    if witness.get("witnessKind") != INSTRUCTION_WITNESS_KIND:
        print("[error] instruction witness kind mismatch", file=sys.stderr)
        return 2
    if witness.get("verdictClass") == "rejected" and not args.allow_failure:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
