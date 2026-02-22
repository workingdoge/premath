#!/usr/bin/env python3
"""Verify ci.required decision artifacts against witness/delta attestation chain."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from control_plane_contract import REQUIRED_DECISION_KIND


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Verify ci.required decision attestation chain.")
    parser.add_argument(
        "decision",
        nargs="?",
        type=Path,
        default=None,
        help="Path to decision JSON file. Default: artifacts/ciwitness/latest-decision.json",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=root / "artifacts" / "ciwitness",
        help=f"Artifact directory (default: {root / 'artifacts' / 'ciwitness'})",
    )
    parser.add_argument(
        "--witness",
        type=Path,
        default=None,
        help="Override witness path. Default: decision.witnessPath or <out-dir>/latest-required.json",
    )
    parser.add_argument(
        "--delta-snapshot",
        type=Path,
        default=None,
        help="Override delta snapshot path. Default: decision.deltaSnapshotPath or <out-dir>/latest-delta.json",
    )
    return parser.parse_args()


def _resolve_path(root: Path, path: Path) -> Path:
    if path.is_absolute():
        return path
    return (root / path).resolve()


def _load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"json root must be object: {path}")
    return payload


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str):
            raise ValueError(f"{label}[{idx}] must be a string")
        out.append(item)
    return out


def _resolve_side_paths(
    root: Path,
    out_dir: Path,
    decision: Dict[str, Any],
    witness_arg: Optional[Path],
    delta_arg: Optional[Path],
) -> Tuple[Path, Path]:
    if witness_arg is not None:
        witness_path = _resolve_path(root, witness_arg)
    else:
        witness_raw = decision.get("witnessPath")
        if isinstance(witness_raw, str) and witness_raw:
            witness_path = _resolve_path(root, Path(witness_raw))
        else:
            witness_path = out_dir / "latest-required.json"

    if delta_arg is not None:
        delta_path = _resolve_path(root, delta_arg)
    else:
        delta_raw = decision.get("deltaSnapshotPath")
        if isinstance(delta_raw, str) and delta_raw:
            delta_path = _resolve_path(root, Path(delta_raw))
        else:
            delta_path = out_dir / "latest-delta.json"

    return (witness_path, delta_path)


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]
    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()

    decision_path = args.decision or (out_dir / "latest-decision.json")
    if not decision_path.is_absolute():
        decision_path = (root / decision_path).resolve()
    if not decision_path.exists() or not decision_path.is_file():
        print(f"[verify-decision] decision not found: {decision_path}", file=sys.stderr)
        return 2

    try:
        decision = _load_json(decision_path)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[verify-decision] invalid decision json: {exc}", file=sys.stderr)
        return 2

    errors: List[str] = []
    if decision.get("decisionKind") != REQUIRED_DECISION_KIND:
        errors.append(f"decisionKind must be {REQUIRED_DECISION_KIND!r}")
    decision_value = decision.get("decision")
    if decision_value not in {"accept", "reject"}:
        errors.append("decision must be 'accept' or 'reject'")

    try:
        witness_path, delta_path = _resolve_side_paths(
            root=root,
            out_dir=out_dir,
            decision=decision,
            witness_arg=args.witness,
            delta_arg=args.delta_snapshot,
        )
    except Exception as exc:  # pragma: no cover - defensive
        print(f"[verify-decision] failed to resolve witness/delta paths: {exc}", file=sys.stderr)
        return 2

    if not witness_path.exists() or not witness_path.is_file():
        errors.append(f"witness not found: {witness_path}")
        witness = None
    else:
        try:
            witness = _load_json(witness_path)
        except (ValueError, json.JSONDecodeError) as exc:
            errors.append(f"invalid witness json: {exc}")
            witness = None

    if not delta_path.exists() or not delta_path.is_file():
        errors.append(f"delta snapshot not found: {delta_path}")
        delta = None
    else:
        try:
            delta = _load_json(delta_path)
        except (ValueError, json.JSONDecodeError) as exc:
            errors.append(f"invalid delta snapshot json: {exc}")
            delta = None

    witness_sha = decision.get("witnessSha256")
    if not isinstance(witness_sha, str) or not witness_sha:
        errors.append("decision.witnessSha256 must be a non-empty string")
    elif witness is not None:
        actual_witness_sha = _sha256_file(witness_path)
        if witness_sha != actual_witness_sha:
            errors.append(
                f"witness sha mismatch (decision={witness_sha}, actual={actual_witness_sha})"
            )

    delta_sha = decision.get("deltaSha256")
    if not isinstance(delta_sha, str) or not delta_sha:
        errors.append("decision.deltaSha256 must be a non-empty string")
    elif delta is not None:
        actual_delta_sha = _sha256_file(delta_path)
        if delta_sha != actual_delta_sha:
            errors.append(
                f"delta sha mismatch (decision={delta_sha}, actual={actual_delta_sha})"
            )

    if witness is not None:
        if witness.get("projectionDigest") != decision.get("projectionDigest"):
            errors.append("projectionDigest mismatch between decision and witness")
        try:
            decision_checks = _string_list(decision.get("requiredChecks", []), "decision.requiredChecks")
            witness_checks = _string_list(witness.get("requiredChecks", []), "witness.requiredChecks")
            if decision_checks != witness_checks:
                errors.append("requiredChecks mismatch between decision and witness")
        except ValueError as exc:
            errors.append(str(exc))

    if delta is not None:
        if delta.get("projectionDigest") != decision.get("projectionDigest"):
            errors.append("projectionDigest mismatch between decision and delta snapshot")
        try:
            decision_checks = _string_list(decision.get("requiredChecks", []), "decision.requiredChecks")
            delta_checks = _string_list(delta.get("requiredChecks", []), "delta.requiredChecks")
            if decision_checks != delta_checks:
                errors.append("requiredChecks mismatch between decision and delta snapshot")
        except ValueError as exc:
            errors.append(str(exc))

    if errors:
        print(f"[verify-decision] FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[verify-decision] OK "
        f"(decision={decision.get('decision')}, projection={decision.get('projectionDigest')})"
    )
    print(f"[verify-decision] decision: {decision_path}")
    print(f"[verify-decision] witness: {witness_path}")
    print(f"[verify-decision] delta: {delta_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
