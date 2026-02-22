#!/usr/bin/env python3
"""Thin CI wrapper for core issue-graph contract checks."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


DEFAULT_NOTE_WARN_THRESHOLD = 2000


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description="Run core issue-graph checks through `premath issue check`."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=root,
        help=f"Repository root (default: {root})",
    )
    parser.add_argument(
        "--issues",
        type=Path,
        default=Path(".premath/issues.jsonl"),
        help="Issue graph JSONL path relative to --repo-root (default: .premath/issues.jsonl).",
    )
    parser.add_argument(
        "--note-warn-threshold",
        type=int,
        default=DEFAULT_NOTE_WARN_THRESHOLD,
        help=f"Warning threshold for issue note length (default: {DEFAULT_NOTE_WARN_THRESHOLD}).",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.note_warn_threshold < 0:
        print("[issue-graph] FAIL (invalid --note-warn-threshold: must be >= 0)")
        return 1

    repo_root = args.repo_root.resolve()
    issues_path = (repo_root / args.issues).resolve()
    command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "premath-cli",
        "--",
        "issue",
        "check",
        "--issues",
        str(issues_path),
        "--note-warn-threshold",
        str(args.note_warn_threshold),
    ]
    completed = subprocess.run(
        command,
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.stdout:
        print(completed.stdout, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
