#!/usr/bin/env python3
"""Run the ci.required attestation chain without split wrapper scripts."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence

from delta_snapshot import default_delta_snapshot_path, load_delta_snapshot, read_changed_paths
from required_decision_verify_client import (
    RequiredDecisionVerifyError,
    run_required_decision_verify,
)
from required_witness_decide_client import (
    RequiredWitnessDecideError,
    run_required_witness_decide,
)
from required_witness_verify_client import verify_required_witness_payload


def _load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"json root must be object: {path}")
    return payload


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _normalize_paths(paths: Sequence[Any]) -> List[str]:
    out = {
        str(item).strip().replace("\\", "/").removeprefix("./")
        for item in paths
        if str(item).strip()
    }
    return sorted(out)


def _native_required_checks() -> List[str]:
    raw = os.environ.get("PREMATH_CI_NATIVE_REQUIRED_CHECKS", "")
    checks: List[str] = []
    seen: set[str] = set()
    for part in raw.split(","):
        check_id = part.strip()
        if not check_id or check_id in seen:
            continue
        seen.add(check_id)
        checks.append(check_id)
    return checks


def _run(cmd: Sequence[str], *, cwd: Path) -> int:
    completed = subprocess.run(cmd, cwd=cwd)
    return int(completed.returncode)


def _resolve_artifacts(root: Path) -> tuple[Path, Path, Path]:
    out_dir = root / "artifacts" / "ciwitness"
    return (
        out_dir / "latest-required.json",
        default_delta_snapshot_path(out_dir),
        out_dir / "latest-decision.json",
    )


def _verify_required(root: Path, witness_path: Path, delta_path: Path) -> int:
    if not witness_path.is_file():
        print(f"[required-attested] missing required witness: {witness_path}", file=sys.stderr)
        return 2
    if not delta_path.is_file():
        print(f"[required-attested] missing delta snapshot: {delta_path}", file=sys.stderr)
        return 2

    try:
        witness = _load_json(witness_path)
        delta = load_delta_snapshot(delta_path)
        witness_paths = witness.get("changedPaths")
        if not isinstance(witness_paths, list):
            print("[required-attested] invalid required witness: changedPaths must be a list", file=sys.stderr)
            return 1
        expected_paths = _normalize_paths(read_changed_paths(delta))
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[required-attested] invalid required witness inputs: {exc}", file=sys.stderr)
        return 2

    errors, derived = verify_required_witness_payload(
        witness,
        witness_paths,
        witness_root=witness_path.parent,
        native_required_checks=_native_required_checks(),
    )
    actual_paths = _normalize_paths(witness_paths)
    if expected_paths != actual_paths:
        errors.append(
            "delta comparison mismatch "
            f"(detected={expected_paths}, witness={actual_paths})"
        )

    if errors:
        print(f"[required-attested] verify-required FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[required-attested] verify-required OK "
        f"(projection={derived['projectionDigest']}, checks={len(derived['requiredChecks'])})"
    )
    return 0


def _write_decision(root: Path, witness_path: Path, delta_path: Path, decision_path: Path) -> int:
    try:
        witness = _load_json(witness_path)
        delta = load_delta_snapshot(delta_path)
        expected_paths = _normalize_paths(read_changed_paths(delta))
    except (ValueError, json.JSONDecodeError) as exc:
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": "invalid_attestation_input",
            "errors": [str(exc)],
        }
        _dump_decision(root, decision_path, decision)
        return 2

    decide_input: Dict[str, Any] = {
        "witness": witness,
        "nativeRequiredChecks": _native_required_checks(),
        "witnessRoot": str(witness_path.parent),
        "expectedChangedPaths": expected_paths,
    }
    try:
        core_decision = run_required_witness_decide(root, decide_input)
    except RequiredWitnessDecideError as exc:
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": exc.failure_class,
            "witnessPath": str(witness_path),
            "errors": [exc.reason],
        }
        _dump_decision(root, decision_path, decision)
        return 2

    decision = {
        "schema": 1,
        "decisionKind": core_decision.get("decisionKind", "ci.required.decision.v1"),
        "decision": core_decision.get("decision", "reject"),
        "witnessPath": str(witness_path),
        "witnessSha256": _sha256_file(witness_path),
        "deltaSnapshotPath": str(delta_path),
        "deltaSha256": _sha256_file(delta_path),
        "typedCoreProjectionDigest": core_decision.get("typedCoreProjectionDigest")
        or witness.get("typedCoreProjectionDigest"),
        "authorityPayloadDigest": core_decision.get("authorityPayloadDigest")
        or witness.get("authorityPayloadDigest"),
        "normalizerId": core_decision.get("normalizerId") or witness.get("normalizerId"),
        "policyDigest": core_decision.get("policyDigest") or witness.get("policyDigest"),
        "projectionDigest": core_decision.get("projectionDigest"),
        "requiredChecks": core_decision.get("requiredChecks"),
        "nativeRequiredChecks": _native_required_checks(),
        "reasonClass": core_decision.get("reasonClass", "verification_reject"),
        "errors": core_decision.get("errors", []),
    }
    _dump_decision(root, decision_path, decision)
    print(
        "[required-attested] decide-required "
        f"{decision['decision']} (reason={decision['reasonClass']})"
    )
    return 0 if decision.get("decision") == "accept" else 1


def _dump_decision(root: Path, decision_path: Path, decision: Dict[str, Any]) -> None:
    if not decision_path.is_absolute():
        decision_path = (root / decision_path).resolve()
    decision_path.parent.mkdir(parents=True, exist_ok=True)
    with decision_path.open("w", encoding="utf-8") as f:
        json.dump(decision, f, indent=2, ensure_ascii=False)
        f.write("\n")


def _verify_decision(root: Path, witness_path: Path, delta_path: Path, decision_path: Path) -> int:
    try:
        decision = _load_json(decision_path)
        witness = _load_json(witness_path)
        delta = load_delta_snapshot(delta_path)
        verify_input = {
            "decision": decision,
            "witness": witness,
            "deltaSnapshot": delta,
            "actualWitnessSha256": _sha256_file(witness_path),
            "actualDeltaSha256": _sha256_file(delta_path),
        }
        payload = run_required_decision_verify(root, verify_input)
    except (ValueError, json.JSONDecodeError, RequiredDecisionVerifyError) as exc:
        print(f"[required-attested] verify-decision failed: {exc}", file=sys.stderr)
        return 2

    errors = payload.get("errors", [])
    if errors:
        print(f"[required-attested] verify-decision FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    derived = payload.get("derived", {})
    print(
        "[required-attested] verify-decision OK "
        f"(decision={derived.get('decision') or decision.get('decision')}, "
        f"projection={derived.get('typedCoreProjectionDigest') or decision.get('typedCoreProjectionDigest')})"
    )
    return 0


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    witness_path, delta_path, decision_path = _resolve_artifacts(repo_root)
    exit_code = _run(("python3", "tools/ci/run_required_checks.py"), cwd=repo_root)
    if exit_code != 0:
        return exit_code
    for step in (
        lambda: _verify_required(repo_root, witness_path, delta_path),
        lambda: _write_decision(repo_root, witness_path, delta_path, decision_path),
        lambda: _verify_decision(repo_root, witness_path, delta_path, decision_path),
    ):
        exit_code = step()
        if exit_code != 0:
            return exit_code
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
