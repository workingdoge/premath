#!/usr/bin/env python3
"""Integration tests for `premath repo-hygiene-check`."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path


def run_repo_hygiene(repo_root: Path, *paths: str) -> subprocess.CompletedProcess[str]:
    command = [
        "cargo",
        "run",
        "--quiet",
        "--package",
        "premath-cli",
        "--",
        "repo-hygiene-check",
        "--repo-root",
        str(repo_root),
    ]
    command.extend(paths)
    command.append("--json")
    return subprocess.run(
        command,
        cwd=Path(__file__).resolve().parents[2],
        text=True,
        capture_output=True,
        check=False,
    )


class RepoHygieneTests(unittest.TestCase):
    def test_rejects_private_surface_path(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-repo-hygiene-") as tmp:
            repo_root = Path(tmp)
            (repo_root / ".gitignore").write_text(
                "\n".join([".claude/", ".serena/", ".premath/cache/"]) + "\n",
                encoding="utf-8",
            )
            completed = run_repo_hygiene(repo_root, ".claude/session.json")
            self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
            payload = json.loads(completed.stdout)
            self.assertEqual(payload["result"], "rejected")
            self.assertIn("repo_hygiene_violation", payload["failureClasses"])
            self.assertTrue(
                any("private_agent_surface" in row for row in payload["violations"]),
                payload["violations"],
            )

    def test_rejects_missing_gitignore_entries(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-repo-hygiene-") as tmp:
            repo_root = Path(tmp)
            (repo_root / ".gitignore").write_text(".claude/\n", encoding="utf-8")
            completed = run_repo_hygiene(repo_root, "specs/premath/draft/README.md")
            self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
            payload = json.loads(completed.stdout)
            self.assertEqual(payload["result"], "rejected")
            self.assertTrue(
                any(".premath/cache/" in row for row in payload["violations"]),
                payload["violations"],
            )
            self.assertTrue(
                any(".serena/" in row for row in payload["violations"]),
                payload["violations"],
            )

    def test_accepts_clean_explicit_paths(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-repo-hygiene-") as tmp:
            repo_root = Path(tmp)
            (repo_root / ".gitignore").write_text(
                "\n".join([".claude/", ".serena/", ".premath/cache/"]) + "\n",
                encoding="utf-8",
            )
            completed = run_repo_hygiene(repo_root, "specs/premath/draft/README.md")
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            payload = json.loads(completed.stdout)
            self.assertEqual(payload["result"], "accepted")
            self.assertEqual(payload["violations"], [])


if __name__ == "__main__":
    unittest.main()
