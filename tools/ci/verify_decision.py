#!/usr/bin/env python3
"""Verify ci.required decision artifacts through core command semantics."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

from required_decision_verify_client import (
    RequiredDecisionVerifyError,
    run_required_decision_verify,
)


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
        print(f"[verify-decision] witness not found: {witness_path}", file=sys.stderr)
        return 2
    if not delta_path.exists() or not delta_path.is_file():
        print(f"[verify-decision] delta snapshot not found: {delta_path}", file=sys.stderr)
        return 2

    try:
        witness = _load_json(witness_path)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[verify-decision] invalid witness json: {exc}", file=sys.stderr)
        return 2
    try:
        delta_snapshot = _load_json(delta_path)
    except (ValueError, json.JSONDecodeError) as exc:
        print(f"[verify-decision] invalid delta snapshot json: {exc}", file=sys.stderr)
        return 2

    verify_input = {
        "decision": decision,
        "witness": witness,
        "deltaSnapshot": delta_snapshot,
        "actualWitnessSha256": _sha256_file(witness_path),
        "actualDeltaSha256": _sha256_file(delta_path),
    }
    try:
        payload = run_required_decision_verify(root, verify_input)
    except RequiredDecisionVerifyError as exc:
        print(
            f"[verify-decision] core verify failed: {exc.failure_class}: {exc.reason}",
            file=sys.stderr,
        )
        return 2

    errors = payload.get("errors", [])
    if errors:
        print(f"[verify-decision] FAIL ({len(errors)} errors)")
        for err in errors:
            print(f"  - {err}")
        return 1

    derived = payload.get("derived", {})
    decision_value = derived.get("decision") or decision.get("decision")
    projection_digest = derived.get("projectionDigest") or decision.get("projectionDigest")
    print(
        "[verify-decision] OK "
        f"(decision={decision_value}, projection={projection_digest})"
    )
    print(f"[verify-decision] decision: {decision_path}")
    print(f"[verify-decision] witness: {witness_path}")
    print(f"[verify-decision] delta: {delta_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
