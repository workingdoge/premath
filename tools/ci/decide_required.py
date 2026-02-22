#!/usr/bin/env python3
"""Produce an accept/reject decision from ci.required witness semantics."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Dict, List

from change_projection import detect_changed_paths, normalize_paths, project_required_checks
from delta_snapshot import (
    default_delta_snapshot_path,
    load_delta_snapshot,
    read_changed_paths,
)
from required_witness_decide_client import (
    RequiredWitnessDecideError,
    run_required_witness_decide,
)


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Decide accept/reject from ci.required witness.")
    parser.add_argument(
        "witness",
        nargs="?",
        type=Path,
        default=None,
        help="Path to witness JSON file. Default: artifacts/ciwitness/latest-required.json",
    )
    parser.add_argument(
        "--from-ref",
        default=None,
        help="Git ref used as delta base (default: PREMATH_CI_BASE_REF or auto-detect).",
    )
    parser.add_argument(
        "--to-ref",
        default=None,
        help="Git ref used as delta head (default: PREMATH_CI_HEAD_REF or HEAD).",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=root / "artifacts" / "ciwitness",
        help=f"Witness directory (default: {root / 'artifacts' / 'ciwitness'})",
    )
    parser.add_argument(
        "--delta-snapshot",
        type=Path,
        default=None,
        help="Path to delta snapshot JSON. Default: <out-dir>/latest-delta.json when present.",
    )
    parser.add_argument(
        "--compare-delta",
        action="store_true",
        help="Compare witness changedPaths against currently detected delta.",
    )
    parser.add_argument(
        "--require-native-check",
        action="append",
        default=None,
        help=(
            "Check ID that must have gateWitnessRefs.source=native. Repeatable. "
            "Can also be set via PREMATH_CI_NATIVE_REQUIRED_CHECKS=csv."
        ),
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Optional path to write decision JSON artifact.",
    )
    return parser.parse_args()


def _load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _native_required_checks(args: argparse.Namespace) -> List[str]:
    checks: List[str] = []
    if args.require_native_check:
        checks.extend(item.strip() for item in args.require_native_check if isinstance(item, str))
    env_csv = os.environ.get("PREMATH_CI_NATIVE_REQUIRED_CHECKS", "")
    if env_csv:
        checks.extend(part.strip() for part in env_csv.split(","))
    out: List[str] = []
    seen: set[str] = set()
    for check_id in checks:
        if not check_id or check_id in seen:
            continue
        seen.add(check_id)
        out.append(check_id)
    return out


def _detect_changed_paths(root: Path, from_ref: str | None, to_ref: str | None) -> List[str]:
    detected = detect_changed_paths(root, from_ref=from_ref, to_ref=to_ref)
    return detected.changed_paths


def _resolve_delta_snapshot_path(root: Path, args: argparse.Namespace, out_dir: Path) -> Path:
    if args.delta_snapshot is not None:
        path = args.delta_snapshot
        if not path.is_absolute():
            path = (root / path).resolve()
        return path
    return default_delta_snapshot_path(out_dir)


def _resolve_compare_paths(root: Path, out_dir: Path, args: argparse.Namespace) -> List[str]:
    snapshot_path = _resolve_delta_snapshot_path(root, args, out_dir)
    if snapshot_path.exists() and snapshot_path.is_file():
        snapshot = load_delta_snapshot(snapshot_path)
        return normalize_paths(read_changed_paths(snapshot))

    return _detect_changed_paths(root, from_ref=args.from_ref, to_ref=args.to_ref)


def _resolve_witness_path(root: Path, args: argparse.Namespace, changed_paths: List[str]) -> Path:
    if args.witness is not None:
        witness_path = args.witness
        if not witness_path.is_absolute():
            witness_path = (root / witness_path).resolve()
        return witness_path

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    latest_path = out_dir / "latest-required.json"
    if latest_path.exists() and latest_path.is_file():
        return latest_path

    projection = project_required_checks(changed_paths)
    return out_dir / f"{projection.projection_digest}.json"


def _render_and_write(decision: Dict[str, Any], root: Path, out_path_arg: Path | None) -> None:
    print(json.dumps(decision, indent=2, ensure_ascii=False))
    if out_path_arg is None:
        return
    out_path = out_path_arg
    if not out_path.is_absolute():
        out_path = (root / out_path).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(decision, f, indent=2, ensure_ascii=False)
        f.write("\n")


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]
    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()

    compare_paths: List[str] = []
    delta_snapshot_path: Path | None = None
    if args.compare_delta:
        try:
            snapshot_candidate = _resolve_delta_snapshot_path(root, args, out_dir)
            if snapshot_candidate.exists() and snapshot_candidate.is_file():
                snapshot = load_delta_snapshot(snapshot_candidate)
                compare_paths = normalize_paths(read_changed_paths(snapshot))
                delta_snapshot_path = snapshot_candidate
            else:
                compare_paths = _resolve_compare_paths(root, out_dir, args)
        except ValueError as exc:
            decision = {
                "schema": 1,
                "decisionKind": "ci.required.decision.v1",
                "decision": "reject",
                "reasonClass": "invalid_delta_snapshot",
                "errors": [str(exc)],
            }
            _render_and_write(decision, root, args.out)
            return 2

    witness_path = _resolve_witness_path(root, args, compare_paths)
    if not witness_path.exists() or not witness_path.is_file():
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": "missing_witness",
            "witnessPath": str(witness_path),
            "errors": [f"witness not found: {witness_path}"],
        }
        _render_and_write(decision, root, args.out)
        return 2

    try:
        witness = _load_json(witness_path)
    except (ValueError, json.JSONDecodeError) as exc:
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": "invalid_witness_json",
            "witnessPath": str(witness_path),
            "errors": [str(exc)],
        }
        _render_and_write(decision, root, args.out)
        return 2

    native_required_checks = _native_required_checks(args)
    decide_input: Dict[str, Any] = {
        "witness": witness,
        "nativeRequiredChecks": native_required_checks,
        "witnessRoot": str(witness_path.parent),
    }
    if args.compare_delta:
        decide_input["expectedChangedPaths"] = normalize_paths(compare_paths)

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
        _render_and_write(decision, root, args.out)
        return 2

    witness_sha = _sha256_file(witness_path)
    delta_sha: str | None = None
    if delta_snapshot_path is not None:
        delta_sha = _sha256_file(delta_snapshot_path)

    decision = {
        "schema": 1,
        "decisionKind": core_decision.get("decisionKind", "ci.required.decision.v1"),
        "decision": core_decision.get("decision", "reject"),
        "witnessPath": str(witness_path),
        "witnessSha256": witness_sha,
        "deltaSnapshotPath": str(delta_snapshot_path) if delta_snapshot_path is not None else None,
        "deltaSha256": delta_sha,
        "typedCoreProjectionDigest": core_decision.get("typedCoreProjectionDigest")
        or witness.get("typedCoreProjectionDigest"),
        "authorityPayloadDigest": core_decision.get("authorityPayloadDigest")
        or witness.get("authorityPayloadDigest"),
        "normalizerId": core_decision.get("normalizerId") or witness.get("normalizerId"),
        "policyDigest": core_decision.get("policyDigest") or witness.get("policyDigest"),
        "projectionDigest": core_decision.get("projectionDigest"),
        "requiredChecks": core_decision.get("requiredChecks"),
        "nativeRequiredChecks": native_required_checks,
        "reasonClass": core_decision.get("reasonClass", "verification_reject"),
        "errors": core_decision.get("errors", []),
    }

    _render_and_write(decision, root, args.out)
    return 0 if decision.get("decision") == "accept" else 1


if __name__ == "__main__":
    raise SystemExit(main())
