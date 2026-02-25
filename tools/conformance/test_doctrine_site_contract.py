#!/usr/bin/env python3
"""Unit tests for doctrine-site generation and roundtrip checks."""

from __future__ import annotations

import copy
import json
import shutil
import unittest
from pathlib import Path

import doctrine_site_contract


ROOT = Path(__file__).resolve().parents[2]
INPUT_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json"
REGISTRY_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
TRACKED_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json"
PACKAGES_ROOT = ROOT / "specs" / "premath" / "site-packages"
CUTOVER_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE-CUTOVER.json"


class DoctrineSiteContractTests(unittest.TestCase):
    def test_generate_site_input_from_packages_matches_tracked_input(self) -> None:
        generated_input = doctrine_site_contract.generate_site_input_from_packages(
            repo_root=ROOT,
            packages_root=PACKAGES_ROOT,
        )
        tracked_input = doctrine_site_contract.load_json_object(INPUT_PATH)
        self.assertEqual(
            doctrine_site_contract.canonicalize_site_input(generated_input),
            doctrine_site_contract.canonicalize_site_input(tracked_input),
        )

    def test_site_input_equality_diff_detects_drift(self) -> None:
        generated_input = doctrine_site_contract.generate_site_input_from_packages(
            repo_root=ROOT,
            packages_root=PACKAGES_ROOT,
        )
        drifted = copy.deepcopy(generated_input)
        rows = drifted["worldRouteBindings"]["rows"]
        self.assertIsInstance(rows, list)
        rows[0]["worldId"] = f"{rows[0]['worldId']}.drift"
        errors = doctrine_site_contract.site_input_equality_diff(generated_input, drifted)
        self.assertGreaterEqual(len(errors), 1)

    def test_generate_site_input_from_packages_rejects_multiple_packages(self) -> None:
        package = doctrine_site_contract.load_json_object(
            PACKAGES_ROOT / "premath.doctrine_operation_site.v0" / "SITE-PACKAGE.json"
        )
        tmp_root = ROOT / "tmp" / "site-packages-multi"
        first = tmp_root / "a" / "SITE-PACKAGE.json"
        second = tmp_root / "b" / "SITE-PACKAGE.json"
        first.parent.mkdir(parents=True, exist_ok=True)
        second.parent.mkdir(parents=True, exist_ok=True)
        first.write_text(json.dumps(package, indent=2) + "\n", encoding="utf-8")
        second.write_text(json.dumps(package, indent=2) + "\n", encoding="utf-8")
        try:
            with self.assertRaises(ValueError):
                doctrine_site_contract.generate_site_input_from_packages(
                    repo_root=ROOT,
                    packages_root=tmp_root,
                )
        finally:
            shutil.rmtree(tmp_root, ignore_errors=True)

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

    def test_missing_operation_class_rejected(self) -> None:
        generated = doctrine_site_contract.generate_operation_registry(
            repo_root=ROOT,
            site_input_path=INPUT_PATH,
        )
        drifted = copy.deepcopy(generated)
        first = drifted["operations"][0]
        self.assertIn("operationClass", first)
        first.pop("operationClass")
        with self.assertRaises(ValueError):
            doctrine_site_contract.canonicalize_operation_registry(drifted)

    def test_route_bound_world_route_mismatch_rejected(self) -> None:
        source = doctrine_site_contract.load_json_object(INPUT_PATH)
        registry = copy.deepcopy(source["operationRegistry"])
        first_route = next(
            row for row in registry["operations"] if row["operationClass"] == "route_bound"
        )
        first_route["routeEligibility"]["routeFamilyId"] = "route.nonexistent"
        source["operationRegistry"] = registry

        tmp_path = ROOT / "tmp" / "doctrine-site-input-class-mismatch.json"
        tmp_path.parent.mkdir(parents=True, exist_ok=True)
        tmp_path.write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
        try:
            with self.assertRaises(ValueError):
                doctrine_site_contract.generate_operation_registry(
                    repo_root=ROOT,
                    site_input_path=tmp_path,
                )
        finally:
            tmp_path.unlink(missing_ok=True)

    def test_cutover_contract_generated_only_phase_disables_legacy_lanes(self) -> None:
        contract = doctrine_site_contract.load_cutover_contract(
            repo_root=ROOT,
            cutover_contract_path=CUTOVER_PATH,
        )
        phase = doctrine_site_contract.current_cutover_phase_policy(contract)
        self.assertFalse(bool(phase.get("allowLegacySourceKind")))
        self.assertFalse(bool(phase.get("allowOperationRegistryOverride")))

    def test_operation_registry_override_rejected_after_cutover(self) -> None:
        with self.assertRaises(ValueError):
            doctrine_site_contract.generate_operation_registry(
                repo_root=ROOT,
                site_input_path=INPUT_PATH,
                operation_registry_path=REGISTRY_PATH,
            )

    def test_legacy_source_kind_fallback_rejected_after_cutover(self) -> None:
        tracked_input = doctrine_site_contract.load_json_object(INPUT_PATH)
        legacy_source = copy.deepcopy(tracked_input["site"])
        legacy_source["schema"] = 1
        legacy_source["sourceKind"] = doctrine_site_contract.SITE_SOURCE_KIND
        legacy_source["operationRegistryPath"] = "tmp/legacy-operation-registry.json"

        tmp_root = ROOT / "tmp"
        tmp_root.mkdir(parents=True, exist_ok=True)
        legacy_source_path = tmp_root / "legacy-site-source.json"
        legacy_registry_path = tmp_root / "legacy-operation-registry.json"
        legacy_source_path.write_text(
            json.dumps(legacy_source, indent=2) + "\n",
            encoding="utf-8",
        )
        legacy_registry_path.write_text(
            json.dumps(tracked_input["operationRegistry"], indent=2) + "\n",
            encoding="utf-8",
        )
        try:
            with self.assertRaises(ValueError):
                doctrine_site_contract.generate_operation_registry(
                    repo_root=ROOT,
                    site_input_path=legacy_source_path,
                )
        finally:
            legacy_source_path.unlink(missing_ok=True)
            legacy_registry_path.unlink(missing_ok=True)

    def test_compatibility_phase_allows_legacy_paths_when_explicitly_selected(self) -> None:
        contract = doctrine_site_contract.load_json_object(CUTOVER_PATH)
        contract["currentPhaseId"] = "compatibility_window"
        tmp_contract_path = ROOT / "tmp" / "doctrine-site-cutover-compatibility.json"
        tmp_contract_path.parent.mkdir(parents=True, exist_ok=True)
        tmp_contract_path.write_text(
            json.dumps(contract, indent=2) + "\n",
            encoding="utf-8",
        )
        try:
            generated_registry = doctrine_site_contract.generate_operation_registry(
                repo_root=ROOT,
                site_input_path=INPUT_PATH,
                operation_registry_path=REGISTRY_PATH,
                cutover_contract_path=tmp_contract_path,
            )
            self.assertEqual(
                doctrine_site_contract.canonicalize_operation_registry(generated_registry),
                doctrine_site_contract.canonicalize_operation_registry(
                    doctrine_site_contract.load_json_object(REGISTRY_PATH)
                ),
            )
        finally:
            tmp_contract_path.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
