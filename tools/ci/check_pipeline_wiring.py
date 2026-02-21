#!/usr/bin/env python3
"""Validate provider-neutral CI workflow pipeline entrypoints."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Iterable, List, Tuple


REQUIRED_BASELINE_COMMAND = "python3 tools/ci/pipeline_required.py"
REQUIRED_INSTRUCTION_COMMAND = "python3 tools/ci/pipeline_instruction.py --instruction \"$INSTRUCTION_PATH\""

BASELINE_REQUIRED_PATTERN = re.compile(
    r"^\s*run:\s*python3 tools/ci/pipeline_required.py\s*$",
    re.MULTILINE,
)
INSTRUCTION_REQUIRED_PATTERN = re.compile(
    r'^\s*run:\s*python3 tools/ci/pipeline_instruction.py --instruction "\$INSTRUCTION_PATH"\s*$',
    re.MULTILINE,
)

FORBIDDEN_PATTERNS: Tuple[Tuple[str, re.Pattern[str]], ...] = (
    ("legacy required gate task call", re.compile(r"^\s*run:\s*mise run ci-required-attested\s*$", re.MULTILINE)),
    ("legacy required gate split call", re.compile(r"^\s*run:\s*mise run ci-required\s*$", re.MULTILINE)),
    ("legacy strict verify call", re.compile(r"^\s*run:\s*mise run ci-verify-required-strict\s*$", re.MULTILINE)),
    ("legacy decision call", re.compile(r"^\s*run:\s*mise run ci-decide-required\s*$", re.MULTILINE)),
    ("legacy decision verify call", re.compile(r"^\s*run:\s*mise run ci-verify-decision\s*$", re.MULTILINE)),
    ("legacy provider env export call", re.compile(r"^\s*run:\s*python3 tools/ci/providers/export_github_env.py", re.MULTILINE)),
    ("legacy instruction check call", re.compile(r"^\s*run:\s*INSTRUCTION=.*mise run ci-instruction-check\s*$", re.MULTILINE)),
    ("legacy run_instruction shell call", re.compile(r"tools/ci/run_instruction.sh")),
    ("inline summary script block", re.compile(r"python3 - <<'PY'")),
)


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check CI workflow pipeline wiring.")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    return parser.parse_args()


def _check_required(
    text: str,
    label: str,
    required_label: str,
    required_command: str,
    required_pattern: re.Pattern[str],
) -> List[str]:
    errors: List[str] = []
    count = len(required_pattern.findall(text))
    if count == 0:
        errors.append(f"{label}: missing {required_label} `{required_command}`")
    elif count > 1:
        errors.append(f"{label}: expected exactly one `{required_command}`, found {count}")
    return errors


def _check_forbidden(text: str, label: str, forbidden: Iterable[Tuple[str, re.Pattern[str]]]) -> List[str]:
    errors: List[str] = []
    for reason, pattern in forbidden:
        if pattern.search(text):
            errors.append(f"{label}: forbidden {reason}")
    return errors


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()

    baseline = root / ".github/workflows/baseline.yml"
    instruction = root / ".github/workflows/instruction.yml"

    errors: List[str] = []
    if not baseline.exists():
        errors.append(f"missing workflow: {baseline}")
    if not instruction.exists():
        errors.append(f"missing workflow: {instruction}")

    if errors:
        print("[pipeline-wiring] FAIL")
        for err in errors:
            print(f"  - {err}")
        return 1

    baseline_text = baseline.read_text(encoding="utf-8")
    instruction_text = instruction.read_text(encoding="utf-8")

    errors.extend(
        _check_required(
            baseline_text,
            "baseline.yml",
            "required pipeline entrypoint",
            REQUIRED_BASELINE_COMMAND,
            BASELINE_REQUIRED_PATTERN,
        )
    )
    errors.extend(
        _check_required(
            instruction_text,
            "instruction.yml",
            "instruction pipeline entrypoint",
            REQUIRED_INSTRUCTION_COMMAND,
            INSTRUCTION_REQUIRED_PATTERN,
        )
    )

    errors.extend(_check_forbidden(baseline_text, "baseline.yml", FORBIDDEN_PATTERNS))
    errors.extend(_check_forbidden(instruction_text, "instruction.yml", FORBIDDEN_PATTERNS))

    if errors:
        print("[pipeline-wiring] FAIL")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[pipeline-wiring] OK "
        f"(baseline={REQUIRED_BASELINE_COMMAND}, instruction={REQUIRED_INSTRUCTION_COMMAND})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
