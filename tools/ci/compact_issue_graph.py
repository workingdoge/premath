#!/usr/bin/env python3
"""Deterministic issue-graph compactness helper (check/apply)."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from check_issue_graph import evaluate_compactness_findings, print_compactness_findings


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description="Check/apply deterministic issue-graph compactness repairs."
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
        "--mode",
        choices=("check", "apply"),
        default="check",
        help="check = report compactness drift; apply = remove redundant blocks edges.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit structured JSON output.",
    )
    return parser.parse_args()


def _resolve_issues_path(repo_root: Path, issues: Path) -> Path:
    if issues.is_absolute():
        return issues
    return (repo_root / issues).resolve()


def _run_premath_json(repo_root: Path, args: list[str]) -> dict[str, Any]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "premath-cli",
        "--",
        *args,
        "--json",
    ]
    completed = subprocess.run(
        command,
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        stderr = completed.stderr.strip()
        stdout = completed.stdout.strip()
        detail = stderr or stdout or "unknown error"
        raise RuntimeError(f"premath command failed: {' '.join(args)} ({detail})")
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"premath command produced invalid JSON for {' '.join(args)}: {exc}"
        ) from exc


def _snapshot_semantics(repo_root: Path, issues_path: Path) -> dict[str, Any]:
    ready = _run_premath_json(
        repo_root, ["issue", "ready", "--issues", str(issues_path)]
    )
    blocked = _run_premath_json(
        repo_root, ["issue", "blocked", "--issues", str(issues_path)]
    )
    diagnostics = _run_premath_json(
        repo_root,
        [
            "dep",
            "diagnostics",
            "--issues",
            str(issues_path),
            "--graph-scope",
            "active",
        ],
    )
    ready_ids = sorted(
        item.get("id", "") for item in ready.get("items", []) if item.get("id")
    )
    blocked_ids = sorted(
        item.get("id", "") for item in blocked.get("items", []) if item.get("id")
    )
    has_cycle = bool(diagnostics.get("integrity", {}).get("hasCycle", False))
    cycle_path = diagnostics.get("integrity", {}).get("cyclePath")
    return {
        "readyIds": ready_ids,
        "blockedIds": blocked_ids,
        "hasCycle": has_cycle,
        "cyclePath": cycle_path,
    }


def _compactness_edge_set(findings: list[dict[str, Any]]) -> list[tuple[str, str]]:
    edges = {
        (str(item["issueId"]), str(item["dependsOnId"]))
        for item in findings
        if item.get("class", "").startswith("issue_graph.compactness.")
    }
    return sorted(edges)


def _remove_blocks_edge(repo_root: Path, issues_path: Path, issue_id: str, depends_on_id: str) -> None:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "premath-cli",
        "--",
        "dep",
        "remove",
        issue_id,
        depends_on_id,
        "--type",
        "blocks",
        "--issues",
        str(issues_path),
    ]
    completed = subprocess.run(
        command,
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        stderr = completed.stderr.strip()
        stdout = completed.stdout.strip()
        detail = stderr or stdout or "unknown error"
        raise RuntimeError(
            f"failed to remove edge {issue_id}->{depends_on_id}: {detail}"
        )


def _print_apply_summary(removed_edges: list[tuple[str, str]]) -> None:
    print(f"[issue-graph-compact] APPLY (removed={len(removed_edges)})")
    for issue_id, depends_on_id in removed_edges:
        print(f"  - removed blocks edge: {issue_id} -> {depends_on_id}")


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    issues_path = _resolve_issues_path(repo_root, args.issues)
    findings = evaluate_compactness_findings(issues_path)

    if args.mode == "check":
        if args.json:
            print(
                json.dumps(
                    {
                        "action": "issue_graph.compactness",
                        "mode": "check",
                        "issuesPath": str(issues_path),
                        "findingCount": len(findings),
                        "findings": findings,
                        "result": "accepted" if not findings else "rejected",
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            if not findings:
                print("[issue-graph-compact] OK (no compactness drift)")
            else:
                print_compactness_findings(findings)
        return 1 if findings else 0

    # apply mode
    edges = _compactness_edge_set(findings)
    if not edges:
        if args.json:
            print(
                json.dumps(
                    {
                        "action": "issue_graph.compactness",
                        "mode": "apply",
                        "issuesPath": str(issues_path),
                        "removedCount": 0,
                        "removedEdges": [],
                        "result": "accepted",
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print("[issue-graph-compact] APPLY (removed=0)")
        return 0

    before = _snapshot_semantics(repo_root, issues_path)
    for issue_id, depends_on_id in edges:
        _remove_blocks_edge(repo_root, issues_path, issue_id, depends_on_id)
    after = _snapshot_semantics(repo_root, issues_path)
    after_findings = evaluate_compactness_findings(issues_path)

    semantic_mismatch = (
        before["readyIds"] != after["readyIds"]
        or before["blockedIds"] != after["blockedIds"]
    )
    cycle_regression = (not before["hasCycle"]) and after["hasCycle"]
    if semantic_mismatch or cycle_regression or after_findings:
        mismatch_payload = {
            "action": "issue_graph.compactness",
            "mode": "apply",
            "issuesPath": str(issues_path),
            "removedEdges": [{"issueId": a, "dependsOnId": b} for a, b in edges],
            "before": before,
            "after": after,
            "residualFindings": after_findings,
            "result": "rejected",
        }
        if args.json:
            print(json.dumps(mismatch_payload, indent=2, sort_keys=True))
        else:
            print("[issue-graph-compact] FAIL (apply invariant mismatch)")
            print(json.dumps(mismatch_payload, indent=2, sort_keys=True))
        return 1

    if args.json:
        print(
            json.dumps(
                {
                    "action": "issue_graph.compactness",
                    "mode": "apply",
                    "issuesPath": str(issues_path),
                    "removedCount": len(edges),
                    "removedEdges": [
                        {"issueId": issue_id, "dependsOnId": depends_on_id}
                        for issue_id, depends_on_id in edges
                    ],
                    "before": before,
                    "after": after,
                    "result": "accepted",
                },
                indent=2,
                sort_keys=True,
            )
        )
    else:
        _print_apply_summary(edges)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
