#!/usr/bin/env python3
"""Execute only required checks derived from deterministic change projection."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List

from change_projection import (
    detect_changed_paths,
    normalize_paths,
    project_required_checks,
    projection_plan_payload,
)


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


def run_check(root: Path, check_id: str) -> Dict[str, Any]:
    cmd = ["sh", str(root / "tools" / "ci" / "run_gate.sh"), check_id]
    started = time.perf_counter()
    completed = subprocess.run(cmd, cwd=root)
    duration_ms = int((time.perf_counter() - started) * 1000)
    exit_code = int(completed.returncode)
    return {
        "checkId": check_id,
        "status": "passed" if exit_code == 0 else "failed",
        "exitCode": exit_code,
        "durationMs": duration_ms,
    }


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

    results: List[Dict[str, Any]] = []
    for check_id in required_checks:
        print(f"[ci-required] running check: {check_id}")
        results.append(run_check(root, check_id))

    failed = [row for row in results if row["exitCode"] != 0]
    verdict_class = "accepted" if not failed else "rejected"
    failure_classes = ["check_failed"] if failed else []

    finished_at = datetime.now(timezone.utc)
    witness = {
        "ciSchema": 1,
        "witnessKind": "ci.required.v1",
        "projectionPolicy": plan["projectionPolicy"],
        "projectionDigest": plan["projectionDigest"],
        "changedPaths": plan["changedPaths"],
        "requiredChecks": required_checks,
        "executedChecks": [row["checkId"] for row in results],
        "results": results,
        "verdictClass": verdict_class,
        "failureClasses": failure_classes,
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

    out_dir = args.out_dir
    if not out_dir.is_absolute():
        out_dir = (root / out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
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

    if verdict_class == "rejected" and not args.allow_failure:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
