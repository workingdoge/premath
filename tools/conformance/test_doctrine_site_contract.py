#!/usr/bin/env python3
"""Unit tests for doctrine-site generation and roundtrip checks."""

from __future__ import annotations

import copy
import unittest
from pathlib import Path

import doctrine_site_contract


ROOT = Path(__file__).resolve().parents[2]
INPUT_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json"
REGISTRY_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
TRACKED_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json"


class DoctrineSiteContractTests(unittest.TestCase):
    def test_generate_matches_tracked_map(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        tracked = doctrine_site_contract.load_json_object(TRACKED_PATH)
        self.assertEqual(
            doctrine_site_contract.canonicalize_site_map(generated),
            doctrine_site_contract.canonicalize_site_map(tracked),
        )

    def test_generate_operation_registry_matches_tracked_registry(self) -> None:
        generated_registry = doctrine_site_contract.generate_operation_registry(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        tracked_registry = doctrine_site_contract.load_json_object(REGISTRY_PATH)
        self.assertEqual(
            doctrine_site_contract.canonicalize_operation_registry(generated_registry),
            doctrine_site_contract.canonicalize_operation_registry(tracked_registry),
        )

    def test_validate_generated_map_has_no_errors(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        errors = doctrine_site_contract.validate_site_map(repo_root=ROOT, site_map=generated)
        self.assertEqual(errors, [])

    def test_equality_diff_detects_drift(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        drifted = copy.deepcopy(generated)
        edges = drifted["edges"]
        self.assertIsInstance(edges, list)
        removed = edges.pop()
        self.assertIsNotNone(removed)
        errors = doctrine_site_contract.equality_diff(generated, drifted)
        self.assertGreaterEqual(len(errors), 1)

    def test_operation_registry_equality_diff_detects_drift(self) -> None:
        generated = doctrine_site_contract.generate_operation_registry(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        drifted = copy.deepcopy(generated)
        operations = drifted["operations"]
        self.assertIsInstance(operations, list)
        removed = operations.pop()
        self.assertIsNotNone(removed)
        errors = doctrine_site_contract.operation_registry_equality_diff(generated, drifted)
        self.assertGreaterEqual(len(errors), 1)


if __name__ == "__main__":
    unittest.main()
