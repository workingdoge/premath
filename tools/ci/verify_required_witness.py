#!/usr/bin/env python3
"""Verify ci.required witness artifacts against deterministic CI-topos projection."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List

from change_projection import detect_changed_paths, normalize_paths, project_required_checks
from required_witness import verify_required_witness_payload


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Verify a projected required-check witness artifact.")
    parser.add_argument(
        "witness",
        nargs="?",
        type=Path,
        default=None,
        help="Path to witness JSON file. Default: artifacts/ciwitness/latest-required.json",
    )
    parser.add_argument(
        "--changed-file",
        action="append",
        default=None,
        help="Changed path (repeatable). If omitted, uses git diff detection.",
    )
    parser.add_argument(
        "--from-ref",
        default=None,
        help="Git ref used as delta base (default: auto-detect).",
    )
    parser.add_argument(
        "--to-ref",
        default="HEAD",
        help="Git ref used as delta head (default: HEAD).",
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
        help="Also compare witness changedPaths to currently detected delta.",
    )
    return parser.parse_args()


def _load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def _detect_changed_paths(root: Path, args: argparse.Namespace) -> List[str]:
    if args.changed_file:
        return normalize_paths(args.changed_file)

    detected = detect_changed_paths(root, from_ref=args.from_ref, to_ref=args.to_ref)
    return detected.changed_paths


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]

    changed_paths: List[str] = []

    witness_path: Path
    if args.witness is not None:
        witness_path = args.witness
        if not witness_path.is_absolute():
            witness_path = (root / witness_path).resolve()
    else:
        out_dir = args.out_dir
        if not out_dir.is_absolute():
            out_dir = (root / out_dir).resolve()
        latest_path = out_dir / "latest-required.json"
        if latest_path.exists() and latest_path.is_file():
            witness_path = latest_path
        else:
            changed_paths = _detect_changed_paths(root, args)
            projection = project_required_checks(changed_paths)
            witness_path = out_dir / f"{projection.projection_digest}.json"

    if not witness_path.exists() or not witness_path.is_file():
        print(f"[verify-required] witness not found: {witness_path}", file=sys.stderr)
        return 2

    try:
        witness = _load_json(witness_path)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[verify-required] invalid witness json: {exc}", file=sys.stderr)
        return 2

    witness_changed_paths = witness.get("changedPaths")
    if not isinstance(witness_changed_paths, list):
        print("[verify-required] invalid witness: changedPaths must be a list", file=sys.stderr)
        return 1

    errors, derived = verify_required_witness_payload(
        witness,
        witness_changed_paths,
        witness_root=witness_path.parent,
    )

    if args.compare_delta:
        if not changed_paths:
            changed_paths = _detect_changed_paths(root, args)
        expected_paths = normalize_paths(changed_paths)
        actual_paths = normalize_paths(witness_changed_paths)
        if expected_paths != actual_paths:
            errors.append(
                "delta comparison mismatch "
                f"(detected={expected_paths}, witness={actual_paths})"
            )

    if errors:
        print(f"[verify-required] FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[verify-required] OK "
        f"(projection={derived['projectionDigest']}, checks={len(derived['requiredChecks'])})"
    )
    print(f"[verify-required] witness: {witness_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
