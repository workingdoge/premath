#!/usr/bin/env python3
"""Unit tests for deterministic issue-graph contract checks."""

from __future__ import annotations

import unittest

import check_issue_graph


class IssueGraphTests(unittest.TestCase):
    def test_epic_title_requires_epic_issue_type(self) -> None:
        rows = [
            {
                "id": "bd-epic",
                "title": "[EPIC] Example",
                "issue_type": "task",
                "status": "open",
                "description": "Acceptance:\n- ok\n\nVerification commands:\n- `mise run baseline`",
            }
        ]
        errors, _warnings = check_issue_graph.evaluate_issue_graph(rows)
        self.assertTrue(any("epic_issue_type_mismatch" in item for item in errors))

    def test_active_issue_requires_acceptance_section(self) -> None:
        rows = [
            {
                "id": "bd-active",
                "title": "Active issue",
                "issue_type": "task",
                "status": "open",
                "description": "No acceptance section here.\nVerification commands:\n- `mise run baseline`",
            }
        ]
        errors, _warnings = check_issue_graph.evaluate_issue_graph(rows)
        self.assertTrue(any("issue_acceptance_missing" in item for item in errors))

    def test_active_issue_requires_verification_command(self) -> None:
        rows = [
            {
                "id": "bd-active",
                "title": "Active issue",
                "issue_type": "task",
                "status": "open",
                "description": "Acceptance:\n- do something",
            }
        ]
        errors, _warnings = check_issue_graph.evaluate_issue_graph(rows)
        self.assertTrue(any("issue_verification_command_missing" in item for item in errors))

    def test_active_issue_accepts_acceptance_and_verification_command(self) -> None:
        rows = [
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
        ]
        errors, warnings = check_issue_graph.evaluate_issue_graph(rows)
        self.assertEqual(errors, [])
        self.assertEqual(warnings, [])

    def test_closed_issue_is_exempt_from_active_contract(self) -> None:
        rows = [
            {
                "id": "bd-closed",
                "title": "Closed issue",
                "issue_type": "task",
                "status": "closed",
                "description": "",
            }
        ]
        errors, _warnings = check_issue_graph.evaluate_issue_graph(rows)
        self.assertEqual(errors, [])

    def test_note_length_emits_warning(self) -> None:
        rows = [
            {
                "id": "bd-note",
                "title": "Note-heavy issue",
                "issue_type": "task",
                "status": "closed",
                "description": "",
                "notes": "x" * 12,
            }
        ]
        errors, warnings = check_issue_graph.evaluate_issue_graph(rows, note_warn_threshold=10)
        self.assertEqual(errors, [])
        self.assertTrue(any("issue_notes_large" in item for item in warnings))


if __name__ == "__main__":
    unittest.main()
