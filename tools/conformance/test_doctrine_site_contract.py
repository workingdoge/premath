#!/usr/bin/env python3
"""Unit tests for doctrine-site generation and roundtrip checks."""

from __future__ import annotations

import copy
import unittest
from pathlib import Path

import doctrine_site_contract


ROOT = Path(__file__).resolve().parents[2]
SOURCE_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE-SOURCE.json"
REGISTRY_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
TRACKED_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json"


class DoctrineSiteContractTests(unittest.TestCase):
    def test_generate_matches_tracked_map(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            source_map_path=SOURCE_PATH,
            operation_registry_path=REGISTRY_PATH,
        )
        tracked = doctrine_site_contract.load_json_object(TRACKED_PATH)
        self.assertEqual(
            doctrine_site_contract.canonicalize_site_map(generated),
            doctrine_site_contract.canonicalize_site_map(tracked),
        )

    def test_validate_generated_map_has_no_errors(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            source_map_path=SOURCE_PATH,
            operation_registry_path=REGISTRY_PATH,
        )
        errors = doctrine_site_contract.validate_site_map(repo_root=ROOT, site_map=generated)
        self.assertEqual(errors, [])

    def test_equality_diff_detects_drift(self) -> None:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=ROOT,
            source_map_path=SOURCE_PATH,
            operation_registry_path=REGISTRY_PATH,
        )
        drifted = copy.deepcopy(generated)
        edges = drifted["edges"]
        self.assertIsInstance(edges, list)
        removed = edges.pop()
        self.assertIsNotNone(removed)
        errors = doctrine_site_contract.equality_diff(generated, drifted)
        self.assertGreaterEqual(len(errors), 1)


if __name__ == "__main__":
    unittest.main()

