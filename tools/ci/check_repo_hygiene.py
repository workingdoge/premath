#!/usr/bin/env python3
"""Fail closed on tracked/staged private or local-only repository surfaces."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path
from typing import Iterable, List, Sequence, Tuple


FORBIDDEN_PREFIX_REASONS: Tuple[Tuple[str, str], ...] = (
    (".claude/", "private_agent_surface"),
    (".serena/", "private_agent_surface"),
    (".premath/cache/", "local_cache_surface"),
    (".premath/sessions/", "local_runtime_surface"),
    ("artifacts/ciwitness/", "ephemeral_ci_artifact_surface"),
    ("artifacts/observation/", "ephemeral_ci_artifact_surface"),
)

REQUIRED_GITIGNORE_ENTRIES: Tuple[str, ...] = (
    ".claude/",
    ".serena/",
    ".premath/cache/",
)


def _normalize_path(path: str) -> str:
    normalized = str(path).strip().replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def classify_forbidden_path(path: str) -> str | None:
    normalized = _normalize_path(path)
    if not normalized:
        return None
    for prefix, reason in FORBIDDEN_PREFIX_REASONS:
        anchor = prefix.rstrip("/")
        if normalized == anchor or normalized.startswith(prefix):
            return reason
    return None


def parse_gitignore_entries(text: str) -> set[str]:
    entries: set[str] = set()
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        entries.add(line)
    return entries


def missing_required_gitignore_entries(text: str) -> List[str]:
    entries = parse_gitignore_entries(text)
    return sorted(entry for entry in REQUIRED_GITIGNORE_ENTRIES if entry not in entries)


def _list_tracked_paths(repo_root: Path) -> List[str]:
    proc = subprocess.run(
        ["git", "ls-files", "--cached", "-z"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    if not proc.stdout:
        return []
    return [_normalize_path(item) for item in proc.stdout.decode("utf-8").split("\0") if item]


def check_paths(paths: Iterable[str]) -> List[str]:
    violations: List[str] = []
    for path in sorted({_normalize_path(path) for path in paths if _normalize_path(path)}):
        reason = classify_forbidden_path(path)
        if reason is not None:
            violations.append(f"{path}: {reason}")
    return violations


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Reject private/local-only tracked or staged surfaces.",
    )
    parser.add_argument(
        "paths",
        nargs="*",
        help="Optional paths to check (default: full tracked tree from git index).",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="Repository root (default: script parent repo).",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()

    if args.paths:
        scan_paths = [_normalize_path(path) for path in args.paths if _normalize_path(path)]
        source = "explicit_paths"
    else:
        scan_paths = _list_tracked_paths(repo_root)
        source = "git_index"

    violations = check_paths(scan_paths)

    gitignore_path = repo_root / ".gitignore"
    if not gitignore_path.exists():
        violations.append(".gitignore: missing required file")
    else:
        missing = missing_required_gitignore_entries(gitignore_path.read_text(encoding="utf-8"))
        for entry in missing:
            violations.append(f".gitignore: missing required ignore entry {entry!r}")

    if violations:
        print(f"[repo-hygiene] FAIL (source={source}, violations={len(violations)})")
        for row in violations:
            print(f"  - {row}")
        return 1

    print(f"[repo-hygiene] OK (source={source}, scanned={len(scan_paths)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
