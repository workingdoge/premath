#!/usr/bin/env python3
"""Integration tests for issue-graph CI wrapper delegation."""

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
    script = repo_root() / "tools" / "ci" / "check_issue_graph.py"
    return subprocess.run(
        [
            "python3",
            str(script),
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
                        "description": (
                            "Acceptance:\n- complete work\n\nVerification commands:\n"
                            "- `python3 tools/ci/check_issue_graph.py`\n"
                        ),
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


if __name__ == "__main__":
    unittest.main()
