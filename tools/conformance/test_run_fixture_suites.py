#!/usr/bin/env python3
"""Unit tests for conformance fixture suite cache bindings."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import run_fixture_suites


class RunFixtureSuitesTests(unittest.TestCase):
    def _suite_by_id(self, suite_id: str) -> run_fixture_suites.Suite:
        for suite in run_fixture_suites.SUITES:
            if suite.suite_id == suite_id:
                return suite
        self.fail(f"missing suite: {suite_id}")

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
        self.assertIn(root / "tools" / "ci" / "control_plane_contract.py", paths)
        self.assertIn(
            root / "specs" / "premath" / "profile" / "ADJOINTS-AND-SITES.md",
            paths,
        )
        self.assertIn(root / "crates" / "premath-kernel" / "src", paths)
        self.assertIn(root / "Cargo.toml", paths)
        self.assertIn(root / "Cargo.lock", paths)

    def test_coherence_contract_input_closure_is_duplicate_free(self) -> None:
        paths = run_fixture_suites.load_coherence_contract_input_paths()
        self.assertEqual(len(paths), len(set(paths)))

    def test_coherence_contract_cache_ref_changes_when_coherence_spec_changes(self) -> None:
        suite = self._suite_by_id("coherence-contract")
        root = run_fixture_suites.ROOT
        coherence_spec_path = root / "specs" / "premath" / "draft" / "PREMATH-COHERENCE.md"
        self.assertIn(coherence_spec_path, suite.input_paths)

        with tempfile.TemporaryDirectory(prefix="premath-suite-cache-") as tmp:
            temp_root = Path(tmp)
            cache_dir = temp_root / "cache"
            original_plan = run_fixture_suites.make_suite_plan(suite, cache_dir)

            mutated_spec_path = temp_root / "PREMATH-COHERENCE.mutated.md"
            original_text = coherence_spec_path.read_text(encoding="utf-8")
            mutated_spec_path.write_text(
                original_text + "\n<!-- cache-drift-test -->\n",
                encoding="utf-8",
            )

            mutated_suite = run_fixture_suites.Suite(
                suite_id=suite.suite_id,
                domain=suite.domain,
                command=suite.command,
                input_paths=tuple(
                    mutated_spec_path if path == coherence_spec_path else path
                    for path in suite.input_paths
                ),
            )
            mutated_plan = run_fixture_suites.make_suite_plan(mutated_suite, cache_dir)

            self.assertEqual(original_plan.params_hash, mutated_plan.params_hash)
            self.assertNotEqual(original_plan.material_digest, mutated_plan.material_digest)
            self.assertNotEqual(original_plan.cache_ref, mutated_plan.cache_ref)

    def _assert_manifest_vector_closure(self, suite_id: str) -> None:
        fixture_root = (
            run_fixture_suites.ROOT / "tests" / "conformance" / "fixtures" / suite_id
        )
        manifest = json.loads((fixture_root / "manifest.json").read_text(encoding="utf-8"))
        listed = set(manifest.get("vectors", []))

        actual: set[str] = set()
        for case_path in fixture_root.glob("*/*/case.json"):
            vector_id = case_path.parent.relative_to(fixture_root).as_posix()
            expect_path = case_path.parent / "expect.json"
            self.assertTrue(
                expect_path.exists(),
                f"{suite_id}: missing expect.json for {vector_id}",
            )
            actual.add(vector_id)

        missing = sorted(listed - actual)
        unexpected = sorted(actual - listed)
        self.assertEqual(
            [],
            missing,
            f"{suite_id}: manifest vectors missing fixture cases: {missing}",
        )
        self.assertEqual(
            [],
            unexpected,
            f"{suite_id}: found unlisted fixture vectors: {unexpected}",
        )

    def test_constructor_vector_suites_have_manifest_closure(self) -> None:
        for suite_id in ("runtime-orchestration", "frontend-parity", "world-core"):
            with self.subTest(suite_id=suite_id):
                self._assert_manifest_vector_closure(suite_id)


if __name__ == "__main__":
    unittest.main()
