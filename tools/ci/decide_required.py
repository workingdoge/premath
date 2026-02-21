#!/usr/bin/env python3
"""Produce an accept/reject decision from verified ci.required witness semantics."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict, List

from change_projection import detect_changed_paths, normalize_paths, project_required_checks
from required_witness import verify_required_witness_payload


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


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]

    changed_paths: List[str] = []
    if args.compare_delta:
        changed_paths = _detect_changed_paths(root, from_ref=args.from_ref, to_ref=args.to_ref)

    witness_path = _resolve_witness_path(root, args, changed_paths)
    if not witness_path.exists() or not witness_path.is_file():
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": "missing_witness",
            "witnessPath": str(witness_path),
            "errors": [f"witness not found: {witness_path}"],
        }
        print(json.dumps(decision, indent=2, ensure_ascii=False))
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
        print(json.dumps(decision, indent=2, ensure_ascii=False))
        return 2

    witness_changed_paths = witness.get("changedPaths")
    if not isinstance(witness_changed_paths, list):
        decision = {
            "schema": 1,
            "decisionKind": "ci.required.decision.v1",
            "decision": "reject",
            "reasonClass": "invalid_witness_shape",
            "witnessPath": str(witness_path),
            "errors": ["changedPaths must be a list"],
        }
        print(json.dumps(decision, indent=2, ensure_ascii=False))
        return 1

    native_required_checks = _native_required_checks(args)
    errors, derived = verify_required_witness_payload(
        witness,
        witness_changed_paths,
        witness_root=witness_path.parent,
        native_required_checks=native_required_checks,
    )

    if args.compare_delta:
        expected_paths = normalize_paths(changed_paths)
        actual_paths = normalize_paths(witness_changed_paths)
        if expected_paths != actual_paths:
            errors.append(
                "delta comparison mismatch "
                f"(detected={expected_paths}, witness={actual_paths})"
            )

    witness_verdict = witness.get("verdictClass")
    if witness_verdict != "accepted":
        errors.append(
            f"required witness verdict must be accepted for decision accept "
            f"(actual={witness_verdict!r})"
        )

    decision = {
        "schema": 1,
        "decisionKind": "ci.required.decision.v1",
        "decision": "accept" if not errors else "reject",
        "witnessPath": str(witness_path),
        "projectionDigest": derived.get("projectionDigest"),
        "requiredChecks": derived.get("requiredChecks"),
        "nativeRequiredChecks": native_required_checks,
        "reasonClass": "verified_accept" if not errors else "verification_reject",
        "errors": errors,
    }

    print(json.dumps(decision, indent=2, ensure_ascii=False))
    if args.out is not None:
        out_path = args.out
        if not out_path.is_absolute():
            out_path = (root / out_path).resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with out_path.open("w", encoding="utf-8") as f:
            json.dump(decision, f, indent=2, ensure_ascii=False)
            f.write("\n")

    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
