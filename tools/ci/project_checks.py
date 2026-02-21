#!/usr/bin/env python3
"""Project changed paths into required CI checks."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from change_projection import (
    detect_changed_paths,
    normalize_paths,
    project_required_checks,
    projection_plan_payload,
)


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Compute required CI checks from a change set.")
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
        "--repo-root",
        type=Path,
        default=root,
        help=f"Repository root (default: {root})",
    )
    parser.add_argument(
        "--checks-only",
        action="store_true",
        help="Print one required check per line instead of JSON.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()

    if args.changed_file:
        changed_paths = normalize_paths(args.changed_file)
        source = "explicit"
        from_ref = args.from_ref
        to_ref = args.to_ref
    else:
        detected = detect_changed_paths(repo_root, from_ref=args.from_ref, to_ref=args.to_ref)
        changed_paths = detected.changed_paths
        source = detected.source
        from_ref = detected.from_ref
        to_ref = detected.to_ref

    projection = project_required_checks(changed_paths)
    payload = projection_plan_payload(projection, source, from_ref, to_ref)

    if args.checks_only:
        for check_id in payload["requiredChecks"]:
            print(check_id)
        return 0

    json.dump(payload, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
