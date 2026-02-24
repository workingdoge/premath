#!/usr/bin/env python3
"""Integration tests for issue-graph checker command surfaces."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Any


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    payload = "\n".join(json.dumps(row) for row in rows) + "\n"
    path.write_text(payload, encoding="utf-8")


def run_wrapper(issues_path: Path, note_warn_threshold: int = 2000) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--package",
            "premath-cli",
            "--",
            "issue-graph-check",
            "--repo-root",
            str(repo_root()),
            "--issues",
            str(issues_path),
            "--note-warn-threshold",
            str(note_warn_threshold),
        ],
        cwd=repo_root(),
        text=True,
        capture_output=True,
        check=False,
    )


def run_compact_helper(
    issues_path: Path, mode: str = "check", json_output: bool = False
) -> subprocess.CompletedProcess[str]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "premath-cli",
        "--",
        "issue-graph-compact",
        "--repo-root",
        str(repo_root()),
        "--issues",
        str(issues_path),
        "--mode",
        mode,
    ]
    if json_output:
        command.append("--json")
    return subprocess.run(
        command,
        cwd=repo_root(),
        text=True,
        capture_output=True,
        check=False,
    )


def active_issue_description() -> str:
    return (
        "Acceptance:\n- complete work\n\nVerification commands:\n"
        "- `cargo run --package premath-cli -- issue-graph-check --repo-root . --issues .premath/issues.jsonl`\n"
    )


class IssueGraphWrapperTests(unittest.TestCase):
    def test_epic_title_requires_epic_issue_type(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-epic",
                        "title": "[EPIC] Example",
                        "issue_type": "task",
                        "status": "open",
                        "description": "Acceptance:\n- ok\n\nVerification commands:\n- `mise run baseline`",
                    }
                ],
            )
            completed = run_wrapper(issues)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("issue_graph.issue_type.epic_mismatch", completed.stdout)

    def test_active_issue_accepts_acceptance_and_verification_command(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-active",
                        "title": "Active issue",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                    }
                ],
            )
            completed = run_wrapper(issues)
            self.assertEqual(completed.returncode, 0)
            self.assertIn("[issue-graph] OK", completed.stdout)

    def test_note_length_emits_warning(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-note",
                        "title": "Note-heavy issue",
                        "issue_type": "task",
                        "status": "closed",
                        "notes": "x" * 12,
                    }
                ],
            )
            completed = run_wrapper(issues, note_warn_threshold=10)
            self.assertEqual(completed.returncode, 0)
            self.assertIn("issue_graph.notes.large", completed.stdout)

    def test_compactness_fails_on_active_to_closed_blocks_edge(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-a",
                        "title": "Issue A",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-b",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-b",
                        "title": "Issue B",
                        "issue_type": "task",
                        "status": "closed",
                    },
                ],
            )
            completed = run_wrapper(issues)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn(
                "issue_graph.compactness.closed_block_edge", completed.stdout
            )
            self.assertIn(
                "issue-graph-compact",
                completed.stdout,
            )

    def test_compactness_fails_on_transitive_redundant_blocks_edge(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-a",
                        "title": "Issue A",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-b",
                                "type": "blocks",
                            },
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-c",
                                "type": "blocks",
                            },
                        ],
                    },
                    {
                        "id": "bd-b",
                        "title": "Issue B",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-b",
                                "depends_on_id": "bd-c",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-c",
                        "title": "Issue C",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                    },
                ],
            )
            completed = run_wrapper(issues)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn(
                "issue_graph.compactness.transitive_block_edge", completed.stdout
            )
            self.assertIn("bd-a -> bd-c", completed.stdout)
            self.assertIn(
                "issue-graph-compact",
                completed.stdout,
            )

    def test_compactness_accepts_non_redundant_blocks_chain(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-a",
                        "title": "Issue A",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-b",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-b",
                        "title": "Issue B",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-b",
                                "depends_on_id": "bd-c",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-c",
                        "title": "Issue C",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                    },
                ],
            )
            completed = run_wrapper(issues)
            self.assertEqual(completed.returncode, 0)
            self.assertNotIn("compactness drift", completed.stdout)

    def test_compact_helper_check_reports_findings(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-compact-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-a",
                        "title": "Issue A",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-b",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-b",
                        "title": "Issue B",
                        "issue_type": "task",
                        "status": "closed",
                    },
                ],
            )
            completed = run_compact_helper(issues, mode="check")
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn(
                "issue_graph.compactness.closed_block_edge", completed.stdout
            )

    def test_compact_helper_apply_removes_redundant_edges(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-issue-graph-compact-test-") as tmp:
            issues = Path(tmp) / "issues.jsonl"
            write_jsonl(
                issues,
                [
                    {
                        "id": "bd-a",
                        "title": "Issue A",
                        "issue_type": "task",
                        "status": "open",
                        "description": active_issue_description(),
                        "dependencies": [
                            {
                                "issue_id": "bd-a",
                                "depends_on_id": "bd-b",
                                "type": "blocks",
                            }
                        ],
                    },
                    {
                        "id": "bd-b",
                        "title": "Issue B",
                        "issue_type": "task",
                        "status": "closed",
                    },
                ],
            )

            apply_completed = run_compact_helper(issues, mode="apply", json_output=True)
            self.assertEqual(apply_completed.returncode, 0, apply_completed.stderr)
            payload = json.loads(apply_completed.stdout)
            self.assertEqual(payload["removedCount"], 1)
            self.assertEqual(
                payload["removedEdges"][0],
                {"issueId": "bd-a", "dependsOnId": "bd-b"},
            )

            check_completed = run_wrapper(issues)
            self.assertEqual(check_completed.returncode, 0, check_completed.stdout)

            rows = [json.loads(line) for line in issues.read_text().splitlines() if line.strip()]
            issue_a = next(row for row in rows if row["id"] == "bd-a")
            self.assertEqual(issue_a.get("dependencies", []), [])


if __name__ == "__main__":
    unittest.main()
