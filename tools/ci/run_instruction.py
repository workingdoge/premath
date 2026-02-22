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
from instruction_check_client import InstructionCheckError, run_instruction_check


SUPPORTED_INSTRUCTION_TYPES = {
    "ci.gate.check",
    "ci.gate.pre_commit",
    "ci.gate.pre_push",
}


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def sorted_unique_strings(values: List[str]) -> List[str]:
    return sorted(set(item for item in values if isinstance(item, str) and item))


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


def classify_instruction(instruction_type: str | None, requested_checks: List[str]) -> Dict[str, str]:
    if instruction_type is not None:
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


def _normalized_typing_policy(envelope: Dict[str, Any] | None) -> Dict[str, Any]:
    if envelope is None:
        return {"allowUnknown": False}
    raw_policy = envelope.get("typingPolicy")
    if not isinstance(raw_policy, dict):
        return {"allowUnknown": False}
    allow_unknown = raw_policy.get("allowUnknown", False)
    if not isinstance(allow_unknown, bool):
        return {"allowUnknown": False}
    return {"allowUnknown": allow_unknown}


def _normalized_requested_checks(envelope: Dict[str, Any] | None) -> List[str]:
    if envelope is None:
        return []
    raw = envelope.get("requestedChecks", [])
    if not isinstance(raw, list):
        return []
    out: List[str] = []
    for item in raw:
        if isinstance(item, str) and item.strip():
            out.append(item.strip())
    return sorted(set(out))


def _normalized_capability_claims(envelope: Dict[str, Any] | None) -> List[str]:
    if envelope is None:
        return []
    raw = envelope.get("capabilityClaims", [])
    if not isinstance(raw, list):
        return []
    out: List[str] = []
    for item in raw:
        if isinstance(item, str) and item.strip():
            out.append(item.strip())
    return sorted(set(out))


def _string_or_none(value: Any) -> str | None:
    if isinstance(value, str) and value.strip():
        return value.strip()
    return None


