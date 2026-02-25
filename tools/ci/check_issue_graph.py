#!/usr/bin/env python3
"""Thin CI wrapper for core issue-graph contract checks."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import deque
from pathlib import Path
from typing import Any


DEFAULT_NOTE_WARN_THRESHOLD = 2000
ACTIVE_STATUSES = {"open", "in_progress"}
BLOCKS_TYPE = "blocks"


class CompactnessFinding(dict):
    """Typed dict-like payload for deterministic compactness diagnostics."""


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


def _normalize_dep_type(raw: Any) -> str:
    if isinstance(raw, str):
        return raw.strip().lower()
    return ""


def _normalize_issue_status(raw: Any) -> str:
    if isinstance(raw, str):
        return raw.strip().lower()
    return ""


def _load_issue_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, start=1):
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            try:
                payload = json.loads(stripped)
            except json.JSONDecodeError as exc:
                raise ValueError(f"line {line_no}: invalid JSON: {exc}") from exc
            if not isinstance(payload, dict):
                raise ValueError(f"line {line_no}: issue row must be a JSON object")
            rows.append(payload)
    return rows


def _build_blocks_adjacency(rows: list[dict[str, Any]]) -> dict[str, set[str]]:
    adjacency: dict[str, set[str]] = {}
    for row in rows:
        issue_id = str(row.get("id", "")).strip()
        if not issue_id:
            continue
        deps = row.get("dependencies") or []
        if not isinstance(deps, list):
            continue
        for dep in deps:
            if not isinstance(dep, dict):
                continue
            if _normalize_dep_type(dep.get("type", dep.get("dep_type"))) != BLOCKS_TYPE:
                continue
            source = str(dep.get("issue_id", issue_id)).strip() or issue_id
            target = str(dep.get("depends_on_id", "")).strip()
            if not target:
                continue
            adjacency.setdefault(source, set()).add(target)
    return adjacency


def _find_path(
    adjacency: dict[str, set[str]],
    start: str,
    target: str,
) -> list[str] | None:
    if start == target:
        return [start]
    queue: deque[list[str]] = deque([[start]])
    visited = {start}
    while queue:
        path = queue.popleft()
        node = path[-1]
        for nxt in sorted(adjacency.get(node, set())):
            if nxt == target:
                return [*path, nxt]
            if nxt in visited:
                continue
            visited.add(nxt)
            queue.append([*path, nxt])
    return None


def evaluate_compactness_findings(issues_path: Path) -> list[CompactnessFinding]:
    rows = _load_issue_rows(issues_path)
    by_id: dict[str, dict[str, Any]] = {}
    for row in rows:
        issue_id = str(row.get("id", "")).strip()
        if issue_id:
            by_id[issue_id] = row

    adjacency = _build_blocks_adjacency(rows)
    findings: list[CompactnessFinding] = []

    for issue_id in sorted(by_id):
        row = by_id[issue_id]
        status = _normalize_issue_status(row.get("status"))
        if status not in ACTIVE_STATUSES:
            continue
        direct_targets = sorted(adjacency.get(issue_id, set()))
        if not direct_targets:
            continue

        for target_id in direct_targets:
            target_row = by_id.get(target_id)
            target_status = _normalize_issue_status(target_row.get("status") if target_row else "")
            if target_status == "closed":
                findings.append(
                    CompactnessFinding(
                        {
                            "class": "issue_graph.compactness.closed_block_edge",
                            "issueId": issue_id,
                            "dependsOnId": target_id,
                        }
                    )
                )

        for target_id in direct_targets:
            target_row = by_id.get(target_id)
            target_status = _normalize_issue_status(target_row.get("status") if target_row else "")
            if target_status == "closed":
                continue
            for candidate_start in direct_targets:
                if candidate_start == target_id:
                    continue
                path = _find_path(adjacency, candidate_start, target_id)
                if path is None:
                    continue
                findings.append(
                    CompactnessFinding(
                        {
                            "class": "issue_graph.compactness.transitive_block_edge",
                            "issueId": issue_id,
                            "dependsOnId": target_id,
                            "witnessPath": path,
                        }
                    )
                )
                break

    findings.sort(
        key=lambda item: (
            item["class"],
            item["issueId"],
            item["dependsOnId"],
            " -> ".join(item.get("witnessPath", [])),
        )
    )
    return findings


def print_compactness_findings(
    findings: list[CompactnessFinding],
    *,
    repo_root: Path | None = None,
    issues_path: Path | None = None,
) -> None:
    if not findings:
        return
    print(f"[issue-graph] FAIL (compactness drift: {len(findings)} finding(s))")
    for finding in findings:
        failure_class = finding["class"]
        issue_id = finding["issueId"]
        depends_on = finding["dependsOnId"]
        if "witnessPath" in finding:
            witness = " -> ".join(finding["witnessPath"])
            print(
                f"  - {failure_class} ({issue_id} -> {depends_on}, witness={witness})"
            )
        else:
            print(f"  - {failure_class} ({issue_id} -> {depends_on})")
    repo_root_arg = str(repo_root) if repo_root is not None else "."
    issues_arg = str(issues_path) if issues_path is not None else ".premath/issues.jsonl"
    print(
        "  remediation: "
        "python3 tools/ci/compact_issue_graph.py "
        f"--repo-root {repo_root_arg} --issues {issues_arg} --mode apply"
    )


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
    if completed.returncode != 0:
        return completed.returncode

    try:
        findings = evaluate_compactness_findings(issues_path)
    except ValueError as exc:
        print(f"[issue-graph] FAIL (compactness check parse error: {exc})")
        return 1
    print_compactness_findings(
        findings,
        repo_root=repo_root,
        issues_path=issues_path,
    )
    return 1 if findings else 0


if __name__ == "__main__":
    raise SystemExit(main())
