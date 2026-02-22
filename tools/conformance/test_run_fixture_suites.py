#!/usr/bin/env python3
"""Unit tests for conformance fixture suite cache bindings."""

from __future__ import annotations

import unittest

import run_fixture_suites


class RunFixtureSuitesTests(unittest.TestCase):
    def test_coherence_contract_input_closure_includes_surface_and_operation_paths(self) -> None:
        paths = set(run_fixture_suites.load_coherence_contract_input_paths())
        root = run_fixture_suites.ROOT

        self.assertIn(
            root / "specs" / "premath" / "draft" / "PREMATH-COHERENCE.md",
            paths,
        )
        self.assertIn(
            root / "specs" / "premath" / "draft" / "SPEC-INDEX.md",
            paths,
        )
        self.assertIn(
            root / "tests" / "conformance" / "fixtures" / "capabilities",
            paths,
        )
        self.assertIn(root / "tools" / "ci" / "run_gate.sh", paths)

    def test_coherence_contract_input_closure_is_duplicate_free(self) -> None:
        paths = run_fixture_suites.load_coherence_contract_input_paths()
        self.assertEqual(len(paths), len(set(paths)))


if __name__ == "__main__":
    unittest.main()
