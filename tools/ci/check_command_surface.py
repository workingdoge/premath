#!/usr/bin/env python3
"""Guard the repository command surface: mise-only, no just references."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path
from typing import Iterable, List, Tuple


INLINE_JUST_RE = re.compile(r"`just\s+[^`]+`")
NIX_JUST_RE = re.compile(r"\bnix\s+develop\s+-c\s+just\b")
RUN_JUST_RE = re.compile(r"\brun:\s*just\s+\S+")
JUSTFILE_WORD_RE = re.compile(r"\bjustfile\b", re.IGNORECASE)


def list_repo_files(repo_root: Path) -> List[Path]:
    ls_files_args = ["ls-files", "--cached", "--others", "--exclude-standard"]
    proc: subprocess.CompletedProcess[str]
    try:
        proc = subprocess.run(
            ["git", *ls_files_args],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as first_error:
        # Bare-repository setups require explicit git-dir/work-tree pairing.
        proc = subprocess.run(
            ["git", "--git-dir=.git", "--work-tree=.", *ls_files_args],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    files: List[Path] = []
    for raw in proc.stdout.splitlines():
        if not raw.strip():
            continue
        files.append((repo_root / raw.strip()).resolve())
    return files


def find_violations(path: Path) -> Iterable[Tuple[int, str]]:
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return []

    violations: List[Tuple[int, str]] = []
    for idx, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip()
        if stripped.startswith("just ") or stripped.startswith("$ just "):
            violations.append((idx, "command-style `just ...` usage"))
            continue
        if INLINE_JUST_RE.search(line):
            violations.append((idx, "inline backtick `just ...` usage"))
            continue
        if NIX_JUST_RE.search(line):
            violations.append((idx, "`nix develop -c just ...` usage"))
            continue
        if RUN_JUST_RE.search(line):
            violations.append((idx, "workflow/task `run: just ...` usage"))
            continue
        if JUSTFILE_WORD_RE.search(line):
            violations.append((idx, "`justfile` reference"))
            continue
    return violations


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    violations: List[str] = []

    justfile = repo_root / "justfile"
    if justfile.exists():
        violations.append(f"{justfile}: expected removed (mise-only command surface)")

    self_path = Path(__file__).resolve()
    for path in list_repo_files(repo_root):
        if path.resolve() == self_path:
            continue
        if not path.is_file():
            continue
        for line_no, reason in find_violations(path):
            rel = path.relative_to(repo_root)
            violations.append(f"{rel}:{line_no}: {reason}")

    if violations:
        print(f"[command-surface] FAIL (violations={len(violations)})")
        for row in violations:
            print(f"  - {row}")
        return 1

    print("[command-surface] OK (mise-only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
