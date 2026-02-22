#!/usr/bin/env python3
"""Execute only required checks derived from deterministic change projection."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from change_projection import (
    detect_changed_paths,
    normalize_paths,
    project_required_checks,
    projection_plan_payload,
)
from delta_snapshot import (
    default_delta_snapshot_path,
    make_delta_snapshot_payload,
    write_delta_snapshot,
)
from required_gate_ref_client import RequiredGateRefError, run_required_gate_ref
from required_witness_client import RequiredWitnessError, run_required_witness

_SAFE_CHECK_ID_RE = re.compile(r"[^A-Za-z0-9._-]+")


def _sanitize_check_id(check_id: str) -> str:
    sanitized = _SAFE_CHECK_ID_RE.sub("_", check_id.strip())
    sanitized = sanitized.strip("._")
    return sanitized or "check"


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Run required checks from projected change set.")
    parser.add_argument(
        "--changed-file",
        action="append",
        default=None,
        help="Changed path (repeatable). If omitted, uses git diff detection.",
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
        help=f"Output directory for CI closure witness artifacts (default: {root / 'artifacts' / 'ciwitness'})",
    )
    parser.add_argument(
        "--allow-failure",
        action="store_true",
        help="Exit 0 even when one or more required checks fail.",
    )
    parser.add_argument(
        "--print-plan",
        action="store_true",
        help="Print projection plan JSON before executing checks.",
    )
    return parser.parse_args()


def _gate_artifact_path(
    out_dir: Path,
    projection_digest: str,
    check_id: str,
    index: int,
) -> Path:
    gate_dir = out_dir / "gates" / projection_digest
    gate_dir.mkdir(parents=True, exist_ok=True)
    file_name = f"{index + 1:02d}-{_sanitize_check_id(check_id)}.json"
    return gate_dir / file_name


def _gate_source_path(
    out_dir: Path,
    projection_digest: str,
    check_id: str,
    index: int,
) -> Path:
    gate_dir = out_dir / "gates" / projection_digest
    gate_dir.mkdir(parents=True, exist_ok=True)
    file_name = f"{index + 1:02d}-{_sanitize_check_id(check_id)}.source"
    return gate_dir / file_name


def _load_json_object(path: Path) -> Optional[Dict[str, Any]]:
    try:
        with path.open("r", encoding="utf-8") as f:
            payload = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return None
    if not isinstance(payload, dict):
        return None
    return payload


def _load_gate_source(path: Path) -> Optional[str]:
    try:
        raw = path.read_text(encoding="utf-8").strip().lower()
    except OSError:
        return None
    if raw in {"native", "fallback"}:
        return raw
    return None


def run_check_with_witness(
    root: Path,
    out_dir: Path,
    check_id: str,
    projection_digest: str,
    policy_digest: str,
    from_ref: str | None,
    to_ref: str | None,
    index: int,
) -> Dict[str, Any]:
    gate_path = _gate_artifact_path(out_dir, projection_digest, check_id, index)
    source_path = _gate_source_path(out_dir, projection_digest, check_id, index)

    # Avoid stale gate artifacts from prior runs with the same projection digest.
    # If this check does not emit a native witness in the current run, fallback
    # witness emission must be derived from current exit code, not old files.
    for stale in (gate_path, source_path):
        try:
            stale.unlink()
        except FileNotFoundError:
            pass

    env = os.environ.copy()
    env["PREMATH_GATE_WITNESS_OUT"] = str(gate_path)
    env["PREMATH_GATE_WITNESS_SOURCE_OUT"] = str(source_path)
    env["PREMATH_GATE_CHECK_ID"] = check_id
    env["PREMATH_GATE_PROJECTION_DIGEST"] = projection_digest
    env["PREMATH_GATE_POLICY_DIGEST"] = policy_digest
    env["PREMATH_GATE_CTX_REF"] = from_ref or "ctx:unknown"
    env["PREMATH_GATE_DATA_HEAD_REF"] = to_ref or "HEAD"

    cmd = ["sh", str(root / "tools" / "ci" / "run_gate.sh"), check_id]
    started = time.perf_counter()
    completed = subprocess.run(cmd, cwd=root, env=env)
    duration_ms = int((time.perf_counter() - started) * 1000)
    exit_code = int(completed.returncode)
    row: Dict[str, Any] = {
        "checkId": check_id,
        "status": "passed" if exit_code == 0 else "failed",
        "exitCode": exit_code,
        "durationMs": duration_ms,
    }
    payload = _load_json_object(gate_path)
    if payload is not None:
        source = _load_gate_source(source_path)
        if source is None:
            source_candidate = payload.get("witnessSource")
            if isinstance(source_candidate, str) and source_candidate in {"native", "fallback"}:
                source = source_candidate
        if source is None:
            source = "native"
        try:
            gate_ref_payload = run_required_gate_ref(
                root,
                {
                    "checkId": check_id,
                    "artifactRelPath": gate_path.relative_to(out_dir).as_posix(),
                    "source": source,
                    "gatePayload": payload,
                },
            )
        except RequiredGateRefError as exc:
            raise ValueError(f"{exc.failure_class}: {exc.reason}") from exc
        gate_ref = gate_ref_payload.get("gateWitnessRef")
        if not isinstance(gate_ref, dict):
            raise ValueError("required-gate-ref payload missing gateWitnessRef")
        row["nativeGateWitnessRef"] = gate_ref
    return row


def emit_gate_witness(
    root: Path,
    out_dir: Path,
    projection_digest: str,
    policy_digest: str,
    from_ref: str | None,
    to_ref: str | None,
    check_row: Dict[str, Any],
    index: int,
) -> Dict[str, Any]:
    check_id = str(check_row["checkId"])
    exit_code = int(check_row["exitCode"])
    ctx_ref = from_ref or "ctx:unknown"
    data_head_ref = to_ref or "HEAD"

    gate_path = _gate_artifact_path(
        out_dir=out_dir,
        projection_digest=projection_digest,
        check_id=check_id,
        index=index,
    )
    try:
        gate_ref_payload = run_required_gate_ref(
            root,
            {
                "checkId": check_id,
                "artifactRelPath": gate_path.relative_to(out_dir).as_posix(),
                "source": "fallback",
                "fallback": {
                    "exitCode": exit_code,
                    "projectionDigest": projection_digest,
                    "policyDigest": policy_digest,
                    "ctxRef": ctx_ref,
                    "dataHeadRef": data_head_ref,
                },
            },
        )
    except RequiredGateRefError as exc:
        raise ValueError(f"{exc.failure_class}: {exc.reason}") from exc

    gate_payload = gate_ref_payload.get("gatePayload")
    if not isinstance(gate_payload, dict):
        raise ValueError("required-gate-ref fallback payload missing gatePayload")
    gate_ref = gate_ref_payload.get("gateWitnessRef")
    if not isinstance(gate_ref, dict):
        raise ValueError("required-gate-ref fallback payload missing gateWitnessRef")

    with gate_path.open("w", encoding="utf-8") as f:
        json.dump(gate_payload, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return gate_ref


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parents[2]

    if args.changed_file:
        changed_paths = normalize_paths(args.changed_file)
        source = "explicit"
        from_ref = args.from_ref
        to_ref = args.to_ref
    else:
        detected = detect_changed_paths(root, from_ref=args.from_ref, to_ref=args.to_ref)
        changed_paths = detected.changed_paths
        source = detected.source
        from_ref = detected.from_ref
        to_ref = detected.to_ref

    projection = project_required_checks(changed_paths)
    plan = projection_plan_payload(projection, source, from_ref, to_ref)

    if args.print_plan:
        json.dump(plan, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")

    required_checks = list(plan["requiredChecks"])
    started_at = datetime.now(timezone.utc)

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    delta_snapshot_path = default_delta_snapshot_path(out_dir)
    delta_snapshot_payload = make_delta_snapshot_payload(plan)
    write_delta_snapshot(delta_snapshot_path, delta_snapshot_payload)

    results: List[Dict[str, Any]] = []
    for idx, check_id in enumerate(required_checks):
        print(f"[ci-required] running check: {check_id}")
        try:
            results.append(
                run_check_with_witness(
                    root=root,
                    out_dir=out_dir,
                    check_id=check_id,
                    projection_digest=plan["projectionDigest"],
                    policy_digest=plan["projectionPolicy"],
                    from_ref=from_ref,
                    to_ref=to_ref,
                    index=idx,
                )
            )
        except ValueError as exc:
            print(f"[error] required gate ref build failed for {check_id}: {exc}", file=sys.stderr)
            return 2

    gate_witness_refs: List[Dict[str, Any]] = []
    for idx, check_row in enumerate(results):
        native_ref = check_row.pop("nativeGateWitnessRef", None)
        if isinstance(native_ref, dict):
            gate_witness_refs.append(native_ref)
            continue

        try:
            gate_witness_refs.append(
                emit_gate_witness(
                    root=root,
                    out_dir=out_dir,
                    projection_digest=plan["projectionDigest"],
                    policy_digest=plan["projectionPolicy"],
                    from_ref=from_ref,
                    to_ref=to_ref,
                    check_row=check_row,
                    index=idx,
                )
            )
        except ValueError as exc:
            print(
                f"[error] required gate fallback synthesis failed for {check_row.get('checkId')}: {exc}",
                file=sys.stderr,
            )
            return 2

    failed = [row for row in results if row["exitCode"] != 0]

    finished_at = datetime.now(timezone.utc)
    runtime_payload = {
        "projectionPolicy": plan["projectionPolicy"],
        "projectionDigest": plan["projectionDigest"],
        "changedPaths": plan["changedPaths"],
        "requiredChecks": list(required_checks),
        "results": results,
        "gateWitnessRefs": gate_witness_refs,
        "docsOnly": plan["docsOnly"],
        "reasons": plan["reasons"],
        "deltaSource": plan["deltaSource"],
        "fromRef": plan["fromRef"],
        "toRef": plan["toRef"],
        "policyDigest": plan["projectionPolicy"],
        "squeakSiteProfile": os.environ.get(
            "PREMATH_SQUEAK_SITE_PROFILE",
            os.environ.get("PREMATH_EXECUTOR_PROFILE", "local"),
        ),
        "runStartedAt": started_at.isoformat(),
        "runFinishedAt": finished_at.isoformat(),
        "runDurationMs": int((finished_at - started_at).total_seconds() * 1000),
    }
    try:
        witness = run_required_witness(root, runtime_payload)
    except RequiredWitnessError as exc:
        print(
            f"[error] required witness build failed: {exc.failure_class}: {exc.reason}",
            file=sys.stderr,
        )
        return 2

    out_path = out_dir / f"{plan['projectionDigest']}.json"
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(witness, f, indent=2, ensure_ascii=False)
        f.write("\n")
    latest_path = out_dir / "latest-required.json"
    with latest_path.open("w", encoding="utf-8") as f:
        json.dump(witness, f, indent=2, ensure_ascii=False)
        f.write("\n")

    if required_checks:
        print(
            "[ci-required] summary: "
            f"checks={len(required_checks)} failed={len(failed)} "
            f"projection={plan['projectionDigest']}"
        )
    else:
        print(f"[ci-required] summary: no required checks (projection={plan['projectionDigest']})")
    print(f"[ci-required] witness written: {out_path}")
    print(f"[ci-required] latest witness: {latest_path}")
    print(f"[ci-required] latest delta: {delta_snapshot_path}")

    if witness.get("verdictClass") == "rejected" and not args.allow_failure:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
