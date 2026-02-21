#!/usr/bin/env python3
"""Validate CI workflow wiring invariants for the required gate chain."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


REQUIRED_COMMAND = "python3 tools/ci/pipeline_required.py"
REQUIRED_PATTERN = re.compile(
    r"^\s*run:\s*python3 tools/ci/pipeline_required.py\s*$",
    re.MULTILINE,
)
FORBIDDEN_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "legacy split-step required gate command",
        re.compile(r"^\s*run:\s*mise run ci-required\s*$", re.MULTILINE),
    ),
    (
        "legacy split-step strict verification command",
        re.compile(r"^\s*run:\s*mise run ci-verify-required-strict\s*$", re.MULTILINE),
    ),
    (
        "legacy split-step decision command",
        re.compile(r"^\s*run:\s*mise run ci-decide-required\s*$", re.MULTILINE),
    ),
    (
        "legacy split-step decision verification command",
        re.compile(r"^\s*run:\s*mise run ci-verify-decision\s*$", re.MULTILINE),
    ),
    (
        "legacy attested task workflow command",
        re.compile(r"^\s*run:\s*mise run ci-required-attested\s*$", re.MULTILINE),
    ),
)


def parse_args(default_workflow: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check CI workflow wiring invariants.")
    parser.add_argument(
        "--workflow",
        type=Path,
        default=default_workflow,
        help=f"Workflow file path (default: {default_workflow})",
    )
    return parser.parse_args()


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root / ".github/workflows/baseline.yml")
    workflow_path = args.workflow.resolve()

    if not workflow_path.exists():
        print(f"[error] workflow file not found: {workflow_path}")
        return 1

    text = workflow_path.read_text(encoding="utf-8")
    errors: list[str] = []

    required_count = len(REQUIRED_PATTERN.findall(text))
    if required_count == 0:
        errors.append(
            f"missing canonical gate command in workflow: `{REQUIRED_COMMAND}`"
        )
    elif required_count > 1:
        errors.append(
            f"expected exactly one canonical gate command `{REQUIRED_COMMAND}`, found {required_count}"
        )

    for label, pattern in FORBIDDEN_PATTERNS:
        if pattern.search(text):
            errors.append(f"forbidden {label} found")

    if errors:
        print(f"[ci-wiring] FAIL ({workflow_path})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        f"[ci-wiring] OK (workflow={workflow_path}, command={REQUIRED_COMMAND})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
