#!/usr/bin/env python3
"""Sentinel tests for runtime orchestration core-authority execution."""

from __future__ import annotations

import copy
import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

THIS_DIR = Path(__file__).resolve().parent
if str(THIS_DIR) not in sys.path:
    sys.path.insert(0, str(THIS_DIR))

import check_runtime_orchestration
import wrapper_failure_guard


class RuntimeOrchestrationCheckerTests(unittest.TestCase):
    def _baseline_inputs(self) -> tuple[dict, dict, str, dict]:
        control_plane_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"],
                    }
                }
            },
            "commandSurface": {
                "governancePromotionCheck": {
                    "canonicalEntrypoint": [
                        "cargo",
                        "run",
                        "--package",
                        "premath-cli",
                        "--",
                        "governance-promotion-check",
                    ]
                },
                "kcirMappingCheck": {
                    "canonicalEntrypoint": [
                        "cargo",
                        "run",
                        "--package",
                        "premath-cli",
                        "--",
                        "kcir-mapping-check",
                    ]
                },
            },
        }
        operation_registry = {
            "operations": [
                {
                    "id": "op/ci.run_gate",
                    "path": "tools/ci/run_gate.sh",
                    "morphisms": ["dm.identity"],
                }
            ]
        }
        harness_runtime_text = (
            "## 1.2 Harness-Squeak composition boundary (required)\n"
            "Harness computes deterministic work context and witness lineage refs.\n"
            "Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.\n"
            "Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.\n"
            "Harness records the resulting references in session/trajectory projections."
        )
        doctrine_site_input = {
            "worldRouteBindings": {
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [],
            }
        }
        return (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        )

    def test_runtime_orchestration_command_override_rejects_noncanonical_prefix(self) -> None:
        with patch.dict(
            os.environ,
            {
                "PREMATH_RUNTIME_ORCHESTRATION_CHECK_CMD": (
                    "python3 tools/conformance/run_runtime_orchestration_vectors.py"
                )
            },
            clear=False,
        ):
            with self.assertRaisesRegex(
                ValueError,
                "runtime-orchestration-check command surface drift",
            ):
                check_runtime_orchestration._resolve_runtime_orchestration_check_command()  # noqa: SLF001

    def test_evaluate_runtime_orchestration_fails_closed_on_malformed_core_payload(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_inputs()

        with patch.object(
            check_runtime_orchestration,
            "_run_kernel_runtime_orchestration_check",
            return_value={"failureClasses": []},
        ):
            with self.assertRaisesRegex(
                ValueError,
                "kernel.result must be 'accepted' or 'rejected'",
            ):
                check_runtime_orchestration.evaluate_runtime_orchestration(
                    control_plane_contract=control_plane_contract,
                    operation_registry=operation_registry,
                    harness_runtime_text=harness_runtime_text,
                    doctrine_site_input=doctrine_site_input,
                )

    def test_evaluate_runtime_orchestration_uses_core_authority(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_inputs()
        control_plane_contract = copy.deepcopy(control_plane_contract)
        control_plane_contract.pop("runtimeRouteBindings")
        operation_registry = {"operations": []}

        core_payload = {
            "schema": 1,
            "checkKind": "conformance.runtime_orchestration.v1",
            "result": "accepted",
            "failureClasses": [],
            "summary": {
                "requiredRoutes": 1,
                "checkedRoutes": 1,
                "checkedKcirMappingRows": 0,
                "checkedPhase3CommandSurfaces": 2,
                "checkedWorldRouteFamilies": 0,
                "errors": 0,
            },
            "routes": [],
            "kcirMappingRows": [],
            "phase3CommandSurfaces": [],
            "worldRouteBindings": [],
            "errors": [],
        }

        with patch.object(
            check_runtime_orchestration,
            "_run_kernel_runtime_orchestration_check",
            return_value=core_payload,
        ):
            payload = check_runtime_orchestration.evaluate_runtime_orchestration(
                control_plane_contract=control_plane_contract,
                operation_registry=operation_registry,
                harness_runtime_text=harness_runtime_text,
                doctrine_site_input=doctrine_site_input,
            )

        self.assertEqual(payload["result"], "accepted")
        self.assertEqual(payload["failureClasses"], [])

    def test_evaluate_runtime_orchestration_passes_optional_site_input(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_inputs()

        with patch.object(
            check_runtime_orchestration,
            "_run_kernel_runtime_orchestration_check",
            return_value={
                "schema": 1,
                "checkKind": "conformance.runtime_orchestration.v1",
                "result": "accepted",
                "failureClasses": [],
                "summary": {
                    "requiredRoutes": 1,
                    "checkedRoutes": 1,
                    "checkedKcirMappingRows": 0,
                    "checkedPhase3CommandSurfaces": 2,
                    "checkedWorldRouteFamilies": 0,
                    "errors": 0,
                },
                "routes": [],
                "kcirMappingRows": [],
                "phase3CommandSurfaces": [],
                "worldRouteBindings": [],
                "errors": [],
            },
        ) as patched:
            check_runtime_orchestration.evaluate_runtime_orchestration(
                control_plane_contract=control_plane_contract,
                operation_registry=operation_registry,
                harness_runtime_text=harness_runtime_text,
                doctrine_site_input=doctrine_site_input,
            )

        _, kwargs = patched.call_args
        self.assertEqual(kwargs["doctrine_site_input"], doctrine_site_input)

    def test_wrapper_failure_constants_stay_nonsemantic(self) -> None:
        failure_constants = [
            value
            for name, value in vars(check_runtime_orchestration).items()
            if name.startswith("FAILURE_") and isinstance(value, str)
        ]
        wrapper_failure_guard.assert_nonsemantic_wrapper_failure_classes(
            wrapper_id="runtime-orchestration-wrapper",
            failure_classes=failure_constants,
        )

    def test_wrapper_nonsemantic_guard_rejects_semantic_failure_class_prefixes(self) -> None:
        with self.assertRaisesRegex(ValueError, "wrapper non-semantic guard"):
            wrapper_failure_guard.assert_nonsemantic_wrapper_failure_classes(
                wrapper_id="runtime-orchestration-wrapper",
                failure_classes=["runtime_route_missing"],
            )


if __name__ == "__main__":
    unittest.main()
