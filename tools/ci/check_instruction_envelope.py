#!/usr/bin/env python3
"""Validate instruction envelopes via core `premath instruction-check`."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import tempfile
from typing import Any, Dict, List

from instruction_check_client import InstructionCheckError, run_instruction_check


DEFAULT_GLOBS = (
    "instructions/*.json",
    "tests/ci/fixtures/instructions/*.json",
)
def validate_envelope(path: Path, payload: Dict[str, Any], repo_root: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="premath-instruction-envelope-check-") as tmp:
        tmp_path = Path(tmp) / path.name
        tmp_path.write_text(
            json.dumps(payload, ensure_ascii=False),
            encoding="utf-8",
        )
        try:
            run_instruction_check(repo_root, tmp_path)
        except InstructionCheckError as exc:
            raise ValueError(str(exc)) from exc


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check instruction envelope schema/shape.")
    parser.add_argument(
        "paths",
        nargs="*",
        help="Instruction files to validate. If omitted, validates default instruction globs.",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    return parser.parse_args()


def resolve_paths(repo_root: Path, args_paths: List[str]) -> List[Path]:
    files: List[Path] = []
    if args_paths:
        for raw in args_paths:
            path = Path(raw)
            if not path.is_absolute():
                path = (repo_root / path).resolve()
            files.append(path)
    else:
        for pattern in DEFAULT_GLOBS:
            files.extend(sorted((repo_root).glob(pattern)))
    return files


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()
    files = resolve_paths(root, args.paths)

    if not files:
        print("[instruction-check] FAIL (no instruction envelopes found)")
        return 1

    errors: List[str] = []
    checked = 0
    for path in files:
        checked += 1
        if not path.exists() or not path.is_file():
            errors.append(f"{path}: file not found")
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("root must be a JSON object")
            validate_envelope(path, payload, root)
        except Exception as exc:
            errors.append(f"{path}: {exc}")

    if errors:
        print(f"[instruction-check] FAIL (checked={checked}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[instruction-check] OK (checked={checked})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
