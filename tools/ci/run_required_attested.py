#!/usr/bin/env python3
"""Run the ci.required attestation chain without a task-runner dependency."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Sequence


def _run(cmd: Sequence[str], *, cwd: Path) -> int:
    completed = subprocess.run(cmd, cwd=cwd)
    return int(completed.returncode)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    commands: tuple[tuple[str, ...], ...] = (
        ("python3", "tools/ci/run_required_checks.py"),
        ("python3", "tools/ci/verify_required_witness.py", "--compare-delta"),
        (
            "python3",
            "tools/ci/decide_required.py",
            "--compare-delta",
            "--out",
            "artifacts/ciwitness/latest-decision.json",
        ),
        ("python3", "tools/ci/verify_decision.py"),
    )
    for cmd in commands:
        exit_code = _run(cmd, cwd=repo_root)
        if exit_code != 0:
            return exit_code
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