def write_pre_execution_reject_witness(
    root: Path,
    out_dir: Path,
    instruction_path: Path,
    instruction_id: str,
    envelope: Dict[str, Any] | None,
    failure_class: str,
    reason: str,
    started_at: datetime,
) -> Path:
    rel_instruction_ref = (
        str(instruction_path.relative_to(root))
        if instruction_path.is_relative_to(root)
        else str(instruction_path)
    )
    finished_at = datetime.now(timezone.utc)

    intent = _string_or_none(envelope.get("intent")) if envelope is not None else None
    if intent is None:
        intent = "(invalid envelope)"

    witness = {
        "ciSchema": 1,
        "witnessKind": INSTRUCTION_WITNESS_KIND,
        "instructionId": instruction_id,
        "instructionRef": rel_instruction_ref,
        "instructionDigest": _normalized_instruction_digest(instruction_path, envelope),
        "instructionClassification": {
            "state": "unknown",
            "reason": "pre_execution_invalid",
        },
        "typingPolicy": _normalized_typing_policy(envelope),
        "intent": intent,
        "scope": envelope.get("scope") if envelope is not None else None,
        "normalizerId": _string_or_none(envelope.get("normalizerId")) if envelope is not None else None,
        "policyDigest": _string_or_none(envelope.get("policyDigest")) if envelope is not None else None,
        "capabilityClaims": _normalized_capability_claims(envelope),
        "requiredChecks": _normalized_requested_checks(envelope),
        "executedChecks": [],
        "results": [],
        "verdictClass": "rejected",
        "operationalFailureClasses": [failure_class],
        "semanticFailureClasses": [],
        "failureClasses": [failure_class],
        "rejectStage": "pre_execution",
        "rejectReason": reason,
        "squeakSiteProfile": os.environ.get(
            "PREMATH_SQUEAK_SITE_PROFILE",
            os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
        ),
        "runStartedAt": started_at.isoformat(),
        "runFinishedAt": finished_at.isoformat(),
        "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
    }

    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{instruction_id}.json"
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(witness, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return out_path


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

    proposal: Dict[str, Any] | None = None
    proposal_obligations: List[Dict[str, Any]] = []
    proposal_discharge: Dict[str, Any] | None = None

    started_at = datetime.now(timezone.utc)
    raw_envelope: Dict[str, Any] | None = None

    try:
        checked = run_instruction_check(root, instruction_path)
        instruction_id = instruction_id_from_path(instruction_path)
        intent = checked["intent"]
        scope = checked["scope"]
        normalizer_id = checked["normalizerId"]
        policy_digest = checked["policyDigest"]
        requested_checks = checked["requestedChecks"]
        typing_policy = checked.get("typingPolicy", {"allowUnknown": False})
        capability_claims = checked.get("capabilityClaims", [])
        instruction_classification = classify_instruction(
            checked.get("instructionType"),
            requested_checks,
        )
        proposal_payload = checked.get("proposal")
        if isinstance(proposal_payload, dict):
            proposal = {
                "canonical": proposal_payload.get("canonical"),
                "digest": proposal_payload.get("digest"),
                "kcirRef": proposal_payload.get("kcirRef"),
            }
            proposal_obligations = proposal_payload.get("obligations", [])
            proposal_discharge = proposal_payload.get("discharge")
        raw_envelope = load_instruction(instruction_path)
    except (InstructionCheckError, ValueError, json.JSONDecodeError) as exc:
        try:
            raw_envelope = load_instruction(instruction_path)
        except Exception:  # noqa: BLE001
            raw_envelope = None
        invalid = classify_invalid_envelope(exc)
        fallback_instruction_id = fallback_instruction_id_from_path(instruction_path)
        reject_path = write_pre_execution_reject_witness(
            root=root,
            out_dir=args.out_dir,
            instruction_path=instruction_path,
            instruction_id=fallback_instruction_id,
            envelope=raw_envelope,
            failure_class=invalid["failureClass"],
            reason=invalid["reason"],
            started_at=started_at,
        )
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
    failed: List[Dict[str, Any]] = []
    operational_failure_classes: List[str] = []
    semantic_failure_classes: List[str] = []
    unknown_rejected = (
        instruction_classification.get("state") == "unknown"
        and not typing_policy.get("allowUnknown", False)
    )
    proposal_rejected = (
        proposal_discharge is not None
        and proposal_discharge.get("outcome") == "rejected"
    )
    if unknown_rejected:
        verdict_class = "rejected"
        operational_failure_classes = ["instruction_unknown_unroutable"]
        reason = instruction_classification.get("reason", "unknown")
        print(
            f"[instruction] classification rejected: unknown(reason={reason}) "
            f"without allowUnknown policy",
            file=sys.stderr,
        )
    elif proposal_rejected:
        verdict_class = "rejected"
        semantic_failure_classes = sorted_unique_strings(
            [
                item
                for item in proposal_discharge.get("failureClasses", [])
                if isinstance(item, str) and item
            ]
        )
        print(
            "[instruction] proposal discharge rejected before execution "
            f"(failureClasses={semantic_failure_classes})",
            file=sys.stderr,
        )
    else:
        for check_id in requested_checks:
            print(f"[instruction] running check: {check_id}")
            results.append(run_check(root, check_id))
        failed = [r for r in results if r["exitCode"] != 0]
        verdict_class = "accepted" if not failed else "rejected"
        if failed:
            operational_failure_classes = ["check_failed"]

    operational_failure_classes = sorted_unique_strings(operational_failure_classes)
    semantic_failure_classes = sorted_unique_strings(semantic_failure_classes)
    failure_classes = sorted_unique_strings(
        operational_failure_classes + semantic_failure_classes
    )

    squeak_site_profile = os.environ.get(
        "PREMATH_SQUEAK_SITE_PROFILE",
        os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
    )

    finished_at = datetime.now(timezone.utc)
    executed_checks = [r["checkId"] for r in results]

    witness = {
        "ciSchema": 1,
        "witnessKind": INSTRUCTION_WITNESS_KIND,
        "instructionId": instruction_id,
        "instructionRef": rel_instruction_ref,
        "instructionDigest": instruction_digest,
        "instructionClassification": instruction_classification,
        "typingPolicy": typing_policy,
        "intent": intent,
        "scope": scope,
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
        "capabilityClaims": capability_claims,
        "requiredChecks": requested_checks,
        "executedChecks": executed_checks,
        "results": results,
        "verdictClass": verdict_class,
        "operationalFailureClasses": operational_failure_classes,
        "semanticFailureClasses": semantic_failure_classes,
        "failureClasses": failure_classes,
        "squeakSiteProfile": squeak_site_profile,
        "runStartedAt": started_at.isoformat(),
        "runFinishedAt": finished_at.isoformat(),
        "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
    }
    if proposal is not None:
        witness["proposalIngest"] = {
            "state": "typed",
            "kind": f"proposal.{proposal['canonical']['proposalKind']}",
            "proposalDigest": proposal["digest"],
            "proposalKcirRef": proposal["kcirRef"],
            "binding": proposal["canonical"]["binding"],
            "targetCtxRef": proposal["canonical"]["targetCtxRef"],
            "targetJudgment": proposal["canonical"]["targetJudgment"],
            "obligations": proposal_obligations,
            "discharge": proposal_discharge,
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
