#!/usr/bin/env python3
"""Unit tests for deterministic repo hygiene checks."""

from __future__ import annotations

import unittest

import check_repo_hygiene


class RepoHygieneTests(unittest.TestCase):
    def test_classify_forbidden_path_prefixes(self) -> None:
        self.assertEqual(
            check_repo_hygiene.classify_forbidden_path(".claude/session.json"),
            "private_agent_surface",
        )
        self.assertEqual(
            check_repo_hygiene.classify_forbidden_path(".serena/memory.md"),
            "private_agent_surface",
        )
        self.assertEqual(
            check_repo_hygiene.classify_forbidden_path(".premath/cache/conformance/cache.json"),
            "local_cache_surface",
        )
        self.assertEqual(
            check_repo_hygiene.classify_forbidden_path("artifacts/ciwitness/latest-required.json"),
            "ephemeral_ci_artifact_surface",
        )
        self.assertIsNone(check_repo_hygiene.classify_forbidden_path("specs/premath/draft/README.md"))

    def test_missing_required_gitignore_entries(self) -> None:
        text = """
        .DS_Store
        .claude/
        # comment
        """
        missing = check_repo_hygiene.missing_required_gitignore_entries(text)
        self.assertEqual(missing, [".premath/cache/", ".serena/"])

    def test_check_paths_reports_forbidden_entries(self) -> None:
        violations = check_repo_hygiene.check_paths(
            [
                "specs/premath/draft/CONFORMANCE.md",
                ".serena/memory.md",
            ]
        )
        self.assertEqual(len(violations), 1)
        self.assertIn("private_agent_surface", violations[0])


if __name__ == "__main__":
    unittest.main()
