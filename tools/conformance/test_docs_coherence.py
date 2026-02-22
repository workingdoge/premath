#!/usr/bin/env python3
"""Unit tests for docs coherence checker parser helpers."""

from __future__ import annotations

import unittest

import check_docs_coherence


class DocsCoherenceParserTests(unittest.TestCase):
    def test_parse_symbol_tuple_values(self) -> None:
        text = """
CAPABILITY_A = "capabilities.alpha"
CAPABILITY_B = "capabilities.beta"

DEFAULT_EXECUTABLE_CAPABILITIES = (
    CAPABILITY_A,
    CAPABILITY_B,
)
"""
        values = check_docs_coherence.parse_symbol_tuple_values(
            text,
            check_docs_coherence.CAP_ASSIGN_RE,
            "DEFAULT_EXECUTABLE_CAPABILITIES",
        )
        self.assertEqual(values, ["capabilities.alpha", "capabilities.beta"])

    def test_extract_section_between(self) -> None:
        text = "prefix START body END suffix"
        self.assertEqual(
            check_docs_coherence.extract_section_between(text, "START", "END").strip(),
            "body",
        )

    def test_parse_mise_task_commands(self) -> None:
        text = """
[tasks.baseline]
run = [
  "mise run fmt",
  "mise run test",
]

[tasks.other]
run = "echo ok"
"""
        commands = check_docs_coherence.parse_mise_task_commands(text, "baseline")
        self.assertEqual(commands, ["mise run fmt", "mise run test"])
        task_ids = check_docs_coherence.parse_baseline_task_ids_from_commands(commands)
        self.assertEqual(task_ids, ["fmt", "test"])

    def test_conditional_normative_entry(self) -> None:
        section = """
- `raw/SQUEAK-SITE` — runtime-location site contracts
  (normative only when `capabilities.squeak_site` is claimed).
"""
        self.assertTrue(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/SQUEAK-SITE",
                "capabilities.squeak_site",
            )
        )
        self.assertFalse(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/PREMATH-CI",
                "capabilities.ci_witnesses",
            )
        )


if __name__ == "__main__":
    unittest.main()
