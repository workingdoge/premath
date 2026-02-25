#!/usr/bin/env python3
"""Sentinel tests for world-core runner kernel-authority behavior."""

from __future__ import annotations

import unittest
from pathlib import Path
import sys
from unittest.mock import patch

THIS_DIR = Path(__file__).resolve().parent
if str(THIS_DIR) not in sys.path:
    sys.path.insert(0, str(THIS_DIR))

import run_world_core_vectors as world_core
import wrapper_failure_guard


class WorldCoreVectorTests(unittest.TestCase):
    def _world_registry_case_path(self) -> Path:
        return (
            world_core.DEFAULT_FIXTURES
            / "golden"
            / "world_core_routes_bound_accept"
            / "case.json"
        )

    def _load_world_registry_case(self) -> dict:
        return world_core.load_json(self._world_registry_case_path())

    def test_world_registry_vector_uses_kernel_authority_only(self) -> None:
        case = self._load_world_registry_case()

        with patch.object(
            world_core.core_command_client,
            "run_world_registry_check",
            return_value={"result": "accepted", "failureClasses": []},
        ):
            evaluated = world_core.evaluate_world_registry_vector(case)

        self.assertEqual(evaluated.result, "accepted")
        self.assertEqual(evaluated.failure_classes, [])

    def test_world_registry_vector_fails_closed_on_malformed_core_payload(self) -> None:
        case = self._load_world_registry_case()

        with patch.object(
            world_core.core_command_client,
            "run_world_registry_check",
            return_value={"failureClasses": []},
        ):
            with self.assertRaisesRegex(ValueError, "core.result"):
                world_core.evaluate_world_registry_vector(case)

    def test_wrapper_failure_constants_stay_nonsemantic(self) -> None:
        failure_constants = [
            value
            for name, value in vars(world_core).items()
            if name.startswith("FAILURE_") and isinstance(value, str)
        ]
        wrapper_failure_guard.assert_nonsemantic_wrapper_failure_classes(
            wrapper_id="world-core-wrapper",
            failure_classes=failure_constants,
        )

    def test_wrapper_nonsemantic_guard_rejects_semantic_failure_class_prefixes(self) -> None:
        with self.assertRaisesRegex(ValueError, "wrapper non-semantic guard"):
            wrapper_failure_guard.assert_nonsemantic_wrapper_failure_classes(
                wrapper_id="world-core-wrapper",
                failure_classes=["site_resolve_ambiguous"],
            )


if __name__ == "__main__":
    unittest.main()
