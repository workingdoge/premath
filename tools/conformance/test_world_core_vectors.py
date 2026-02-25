#!/usr/bin/env python3
"""Sentinel tests for world-core runner kernel-authority behavior."""

from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import patch

import run_world_core_vectors as world_core


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
            world_core.check_runtime_orchestration,
            "_run_kernel_world_registry_check",
            return_value={"result": "accepted", "failureClasses": []},
        ):
            with patch.object(
                world_core.check_runtime_orchestration,
                "evaluate_runtime_orchestration",
                side_effect=AssertionError("adapter semantics must not be invoked"),
            ):
                evaluated = world_core.evaluate_world_registry_vector(case)

        self.assertEqual(evaluated.result, "accepted")
        self.assertEqual(evaluated.failure_classes, [])

    def test_world_registry_vector_fails_closed_on_malformed_core_payload(self) -> None:
        case = self._load_world_registry_case()

        with patch.object(
            world_core.check_runtime_orchestration,
            "_run_kernel_world_registry_check",
            return_value={"failureClasses": []},
        ):
            with self.assertRaisesRegex(ValueError, "core.result"):
                world_core.evaluate_world_registry_vector(case)


if __name__ == "__main__":
    unittest.main()
