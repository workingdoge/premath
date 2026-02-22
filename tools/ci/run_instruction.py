#!/usr/bin/env python3
"""Run instruction envelopes through the CI gate and emit a CI witness artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List

from instruction_policy import (
    PolicyValidationError,
    validate_proposal_binding_matches_envelope,
    validate_requested_checks,
)
from instruction_proposal import (
    ProposalValidationError,
    compile_proposal_obligations,
    discharge_proposal_obligations,
    validate_instruction_proposal,
)


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
    trimmed = value.strip()
    if trimmed != value:
        raise ValueError(f"{label} must not include leading/trailing whitespace")
    return trimmed


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


def parse_capability_claims(envelope: Dict[str, Any]) -> List[str]:
    raw_claims = envelope.get("capabilityClaims", [])
    if raw_claims is None:
        raw_claims = []
    if not isinstance(raw_claims, list):
        raise ValueError("capabilityClaims must be a list when provided")
    claims: List[str] = []
    for idx, item in enumerate(raw_claims):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"capabilityClaims[{idx}] must be a non-empty string")
        claims.append(item.strip())
    return sorted(set(claims))


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


def fallback_instruction_id_from_path(path: Path) -> str:
    stem = path.stem
    if stem:
        return stem
    name = path.name
    if name:
        return name
    return "instruction-invalid"


def classify_invalid_envelope(exc: Exception) -> Dict[str, str]:
    message = str(exc).strip()
    if not message:
        return {
            "failureClass": "instruction_envelope_invalid",
            "reason": "invalid instruction envelope",
        }

    # Policy/proposal validation errors are raised as "<failure_class>: <message>".
    match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
    if match:
        return {
            "failureClass": match.group("class"),
            "reason": match.group("reason") or message,
        }

    lowered = message.lower()
    if "jsondecodeerror" in lowered or "expecting value" in lowered:
        failure_class = "instruction_envelope_invalid_json"
    elif "filename" in lowered or ".json" in lowered:
        failure_class = "instruction_filename_invalid"
    elif "schema" in lowered:
        failure_class = "instruction_invalid_schema"
    elif "intent" in lowered:
        failure_class = "instruction_invalid_intent"
    elif "normalizerid" in lowered:
        failure_class = "instruction_invalid_normalizer"
    elif "policydigest" in lowered:
        failure_class = "instruction_invalid_policy_digest"
    elif "scope is required" in lowered:
        failure_class = "instruction_scope_missing"
    elif "scope" in lowered:
        failure_class = "instruction_scope_invalid"
    elif "requestedchecks" in lowered:
        failure_class = "instruction_requested_checks_invalid"
    elif "instructiontype" in lowered:
        failure_class = "instruction_instruction_type_invalid"
    elif "typingpolicy" in lowered:
        failure_class = "instruction_typing_policy_invalid"
    elif "root must be a json object" in lowered or "instruction envelope root must be an object" in lowered:
        failure_class = "instruction_envelope_invalid_shape"
    else:
        failure_class = "instruction_envelope_invalid"

    return {
        "failureClass": failure_class,
        "reason": message,
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
        "witnessKind": "ci.instruction.v1",
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
        envelope = load_instruction(instruction_path)
        raw_envelope = envelope
        instruction_id = instruction_id_from_path(instruction_path)

        intent = ensure_string(envelope.get("intent"), "intent")
        normalizer_id = ensure_string(envelope.get("normalizerId"), "normalizerId")
        policy_digest = ensure_string(envelope.get("policyDigest"), "policyDigest")
        if "scope" not in envelope:
            raise ValueError("scope is required")
        scope = envelope.get("scope")
        if scope in (None, ""):
            raise ValueError("scope must be non-empty")
        requested_checks = ensure_string_list(envelope.get("requestedChecks"), "requestedChecks")
        try:
            validate_requested_checks(
                policy_digest,
                requested_checks,
                normalizer_id=normalizer_id,
            )
        except PolicyValidationError as exc:
            raise ValueError(f"{exc.failure_class}: {exc}") from exc
        typing_policy = parse_typing_policy(envelope)
        capability_claims = parse_capability_claims(envelope)
        instruction_classification = classify_instruction(envelope, requested_checks)
        try:
            proposal = validate_instruction_proposal(envelope)
        except ProposalValidationError as exc:
            raise ValueError(f"{exc.failure_class}: {exc}") from exc
        try:
            validate_proposal_binding_matches_envelope(normalizer_id, policy_digest, proposal)
        except PolicyValidationError as exc:
            raise ValueError(f"{exc.failure_class}: {exc}") from exc
        if proposal is not None:
            proposal_obligations = compile_proposal_obligations(proposal["canonical"])
            proposal_discharge = discharge_proposal_obligations(
                proposal["canonical"],
                proposal_obligations,
            )
    except (ValueError, json.JSONDecodeError) as exc:
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

    instruction_digest = "instr1_" + stable_hash(envelope)
    rel_instruction_ref = str(instruction_path.relative_to(root)) if instruction_path.is_relative_to(root) else str(instruction_path)

    results: List[Dict[str, Any]] = []
    failed: List[Dict[str, Any]] = []
    failure_classes: List[str] = []
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
        failure_classes = ["instruction_unknown_unroutable"]
        reason = instruction_classification.get("reason", "unknown")
        print(
            f"[instruction] classification rejected: unknown(reason={reason}) "
            f"without allowUnknown policy",
            file=sys.stderr,
        )
    elif proposal_rejected:
        verdict_class = "rejected"
        failure_classes = sorted(
            set(
                item
                for item in proposal_discharge.get("failureClasses", [])
                if isinstance(item, str) and item
            )
        )
        print(
            "[instruction] proposal discharge rejected before execution "
            f"(failureClasses={failure_classes})",
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
        "normalizerId": normalizer_id,
        "policyDigest": policy_digest,
        "capabilityClaims": capability_claims,
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
    if proposal is not None:
        witness["proposalIngest"] = {
            "state": "typed",
            "kind": f"proposal.{proposal['canonical']['proposalKind']}",
            "proposalDigest": proposal["digest"],
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
