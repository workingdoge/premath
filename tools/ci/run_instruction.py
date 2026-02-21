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


SUPPORTED_INSTRUCTION_TYPES = {
    "ci.gate.check",
    "ci.gate.pre_commit",
    "ci.gate.pre_push",
}


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


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label} must be a non-empty list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    if len(set(out)) != len(out):
        raise ValueError(f"{label} must not contain duplicate check IDs")
    return out


def parse_typing_policy(envelope: Dict[str, Any]) -> Dict[str, Any]:
    raw_policy = envelope.get("typingPolicy", {})
    if raw_policy is None:
        raw_policy = {}
    if not isinstance(raw_policy, dict):
        raise ValueError("typingPolicy must be an object when provided")
    allow_unknown = raw_policy.get("allowUnknown", False)
    if not isinstance(allow_unknown, bool):
        raise ValueError("typingPolicy.allowUnknown must be a boolean when provided")
    return {"allowUnknown": allow_unknown}


def classify_instruction(envelope: Dict[str, Any], requested_checks: List[str]) -> Dict[str, str]:
    instruction_type = envelope.get("instructionType")
    if instruction_type is not None:
        instruction_type = ensure_string(instruction_type, "instructionType")
        if instruction_type in SUPPORTED_INSTRUCTION_TYPES:
            return {"state": "typed", "kind": instruction_type}
        return {"state": "unknown", "reason": "unsupported_instruction_type"}

    hk_prefixed = all(check_id.startswith("hk-") for check_id in requested_checks)
    if hk_prefixed:
        if requested_checks == ["hk-pre-commit"]:
            return {"state": "typed", "kind": "ci.gate.pre_commit"}
        if requested_checks == ["hk-pre-push"]:
            return {"state": "typed", "kind": "ci.gate.pre_push"}
        return {"state": "typed", "kind": "ci.gate.check"}

    return {"state": "unknown", "reason": "unrecognized_requested_checks"}


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

    try:
        envelope = load_instruction(instruction_path)
        instruction_id = instruction_id_from_path(instruction_path)

        intent = ensure_string(envelope.get("intent"), "intent")
        policy_digest = ensure_string(envelope.get("policyDigest"), "policyDigest")
        if "scope" not in envelope:
            raise ValueError("scope is required")
        scope = envelope.get("scope")
        if scope in (None, ""):
            raise ValueError("scope must be non-empty")
        requested_checks = ensure_string_list(envelope.get("requestedChecks"), "requestedChecks")
        typing_policy = parse_typing_policy(envelope)
        instruction_classification = classify_instruction(envelope, requested_checks)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[error] invalid instruction envelope: {exc}", file=sys.stderr)
        return 2

    instruction_digest = "instr1_" + stable_hash(envelope)
    rel_instruction_ref = str(instruction_path.relative_to(root)) if instruction_path.is_relative_to(root) else str(instruction_path)

    started_at = datetime.now(timezone.utc)
    results: List[Dict[str, Any]] = []
    failed: List[Dict[str, Any]] = []
    failure_classes: List[str] = []
    unknown_rejected = (
        instruction_classification.get("state") == "unknown"
        and not typing_policy.get("allowUnknown", False)
    )

    if unknown_rejected:
        verdict_class = "rejected"
        failure_classes = ["instruction_unknown_unroutable"]
        reason = instruction_classification.get("reason", "unknown")
        print(
            f"[instruction] classification rejected: unknown(reason={reason}) "
            f"without allowUnknown policy",
            file=sys.stderr,
        )
    else:
        for check_id in requested_checks:
            print(f"[instruction] running check: {check_id}")
            results.append(run_check(root, check_id))
        failed = [r for r in results if r["exitCode"] != 0]
        verdict_class = "accepted" if not failed else "rejected"
        if failed:
            failure_classes = ["check_failed"]

    squeak_site_profile = os.environ.get(
        "PREMATH_SQUEAK_SITE_PROFILE",
        os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
    )

    finished_at = datetime.now(timezone.utc)
    executed_checks = [r["checkId"] for r in results]

    witness = {
        "ciSchema": 1,
        "witnessKind": "ci.instruction.v1",
        "instructionId": instruction_id,
        "instructionRef": rel_instruction_ref,
        "instructionDigest": instruction_digest,
        "instructionClassification": instruction_classification,
        "typingPolicy": typing_policy,
        "intent": intent,
        "scope": scope,
        "policyDigest": policy_digest,
        "requiredChecks": requested_checks,
        "executedChecks": executed_checks,
        "results": results,
        "verdictClass": verdict_class,
        "failureClasses": failure_classes,
        "squeakSiteProfile": squeak_site_profile,
        "runStartedAt": started_at.isoformat(),
        "runFinishedAt": finished_at.isoformat(),
        "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
    }

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{instruction_id}.json"
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(witness, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"[instruction] witness written: {out_path}")

    if verdict_class == "rejected" and not args.allow_failure:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
