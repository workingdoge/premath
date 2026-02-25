#!/usr/bin/env python3
"""Unit tests for runtime orchestration checker."""

from __future__ import annotations

import copy
import unittest

import check_runtime_orchestration


class RuntimeOrchestrationCheckerTests(unittest.TestCase):
    def _baseline_worldized_payload_inputs(self) -> tuple[dict, dict, str, dict]:
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
            "hostActionSurface": {
                "requiredActions": {
                    "issue.claim": {"operationId": "op/mcp.issue_claim"},
                    "issue.claim_next": {"operationId": "op/transport.issue_claim_next"},
                    "issue.lease_renew": {"operationId": "op/mcp.issue_lease_renew"},
                    "issue.lease_release": {"operationId": "op/mcp.issue_lease_release"},
                    "issue.discover": {"operationId": "op/mcp.issue_discover"},
                    "instruction.run": {"operationId": "op/mcp.instruction_run"},
                    "required.witness_verify": {
                        "operationId": "op/ci.verify_required_witness"
                    },
                    "required.witness_decide": {"operationId": "op/ci.decide_required"},
                    "fiber.spawn": {"operationId": "op/transport.fiber_spawn"},
                    "fiber.join": {"operationId": "op/transport.fiber_join"},
                    "fiber.cancel": {"operationId": "op/transport.fiber_cancel"},
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
                {
                    "id": "op/ci.run_instruction",
                    "path": "tools/ci/run_instruction.sh",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/mcp.instruction_run",
                    "path": "tools/mcp/server.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/mcp.issue_claim",
                    "path": "tools/mcp/server.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/mcp.issue_discover",
                    "path": "tools/mcp/server.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/mcp.issue_lease_release",
                    "path": "tools/mcp/server.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/mcp.issue_lease_renew",
                    "path": "tools/mcp/server.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/transport.issue_claim_next",
                    "path": "crates/premath-cli/src/commands/issue.rs",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
                {
                    "id": "op/ci.decide_required",
                    "path": "tools/ci/pipeline_required.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.presentation.projection",
                    ],
                },
                {
                    "id": "op/ci.verify_required_witness",
                    "path": "tools/ci/pipeline_required.py",
                    "morphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.presentation.projection",
                    ],
                },
                {
                    "id": "op/harness.session_bootstrap",
                    "path": "tools/harness/session.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/harness.session_read",
                    "path": "tools/harness/session.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/mcp.observe_instruction",
                    "path": "tools/mcp/server.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/mcp.observe_latest",
                    "path": "tools/mcp/server.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/mcp.observe_needs_attention",
                    "path": "tools/mcp/server.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/mcp.observe_projection",
                    "path": "tools/mcp/server.py",
                    "morphisms": ["dm.identity", "dm.presentation.projection"],
                },
                {
                    "id": "op/transport.fiber_cancel",
                    "path": "crates/premath-cli/src/commands/transport_dispatch.rs",
                    "morphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.world",
                    ],
                },
                {
                    "id": "op/transport.fiber_join",
                    "path": "crates/premath-cli/src/commands/transport_dispatch.rs",
                    "morphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.world",
                    ],
                },
                {
                    "id": "op/transport.fiber_spawn",
                    "path": "crates/premath-cli/src/commands/transport_dispatch.rs",
                    "morphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.world",
                    ],
                },
            ]
        }
        doctrine_site_input = {
            "worldRouteBindings": {
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [
                    {
                        "routeFamilyId": "route.fiber.lifecycle",
                        "worldId": "world.fiber.v1",
                        "morphismRowId": "wm.control.fiber.lifecycle",
                        "operationIds": [
                            "op/transport.fiber_cancel",
                            "op/transport.fiber_join",
                            "op/transport.fiber_spawn",
                        ],
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.world",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                    {
                        "routeFamilyId": "route.gate_execution",
                        "worldId": "world.kernel.semantic.v1",
                        "morphismRowId": "wm.kernel.semantic.runtime_gate",
                        "operationIds": ["op/ci.run_gate", "op/ci.run_gate_terraform"],
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.profile.execution",
                            "dm.transport.location",
                            "dm.transport.world",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                    {
                        "routeFamilyId": "route.instruction_execution",
                        "worldId": "world.instruction.v1",
                        "morphismRowId": "wm.control.instruction.execution",
                        "operationIds": ["op/ci.run_instruction", "op/mcp.instruction_run"],
                        "requiredMorphisms": [
                            "dm.commitment.attest",
                            "dm.identity",
                            "dm.profile.execution",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                    {
                        "routeFamilyId": "route.issue_claim_lease",
                        "worldId": "world.lease.v1",
                        "morphismRowId": "wm.control.lease.mutation",
                        "operationIds": [
                            "op/mcp.issue_claim",
                            "op/transport.issue_claim_next",
                            "op/mcp.issue_discover",
                            "op/mcp.issue_lease_release",
                            "op/mcp.issue_lease_renew",
                        ],
                        "requiredMorphisms": [
                            "dm.commitment.attest",
                            "dm.identity",
                            "dm.profile.execution",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                    {
                        "routeFamilyId": "route.required_decision_attestation",
                        "worldId": "world.ci_witness.v1",
                        "morphismRowId": "wm.control.ci_witness.attest",
                        "operationIds": [
                            "op/ci.decide_required",
                            "op/ci.verify_required_witness",
                        ],
                        "requiredMorphisms": [
                            "dm.commitment.attest",
                            "dm.identity",
                            "dm.presentation.projection",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                    {
                        "routeFamilyId": "route.session_projection",
                        "worldId": "world.control_plane.bundle.v0",
                        "morphismRowId": "wm.control.bundle.projection",
                        "operationIds": [
                            "op/harness.session_bootstrap",
                            "op/harness.session_read",
                            "op/mcp.observe_instruction",
                            "op/mcp.observe_latest",
                            "op/mcp.observe_needs_attention",
                            "op/mcp.observe_projection",
                        ],
                        "requiredMorphisms": [
                            "dm.identity",
                            "dm.presentation.projection",
                        ],
                        "failureClassUnbound": "world_route_unbound",
                    },
                ],
            }
        }
        harness_runtime_text = """
## 1.2 Harness-Squeak composition boundary (required)
Harness computes deterministic work context and witness lineage refs.
Squeak performs transport/runtime-placement mapping and emits transport-class witness outcomes.
Destination Tusk/Gate performs destination-local admissibility checks and emits Gate-class outcomes.
Harness records the resulting references in session/trajectory projections.
""".strip()
        return (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        )

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

    def test_rejects_when_phase3_command_surface_section_is_missing(self) -> None:
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

    def test_world_routes_accept_when_bindings_are_complete(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        self.assertEqual(payload["result"], "accepted")
        self.assertEqual(payload["failureClasses"], [])
        self.assertEqual(payload["summary"]["checkedWorldRouteFamilies"], 6)

    def test_world_routes_reject_when_required_family_is_missing(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        doctrine_site_input["worldRouteBindings"]["rows"] = [
            row
            for row in doctrine_site_input["worldRouteBindings"]["rows"]
            if row["routeFamilyId"] != "route.issue_claim_lease"
        ]
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("world_route_unbound", payload["failureClasses"])

    def test_world_routes_reject_when_operation_morphism_drifts(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        for operation in operation_registry["operations"]:
            if operation["id"] == "op/ci.verify_required_witness":
                operation["morphisms"] = ["dm.commitment.attest", "dm.identity"]
                break
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("world_route_morphism_drift", payload["failureClasses"])

    def test_world_routes_reject_when_control_plane_operation_binding_is_unbound(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        for row in doctrine_site_input["worldRouteBindings"]["rows"]:
            if row["routeFamilyId"] == "route.required_decision_attestation":
                row["operationIds"] = ["op/ci.decide_required"]
                break
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("world_route_unbound", payload["failureClasses"])

    def test_world_routes_reject_when_control_plane_binding_lacks_operation_id(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        control_plane_contract["hostActionSurface"]["requiredActions"][
            "required.witness_verify"
        ] = {}
        payload = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        self.assertEqual(payload["result"], "rejected")
        self.assertIn("runtime_route_contract_unbound", payload["failureClasses"])

    def test_world_routes_are_order_invariant(self) -> None:
        (
            control_plane_contract,
            operation_registry,
            harness_runtime_text,
            doctrine_site_input,
        ) = self._baseline_worldized_payload_inputs()
        permuted_site_input = copy.deepcopy(doctrine_site_input)
        permuted_site_input["worldRouteBindings"]["rows"] = list(
            reversed(permuted_site_input["worldRouteBindings"]["rows"])
        )
        payload_a = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
        payload_b = check_runtime_orchestration.evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=permuted_site_input,
        )
        self.assertEqual(payload_a["result"], "accepted")
        self.assertEqual(payload_b["result"], "accepted")
        self.assertEqual(
            [row["routeFamilyId"] for row in payload_a["worldRouteBindings"]],
            [row["routeFamilyId"] for row in payload_b["worldRouteBindings"]],
        )


if __name__ == "__main__":
    unittest.main()
