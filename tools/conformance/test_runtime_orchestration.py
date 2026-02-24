#!/usr/bin/env python3
"""Unit tests for runtime orchestration checker."""

from __future__ import annotations

import unittest

import check_runtime_orchestration


class RuntimeOrchestrationCheckerTests(unittest.TestCase):
    def test_accepts_when_routes_and_handoff_contract_match(self) -> None:
        control_plane_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                    },
                    "runGateTerraform": {
                        "operationId": "op/ci.run_gate_terraform",
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                    },
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
                    "morphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.location",
                        "dm.transport.world",
                    ],
                },
                {
                    "id": "op/ci.run_gate_terraform",
                    "path": "tools/ci/run_gate_terraform.sh",
                    "morphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.location",
                        "dm.transport.world",
                    ],
                },
            ]
        }
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
        )
        self.assertEqual(payload["result"], "accepted")
        self.assertEqual(payload["errors"], [])
        self.assertEqual(payload["summary"]["checkedPhase3CommandSurfaces"], 2)

    def test_rejects_when_required_operation_missing(self) -> None:
        control_plane_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"],
                    }
                }
            }
        }
        operation_registry = {
            "operations": [
                {
                    "id": "op/ci.unrelated",
                    "path": "tools/ci/run_gate.sh",
                    "morphisms": ["dm.identity"],
                }
            ]
        }
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertTrue(payload["errors"])

    def test_rejects_when_operation_path_outside_ci_boundary(self) -> None:
        control_plane_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"],
                    }
                }
            }
        }
        operation_registry = {
            "operations": [
                {
                    "id": "op/ci.run_gate",
                    "path": "scripts/run_gate.sh",
                    "morphisms": ["dm.identity"],
                }
            ]
        }
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("runtime_route_contract_unbound", payload["failureClasses"])

    def test_rejects_when_kcir_mapping_rows_are_missing(self) -> None:
        control_plane_contract = {
            "runtimeRouteBindings": {
                "requiredOperationRoutes": {
                    "runGate": {
                        "operationId": "op/ci.run_gate",
                        "requiredMorphisms": ["dm.identity"],
                    }
                }
            },
            "controlPlaneKcirMappings": {
                "mappingTable": {
                    "instructionEnvelope": {
                        "sourceKind": "ci.instruction.envelope.v1",
                        "targetDomain": "kcir.node",
                        "targetKind": "ci.instruction.v1",
                        "identityFields": [
                            "instructionDigest",
                            "normalizerId",
                            "policyDigest",
                        ],
                    }
                }
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
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("kcir_mapping_contract_violation", payload["failureClasses"])
        self.assertTrue(payload["kcirMappingRows"])

    def test_rejects_when_phase3_command_surface_is_missing(self) -> None:
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
                }
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
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("runtime_route_contract_unbound", payload["failureClasses"])
        self.assertTrue(payload["phase3CommandSurfaces"])


if __name__ == "__main__":
    unittest.main()
