#!/usr/bin/env python3
"""Unit tests for control-plane contract loader lane registry extensions."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import control_plane_contract


def _base_payload() -> dict:
    return {
        "schema": 1,
        "contractKind": "premath.control_plane.contract.v1",
        "contractId": "control-plane.default.v1",
        "schemaLifecycle": {
            "activeEpoch": "2026-02",
            "governance": {
                "mode": "rollover",
                "decisionRef": "decision-0105",
                "owner": "premath-core",
                "rolloverCadenceMonths": 6,
            },
            "kindFamilies": {
                "controlPlaneContractKind": {
                    "canonicalKind": "premath.control_plane.contract.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "premath.control_plane.contract.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "premath.control_plane.contract.v1",
                        }
                    ],
                },
                "requiredWitnessKind": {
                    "canonicalKind": "ci.required.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.required.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.v1",
                        }
                    ],
                },
                "requiredDecisionKind": {
                    "canonicalKind": "ci.required.decision.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.required.decision.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.decision.v1",
                        }
                    ],
                },
                "instructionWitnessKind": {
                    "canonicalKind": "ci.instruction.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.instruction.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.instruction.v1",
                        }
                    ],
                },
                "instructionPolicyKind": {
                    "canonicalKind": "ci.instruction.policy.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.instruction.policy.v0",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.instruction.policy.v1",
                        }
                    ],
                },
                "requiredProjectionPolicy": {
                    "canonicalKind": "ci-topos-v0",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci-topos-v0-preview",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci-topos-v0",
                        }
                    ],
                },
                "requiredDeltaKind": {
                    "canonicalKind": "ci.required.delta.v1",
                    "compatibilityAliases": [
                        {
                            "aliasKind": "ci.delta.v1",
                            "supportUntilEpoch": "2026-06",
                            "replacementKind": "ci.required.delta.v1",
                        }
                    ],
                },
            },
        },
        "controlPlaneBundleProfile": {
            "profileId": "cp.bundle.v0",
            "contextFamily": {
                "id": "C_cp",
                "contextKinds": [
                    "repo_head",
                    "workspace_delta",
                    "instruction_envelope",
                    "policy_snapshot",
                    "witness_projection",
                ],
                "morphismKinds": [
                    "ctx.identity",
                    "ctx.rebase",
                    "ctx.patch",
                    "ctx.policy_rollover",
                ],
            },
            "artifactFamily": {
                "id": "E_cp",
                "artifactRefs": {
                    "controlPlaneContract": "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
                    "coherenceContract": "specs/premath/draft/COHERENCE-CONTRACT.json",
                    "capabilityRegistry": "specs/premath/draft/CAPABILITY-REGISTRY.json",
                    "doctrineSiteInput": "specs/premath/draft/DOCTRINE-SITE-INPUT.json",
                    "doctrineOpRegistry": "specs/premath/draft/DOCTRINE-OP-REGISTRY.json",
                },
            },
            "reindexingCoherence": {
                "requiredObligations": [
                    "identity_preserved",
                    "composition_preserved",
                    "policy_digest_stable",
                    "route_bindings_total",
                ],
                "commutationWitness": "span_square_commutation",
            },
            "coverGlue": {
                "workerCoverKind": "worktree_partition_cover",
                "mergeCompatibilityWitness": "span_square_commutation",
                "requiredMergeArtifacts": [
                    "ci.required.v1",
                    "ci.instruction.v1",
                    "coherence_witness",
                ],
            },
            "authoritySplit": {
                "semanticAuthority": [
                    "PREMATH-KERNEL",
                    "GATE",
                    "BIDIR-DESCENT",
                ],
                "controlPlaneRole": "projection_and_parity_only",
                "forbiddenControlPlaneRoles": [
                    "semantic_obligation_discharge",
                    "admissibility_override",
                ],
            },
        },
        "controlPlaneKcirMappings": {
            "profileId": "cp.kcir.mapping.v0",
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
                },
                "proposalPayload": {
                    "sourceKind": "ci.proposal.payload.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "ci.proposal.check.v1",
                    "identityFields": [
                        "proposalDigest",
                        "proposalKcirRef",
                        "policyDigest",
                    ],
                },
                "coherenceObligations": {
                    "sourceKind": "coherence.obligation.set.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "coherence.obligation.witness.v1",
                    "identityFields": [
                        "obligationDigest",
                        "normalizerId",
                        "policyDigest",
                    ],
                },
                "coherenceCheckPayload": {
                    "sourceKind": "coherence.check.payload.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "coherence.check.witness.v1",
                    "identityFields": [
                        "projectionDigest",
                        "normalizerId",
                        "policyDigest",
                    ],
                },
                "doctrineRouteBinding": {
                    "sourceKind": "doctrine.route.binding.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "doctrine.route.witness.v1",
                    "identityFields": [
                        "operationId",
                        "siteDigest",
                        "policyDigest",
                    ],
                },
                "requiredDecisionInput": {
                    "sourceKind": "ci.required.decision.input.v1",
                    "targetDomain": "kcir.node",
                    "targetKind": "ci.required.decision.v1",
                    "identityFields": [
                        "requiredDigest",
                        "decisionDigest",
                        "policyDigest",
                    ],
                },
            },
            "identityDigestLineage": {
                "digestAlgorithm": "sha256",
                "refProfilePath": "policies/ref/sha256_detached_v1.json",
                "normalizerField": "normalizerId",
                "policyDigestField": "policyDigest",
            },
            "compatibilityPolicy": {
                "legacyNonKcirEncodings": {
                    "mode": "projection_only",
                    "authorityMode": "forbidden",
                    "supportUntilEpoch": "2026-06",
                    "failureClass": "kcir_mapping_legacy_encoding_authority_violation",
                }
            },
        },
        "requiredGateProjection": {
            "projectionPolicy": "ci-topos-v0",
            "checkIds": {
                "baseline": "baseline",
                "build": "build",
                "test": "test",
                "testToy": "test-toy",
                "testKcirToy": "test-kcir-toy",
                "conformanceCheck": "conformance-check",
                "conformanceRun": "conformance-run",
                "doctrineCheck": "doctrine-check",
            },
            "checkOrder": [
                "baseline",
                "build",
                "test",
                "test-toy",
                "test-kcir-toy",
                "conformance-check",
                "conformance-run",
                "doctrine-check",
            ],
        },
        "requiredWitness": {
            "witnessKind": "ci.required.v1",
            "decisionKind": "ci.required.decision.v1",
        },
        "instructionWitness": {
            "witnessKind": "ci.instruction.v1",
            "policyKind": "ci.instruction.policy.v1",
            "policyDigestPrefix": "pol1_",
        },
        "evidenceStage1Parity": {
            "profileKind": "ev.stage1.core.v1",
            "authorityToTypedCoreRoute": "authority_to_typed_core_projection",
            "comparisonTuple": {
                "authorityDigestRef": "authorityPayloadDigest",
                "typedCoreDigestRef": "typedCoreProjectionDigest",
                "normalizerIdRef": "normalizerId",
                "policyDigestRef": "policyDigest",
            },
            "failureClasses": {
                "missing": "unification.evidence_stage1.parity.missing",
                "mismatch": "unification.evidence_stage1.parity.mismatch",
                "unbound": "unification.evidence_stage1.parity.unbound",
            },
        },
        "evidenceStage1Rollback": {
            "profileKind": "ev.stage1.rollback.v1",
            "witnessKind": "ev.stage1.rollback.witness.v1",
            "fromStage": "stage1",
            "toStage": "stage0",
            "triggerFailureClasses": [
                "unification.evidence_stage1.parity.missing",
                "unification.evidence_stage1.parity.mismatch",
                "unification.evidence_stage1.parity.unbound",
            ],
            "identityRefs": {
                "authorityDigestRef": "authorityPayloadDigest",
                "rollbackAuthorityDigestRef": "rollbackAuthorityPayloadDigest",
                "normalizerIdRef": "normalizerId",
                "policyDigestRef": "policyDigest",
            },
            "failureClasses": {
                "precondition": "unification.evidence_stage1.rollback.precondition",
                "identityDrift": "unification.evidence_stage1.rollback.identity_drift",
                "unbound": "unification.evidence_stage1.rollback.unbound",
            },
        },
        "evidenceStage2Authority": {
            "profileKind": "ev.stage2.authority.v1",
            "activeStage": "stage2",
            "typedAuthority": {
                "kindRef": "ev.stage1.core.v1",
                "digestRef": "typedCoreProjectionDigest",
                "normalizerIdRef": "normalizerId",
                "policyDigestRef": "policyDigest",
            },
            "compatibilityAlias": {
                "kindRef": "ev.legacy.payload.v1",
                "digestRef": "authorityPayloadDigest",
                "role": "projection_only",
                "supportUntilEpoch": "2026-06",
            },
            "bidirEvidenceRoute": {
                "routeKind": "direct_checker_discharge",
                "obligationFieldRef": "bidirCheckerObligations",
                "requiredObligations": [
                    "stability",
                    "locality",
                    "descent_exists",
                    "descent_contractible",
                    "adjoint_triple",
                    "ext_gap",
                    "ext_ambiguous",
                ],
                "failureClasses": {
                    "missing": "unification.evidence_stage2.kernel_compliance_missing",
                    "drift": "unification.evidence_stage2.kernel_compliance_drift",
                },
            },
            "failureClasses": {
                "authorityAliasViolation": "unification.evidence_stage2.authority_alias_violation",
                "aliasWindowViolation": "unification.evidence_stage2.alias_window_violation",
                "unbound": "unification.evidence_stage2.unbound",
            },
        },
        "workerLaneAuthority": {
            "mutationPolicy": {
                "defaultMode": "instruction-linked",
                "allowedModes": [
                    "instruction-linked",
                    "human-override",
                ],
                "compatibilityOverrides": [
                    {
                        "mode": "human-override",
                        "supportUntilEpoch": "2026-06",
                        "requiresReason": True,
                    }
                ],
            },
            "mutationRoutes": {
                "issueClaim": "capabilities.change_morphisms.issue_claim",
                "issueLeaseRenew": "capabilities.change_morphisms.issue_lease_renew",
                "issueLeaseRelease": "capabilities.change_morphisms.issue_lease_release",
                "issueDiscover": "capabilities.change_morphisms.issue_discover",
            },
            "failureClasses": {
                "policyDrift": "worker_lane_policy_drift",
                "mutationModeDrift": "worker_lane_mutation_mode_drift",
                "routeUnbound": "worker_lane_route_unbound",
            },
        },
        "worldDescentContract": {
            "contractId": "doctrine.world_descent.v1",
            "requiredRouteFamilies": [
                "route.gate_execution",
                "route.instruction_execution",
                "route.required_decision_attestation",
                "route.fiber.lifecycle",
                "route.issue_claim_lease",
                "route.session_projection",
                "route.transport.dispatch",
            ],
            "requiredActionRouteBindings": {
                "route.instruction_execution": [
                    "instruction.run",
                ],
                "route.required_decision_attestation": [
                    "required.witness_verify",
                    "required.witness_decide",
                ],
                "route.fiber.lifecycle": [
                    "fiber.spawn",
                    "fiber.join",
                    "fiber.cancel",
                ],
                "route.issue_claim_lease": [
                    "issue.claim_next",
                    "issue.claim",
                    "issue.lease_renew",
                    "issue.lease_release",
                    "issue.discover",
                ],
            },
            "requiredStaticOperationBindings": {
                "route.transport.dispatch": [
                    "op/transport.world_route_binding",
                ]
            },
            "failureClasses": {
                "identityMissing": "world_route_identity_missing",
                "descentDataMissing": "world_descent_data_missing",
                "kcirHandoffIdentityMissing": "kcir_handoff_identity_missing",
            },
        },
        "runtimeRouteBindings": {
            "requiredOperationRoutes": {
                "runGate": {
                    "operationId": "op/ci.run_gate",
                    "routeFamilyId": "route.gate_execution",
                    "requiredMorphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.location",
                        "dm.transport.world",
                    ],
                },
                "runGateTerraform": {
                    "operationId": "op/ci.run_gate_terraform",
                    "routeFamilyId": "route.gate_execution",
                    "requiredMorphisms": [
                        "dm.identity",
                        "dm.profile.execution",
                        "dm.transport.location",
                        "dm.transport.world",
                    ],
                },
                "runInstruction": {
                    "operationId": "op/ci.run_instruction",
                    "routeFamilyId": "route.instruction_execution",
                    "requiredMorphisms": [
                        "dm.commitment.attest",
                        "dm.identity",
                        "dm.profile.execution",
                    ],
                },
            },
            "failureClasses": {
                "missingRoute": "runtime_route_missing",
                "morphismDrift": "runtime_route_morphism_drift",
                "contractUnbound": "runtime_route_contract_unbound",
            },
        },
        "commandSurface": {
            "requiredDecision": {
                "canonicalEntrypoint": [
                    "mise",
                    "run",
                    "ci-required-attested",
                ],
                "compatibilityAliases": [
                    [
                        "mise",
                        "run",
                        "ci-check",
                    ]
                ],
            },
            "instructionEnvelopeCheck": {
                "canonicalEntrypoint": [
                    "cargo",
                    "run",
                    "--package",
                    "premath-cli",
                    "--",
                    "instruction-check",
                    "--instruction",
                    "$INSTRUCTION_PATH",
                    "--repo-root",
                    "$REPO_ROOT",
                    "--json",
                ],
                "compatibilityAliases": [],
            },
            "instructionDecision": {
                "canonicalEntrypoint": [
                    "python3",
                    "tools/ci/run_instruction.py",
                ],
                "compatibilityAliases": [
                    [
                        "sh",
                        "tools/ci/run_instruction.sh",
                    ]
                ],
            },
            "governancePromotionCheck": {
                "canonicalEntrypoint": [
                    "cargo",
                    "run",
                    "--package",
                    "premath-cli",
                    "--",
                    "governance-promotion-check",
                ],
                "compatibilityAliases": [],
            },
            "kcirMappingCheck": {
                "canonicalEntrypoint": [
                    "cargo",
                    "run",
                    "--package",
                    "premath-cli",
                    "--",
                    "kcir-mapping-check",
                ],
                "compatibilityAliases": [],
            },
            "failureClasses": {
                "unbound": "control_plane_command_surface_unbound",
            },
        },
        "pipelineWrapperSurface": {
            "requiredPipelineEntrypoint": [
                "python3",
                "tools/ci/pipeline_required.py",
            ],
            "instructionPipelineEntrypoint": [
                "python3",
                "tools/ci/pipeline_instruction.py",
                "--instruction",
                "$INSTRUCTION_PATH",
            ],
            "requiredGateHooks": {
                "governance": "governance_failure_classes",
                "kcirMapping": "evaluate_required_mapping",
            },
            "instructionGateHooks": {
                "governance": "governance_failure_classes",
                "kcirMapping": "evaluate_instruction_mapping",
            },
            "failureClasses": {
                "unbound": "control_plane_pipeline_wrapper_unbound",
                "parityDrift": "control_plane_pipeline_wrapper_parity_drift",
                "governanceGateMissing": "control_plane_pipeline_governance_gate_missing",
                "kcirMappingGateMissing": "control_plane_pipeline_kcir_mapping_gate_missing",
            },
        },
        "hostActionSurface": {
            "requiredActions": {
                "issue.ready": {
                    "canonicalCli": "premath issue ready --issues <path> --json",
                    "mcpTool": "issue_ready",
                    "operationId": "op/mcp.issue_ready",
                },
                "issue.claim": {
                    "canonicalCli": "premath issue claim <issue-id> --assignee <name> --issues <path> --json",
                    "mcpTool": "issue_claim",
                    "operationId": "op/mcp.issue_claim",
                },
                "coherence.check": {
                    "canonicalCli": "premath coherence-check --contract <path> --repo-root <repo> --json",
                    "mcpTool": None,
                },
                "instruction.run": {
                    "canonicalCli": "premath transport-dispatch --action instruction.run --payload '<json>' --json",
                    "mcpTool": "instruction_run",
                    "operationId": "op/mcp.instruction_run",
                },
                "issue.lease_renew": {
                    "canonicalCli": None,
                    "mcpTool": "issue_lease_renew",
                    "operationId": "op/mcp.issue_lease_renew",
                },
                "issue.lease_release": {
                    "canonicalCli": None,
                    "mcpTool": "issue_lease_release",
                    "operationId": "op/mcp.issue_lease_release",
                },
            },
            "mcpOnlyHostActions": [
                "issue.lease_renew",
                "issue.lease_release",
            ],
            "failureClasses": {
                "unregisteredHostId": "control_plane_host_action_unregistered",
                "bindingMismatch": "control_plane_host_action_binding_mismatch",
                "duplicateBinding": "control_plane_host_action_duplicate_binding",
                "contractUnbound": "control_plane_host_action_contract_unbound",
            },
        },
        "harnessRetry": {
            "policyKind": "ci.harness.retry.policy.v1",
            "policyPath": "policies/control/harness-retry-policy-v1.json",
            "escalationActions": [
                "issue_discover",
                "mark_blocked",
                "stop",
            ],
            "activeIssueEnvKeys": [
                "PREMATH_ACTIVE_ISSUE_ID",
                "PREMATH_ISSUE_ID",
            ],
            "issuesPathEnvKey": "PREMATH_ISSUES_PATH",
            "sessionPathEnvKey": "PREMATH_HARNESS_SESSION_PATH",
            "sessionPathDefault": ".premath/harness_session.json",
            "sessionIssueField": "issueId",
        },
    }


def _with_lane_registry(payload: dict) -> dict:
    out = dict(payload)
    out["evidenceLanes"] = {
        "semanticDoctrine": "semantic_doctrine",
        "strictChecker": "strict_checker",
        "witnessCommutation": "witness_commutation",
        "runtimeTransport": "runtime_transport",
    }
    out["laneArtifactKinds"] = {
        "semantic_doctrine": ["kernel_obligation"],
        "strict_checker": ["coherence_obligation"],
        "witness_commutation": ["square_witness"],
        "runtime_transport": ["squeak_site_witness"],
    }
    out["laneOwnership"] = {
        "checkerCoreOnlyObligations": ["cwf_substitution_identity"],
        "requiredCrossLaneWitnessRoute": {
            "pullbackBaseChange": "span_square_commutation"
        },
    }
    out["laneFailureClasses"] = [
        "lane_unknown",
        "lane_kind_unbound",
        "lane_ownership_violation",
        "lane_route_missing",
    ]
    return out


class ControlPlaneContractTests(unittest.TestCase):
    def _load(self, payload: dict) -> dict:
        with tempfile.TemporaryDirectory(prefix="control-plane-contract-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            return control_plane_contract.load_control_plane_contract(path)

    def test_load_accepts_lane_registry_extension(self) -> None:
        payload = _with_lane_registry(_base_payload())
        loaded = self._load(payload)
        self.assertEqual(
            loaded["evidenceLanes"]["semanticDoctrine"], "semantic_doctrine"
        )
        self.assertEqual(
            loaded["laneOwnership"]["requiredCrossLaneWitnessRoute"],
            "span_square_commutation",
        )
        self.assertIn("lane_route_missing", loaded["laneFailureClasses"])
        self.assertEqual(
            loaded["schemaLifecycle"]["epochDiscipline"]["rolloverEpoch"],
            "2026-06",
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["epochDiscipline"]["aliasRunwayMonths"],
            4,
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["mode"],
            "rollover",
        )
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["rolloverCadenceMonths"],
            6,
        )
        self.assertEqual(
            loaded["harnessRetry"]["policyKind"],
            "ci.harness.retry.policy.v1",
        )
        self.assertIn("mark_blocked", loaded["harnessRetry"]["escalationActions"])
        self.assertEqual(
            loaded["harnessRetry"]["sessionPathEnvKey"],
            "PREMATH_HARNESS_SESSION_PATH",
        )
        self.assertEqual(
            loaded["evidenceStage1Parity"]["profileKind"],
            "ev.stage1.core.v1",
        )
        self.assertEqual(
            loaded["evidenceStage1Rollback"]["witnessKind"],
            "ev.stage1.rollback.witness.v1",
        )
        self.assertEqual(
            loaded["evidenceStage2Authority"]["profileKind"],
            "ev.stage2.authority.v1",
        )
        self.assertEqual(
            loaded["evidenceStage2Authority"]["compatibilityAlias"]["role"],
            "projection_only",
        )
        self.assertIn(
            "stability",
            loaded["evidenceStage2Authority"]["bidirEvidenceRoute"]["requiredObligations"],
        )
        self.assertEqual(
            loaded["workerLaneAuthority"]["mutationPolicy"]["defaultMode"],
            "instruction-linked",
        )
        self.assertIn(
            "human-override",
            loaded["workerLaneAuthority"]["mutationPolicy"]["allowedModes"],
        )
        self.assertEqual(
            loaded["workerLaneAuthority"]["mutationRoutes"]["issueDiscover"],
            "capabilities.change_morphisms.issue_discover",
        )
        self.assertEqual(
            loaded["workerLaneAuthority"]["failureClasses"]["routeUnbound"],
            "worker_lane_route_unbound",
        )
        self.assertEqual(
            loaded["worldDescentContract"]["contractId"],
            "doctrine.world_descent.v1",
        )
        self.assertIn(
            "route.transport.dispatch",
            loaded["worldDescentContract"]["requiredRouteFamilies"],
        )
        self.assertEqual(
            loaded["worldDescentContract"]["requiredStaticOperationBindings"][
                "route.transport.dispatch"
            ],
            ["op/transport.world_route_binding"],
        )
        self.assertEqual(
            loaded["worldDescentContract"]["failureClasses"]["descentDataMissing"],
            "world_descent_data_missing",
        )
        self.assertEqual(
            loaded["runtimeRouteBindings"]["requiredOperationRoutes"]["runGate"][
                "operationId"
            ],
            "op/ci.run_gate",
        )
        self.assertEqual(
            loaded["runtimeRouteBindings"]["requiredOperationRoutes"]["runGate"][
                "routeFamilyId"
            ],
            "route.gate_execution",
        )
        self.assertEqual(
            loaded["runtimeRouteBindings"]["requiredOperationRoutes"]["runInstruction"][
                "operationId"
            ],
            "op/ci.run_instruction",
        )
        self.assertEqual(
            loaded["runtimeRouteBindings"]["requiredOperationRoutes"]["runInstruction"][
                "routeFamilyId"
            ],
            "route.instruction_execution",
        )
        self.assertEqual(
            loaded["commandSurface"]["requiredDecision"]["canonicalEntrypoint"],
            ["mise", "run", "ci-required-attested"],
        )
        self.assertEqual(
            loaded["commandSurface"]["instructionDecision"]["compatibilityAliases"],
            [["sh", "tools/ci/run_instruction.sh"]],
        )
        self.assertEqual(
            loaded["commandSurface"]["governancePromotionCheck"]["canonicalEntrypoint"],
            [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "governance-promotion-check",
            ],
        )
        self.assertEqual(
            loaded["commandSurface"]["kcirMappingCheck"]["canonicalEntrypoint"],
            [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "kcir-mapping-check",
            ],
        )
        self.assertEqual(
            loaded["pipelineWrapperSurface"]["requiredPipelineEntrypoint"],
            ["python3", "tools/ci/pipeline_required.py"],
        )
        self.assertEqual(
            loaded["pipelineWrapperSurface"]["instructionPipelineEntrypoint"],
            [
                "python3",
                "tools/ci/pipeline_instruction.py",
                "--instruction",
                "$INSTRUCTION_PATH",
            ],
        )
        self.assertEqual(
            loaded["pipelineWrapperSurface"]["requiredGateHooks"]["governance"],
            "governance_failure_classes",
        )
        self.assertEqual(
            loaded["pipelineWrapperSurface"]["instructionGateHooks"]["kcirMapping"],
            "evaluate_instruction_mapping",
        )
        self.assertEqual(
            loaded["hostActionSurface"]["requiredActions"]["issue.ready"][
                "canonicalCli"
            ],
            "premath issue ready --issues <path> --json",
        )
        self.assertEqual(
            loaded["hostActionSurface"]["requiredActions"]["issue.lease_renew"][
                "canonicalCli"
            ],
            None,
        )
        self.assertEqual(
            loaded["hostActionSurface"]["requiredActions"]["issue.lease_renew"][
                "mcpTool"
            ],
            "issue_lease_renew",
        )
        self.assertEqual(
            loaded["hostActionSurface"]["requiredActions"]["issue.lease_renew"][
                "operationId"
            ],
            "op/mcp.issue_lease_renew",
        )
        self.assertEqual(
            loaded["hostActionSurface"]["requiredActions"]["instruction.run"][
                "canonicalCli"
            ],
            "premath transport-dispatch --action instruction.run --payload '<json>' --json",
        )
        self.assertEqual(
            loaded["hostActionSurface"]["mcpOnlyHostActions"],
            ["issue.lease_renew", "issue.lease_release"],
        )
        self.assertEqual(
            loaded["hostActionSurface"]["failureClasses"]["duplicateBinding"],
            "control_plane_host_action_duplicate_binding",
        )
        self.assertEqual(
            loaded["controlPlaneBundleProfile"]["profileId"],
            "cp.bundle.v0",
        )
        self.assertEqual(
            loaded["controlPlaneBundleProfile"]["contextFamily"]["id"],
            "C_cp",
        )
        self.assertEqual(
            loaded["controlPlaneBundleProfile"]["artifactFamily"]["id"],
            "E_cp",
        )
        self.assertEqual(
            loaded["controlPlaneKcirMappings"]["profileId"],
            "cp.kcir.mapping.v0",
        )
        self.assertEqual(
            loaded["controlPlaneKcirMappings"]["compatibilityPolicy"][
                "legacyNonKcirEncodings"
            ]["mode"],
            "projection_only",
        )
        self.assertIn(
            "runtime_route_morphism_drift",
            loaded["runtimeRouteBindings"]["failureClasses"]["morphismDrift"],
        )

    def test_load_rejects_duplicate_lane_ids(self) -> None:
        payload = _with_lane_registry(_base_payload())
        payload["evidenceLanes"]["runtimeTransport"] = "strict_checker"
        with self.assertRaises(ValueError):
            self._load(payload)

    def test_load_rejects_unknown_lane_artifact_mapping(self) -> None:
        payload = _with_lane_registry(_base_payload())
        payload["laneArtifactKinds"]["unknown_lane"] = ["opaque_kind"]
        with self.assertRaises(ValueError):
            self._load(payload)

    def test_load_rejects_worker_lane_default_mode_drift(self) -> None:
        payload = _base_payload()
        payload["workerLaneAuthority"]["mutationPolicy"]["defaultMode"] = "human-override"
        with self.assertRaisesRegex(ValueError, "defaultMode"):
            self._load(payload)

    def test_load_rejects_worker_lane_route_drift(self) -> None:
        payload = _base_payload()
        payload["workerLaneAuthority"]["mutationRoutes"]["issueDiscover"] = "issue_discover"
        with self.assertRaisesRegex(ValueError, "canonical route"):
            self._load(payload)

    def test_load_rejects_missing_world_descent_contract(self) -> None:
        payload = _base_payload()
        payload.pop("worldDescentContract")
        with self.assertRaisesRegex(
            ValueError, "controlPlaneContract.worldDescentContract"
        ):
            self._load(payload)

    def test_load_rejects_world_descent_failure_class_key_drift(self) -> None:
        payload = _base_payload()
        payload["worldDescentContract"]["failureClasses"].pop("descentDataMissing")
        with self.assertRaisesRegex(
            ValueError,
            "worldDescentContract.failureClasses.descentDataMissing",
        ):
            self._load(payload)

    def test_load_rejects_runtime_route_morphism_drift(self) -> None:
        payload = _base_payload()
        payload["runtimeRouteBindings"]["requiredOperationRoutes"]["runGate"][
            "requiredMorphisms"
        ] = []
        with self.assertRaisesRegex(ValueError, "non-empty list"):
            self._load(payload)

    def test_load_rejects_command_surface_canonical_entrypoint_drift(self) -> None:
        payload = _base_payload()
        payload["commandSurface"]["requiredDecision"]["canonicalEntrypoint"] = [
            "mise",
            "run",
            "ci-check",
        ]
        with self.assertRaisesRegex(ValueError, "canonicalEntrypoint"):
            self._load(payload)

    def test_load_rejects_command_surface_alias_set_drift(self) -> None:
        payload = _base_payload()
        payload["commandSurface"]["instructionDecision"]["compatibilityAliases"] = [
            ["python3", "tools/ci/run_instruction.py"]
        ]
        with self.assertRaisesRegex(ValueError, "must not include canonicalEntrypoint"):
            self._load(payload)

    def test_load_rejects_pipeline_wrapper_missing_gate_hook(self) -> None:
        payload = _base_payload()
        payload["pipelineWrapperSurface"]["requiredGateHooks"].pop("governance")
        with self.assertRaisesRegex(ValueError, "requiredGateHooks missing required keys"):
            self._load(payload)

    def test_load_rejects_pipeline_wrapper_failure_class_key_drift(self) -> None:
        payload = _base_payload()
        payload["pipelineWrapperSurface"]["failureClasses"].pop("parityDrift")
        with self.assertRaisesRegex(ValueError, "failureClasses missing required keys"):
            self._load(payload)

    def test_load_rejects_host_action_with_no_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.ready"] = {
            "canonicalCli": None,
            "mcpTool": None,
        }
        with self.assertRaisesRegex(ValueError, "must bind at least one"):
            self._load(payload)

    def test_load_rejects_host_action_duplicate_cli_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.list"] = {
            "canonicalCli": "premath issue ready --issues <path> --json",
            "mcpTool": "issue_list",
            "operationId": "op/mcp.issue_list",
        }
        with self.assertRaisesRegex(ValueError, "canonicalCli binding is ambiguous"):
            self._load(payload)

    def test_load_rejects_host_action_duplicate_mcp_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.list"] = {
            "canonicalCli": "premath issue list --issues <path> --json",
            "mcpTool": "issue_ready",
            "operationId": "op/mcp.issue_ready",
        }
        with self.assertRaisesRegex(ValueError, "mcpTool binding is ambiguous"):
            self._load(payload)

    def test_load_rejects_host_action_failure_class_key_drift(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["failureClasses"].pop("contractUnbound")
        with self.assertRaisesRegex(ValueError, "missing required keys"):
            self._load(payload)

    def test_load_rejects_host_action_mcp_only_unknown_action(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["mcpOnlyHostActions"] = [
            "issue.lease_renew",
            "issue.not_real",
        ]
        with self.assertRaisesRegex(ValueError, "references unknown host action"):
            self._load(payload)

    def test_load_rejects_host_action_missing_mcp_operation_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.claim"].pop("operationId")
        with self.assertRaisesRegex(ValueError, "must match mcpTool binding"):
            self._load(payload)

    def test_load_rejects_host_action_unknown_operation_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.claim"][
            "operationId"
        ] = "op/mcp.issue_claim_shadow"
        with self.assertRaisesRegex(ValueError, "must exist in doctrine op registry"):
            self._load(payload)

    def test_load_rejects_host_action_mcp_only_cli_binding(self) -> None:
        payload = _base_payload()
        payload["hostActionSurface"]["requiredActions"]["issue.lease_renew"][
            "canonicalCli"
        ] = "premath issue lease-renew <issue-id>"
        with self.assertRaisesRegex(ValueError, "requires null canonicalCli"):
            self._load(payload)

    def test_load_rejects_control_plane_bundle_context_family_id_mismatch(self) -> None:
        payload = _base_payload()
        payload["controlPlaneBundleProfile"]["contextFamily"]["id"] = "C_runtime"
        with self.assertRaisesRegex(ValueError, "contextFamily.id"):
            self._load(payload)

    def test_load_rejects_control_plane_bundle_control_plane_role_mismatch(self) -> None:
        payload = _base_payload()
        payload["controlPlaneBundleProfile"]["authoritySplit"]["controlPlaneRole"] = (
            "semantic_authority"
        )
        with self.assertRaisesRegex(ValueError, "controlPlaneRole"):
            self._load(payload)

    def test_load_rejects_control_plane_kcir_mapping_missing_row(self) -> None:
        payload = _base_payload()
        payload["controlPlaneKcirMappings"]["mappingTable"] = {}
        with self.assertRaisesRegex(ValueError, "must be non-empty"):
            self._load(payload)

    def test_load_rejects_control_plane_kcir_legacy_policy_window_mismatch(self) -> None:
        payload = _base_payload()
        payload["controlPlaneKcirMappings"]["compatibilityPolicy"][
            "legacyNonKcirEncodings"
        ]["supportUntilEpoch"] = "2026-07"
        with self.assertRaisesRegex(ValueError, "rolloverEpoch"):
            self._load(payload)

    def test_load_rejects_worker_lane_expired_override(self) -> None:
        payload = _base_payload()
        payload["workerLaneAuthority"]["mutationPolicy"]["compatibilityOverrides"][0][
            "supportUntilEpoch"
        ] = "2026-01"
        with self.assertRaisesRegex(ValueError, "expired"):
            self._load(payload)

    def test_resolve_schema_kind_accepts_alias_within_support_window(self) -> None:
        self.assertEqual(
            control_plane_contract.resolve_schema_kind(
                "requiredWitnessKind",
                "ci.required.v0",
                active_epoch="2026-06",
            ),
            "ci.required.v1",
        )

    def test_resolve_schema_kind_rejects_expired_alias(self) -> None:
        with self.assertRaisesRegex(ValueError, "expired"):
            control_plane_contract.resolve_schema_kind(
                "requiredWitnessKind",
                "ci.required.v0",
                active_epoch="2026-07",
            )

    def test_load_rejects_mixed_rollover_epochs(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["kindFamilies"]["requiredWitnessKind"][
            "compatibilityAliases"
        ][0]["supportUntilEpoch"] = "2026-07"
        with self.assertRaisesRegex(ValueError, "one shared supportUntilEpoch"):
            self._load(payload)

    def test_load_rejects_rollover_runway_too_large(self) -> None:
        payload = _base_payload()
        for family in payload["schemaLifecycle"]["kindFamilies"].values():
            aliases = family.get("compatibilityAliases", [])
            for alias_row in aliases:
                alias_row["supportUntilEpoch"] = "2027-03"
        with self.assertRaisesRegex(ValueError, "max runway"):
            self._load(payload)

    def test_load_rejects_duplicate_harness_escalation_actions(self) -> None:
        payload = _base_payload()
        payload["harnessRetry"]["escalationActions"] = [
            "issue_discover",
            "issue_discover",
        ]
        with self.assertRaisesRegex(ValueError, "must not contain duplicates"):
            self._load(payload)

    def test_load_rejects_stage1_parity_class_mismatch(self) -> None:
        payload = _base_payload()
        payload["evidenceStage1Parity"]["failureClasses"]["unbound"] = "ev.stage1.parity.unbound"
        with self.assertRaisesRegex(ValueError, "canonical Stage 1 parity classes"):
            self._load(payload)

    def test_load_rejects_stage1_rollback_missing_trigger_class(self) -> None:
        payload = _base_payload()
        payload["evidenceStage1Rollback"]["triggerFailureClasses"] = [
            "unification.evidence_stage1.parity.missing",
            "unification.evidence_stage1.parity.mismatch",
        ]
        with self.assertRaisesRegex(ValueError, "include canonical Stage 1 parity classes"):
            self._load(payload)

    def test_load_rejects_stage1_rollback_identity_ref_aliasing(self) -> None:
        payload = _base_payload()
        payload["evidenceStage1Rollback"]["identityRefs"]["rollbackAuthorityDigestRef"] = (
            "authorityPayloadDigest"
        )
        with self.assertRaisesRegex(ValueError, "authority/rollback refs must differ"):
            self._load(payload)

    def test_load_rejects_stage2_alias_role_mismatch(self) -> None:
        payload = _base_payload()
        payload["evidenceStage2Authority"]["compatibilityAlias"]["role"] = "authority"
        with self.assertRaisesRegex(ValueError, "projection_only"):
            self._load(payload)

    def test_load_rejects_stage2_alias_window_mismatch(self) -> None:
        payload = _base_payload()
        payload["evidenceStage2Authority"]["compatibilityAlias"]["supportUntilEpoch"] = "2026-07"
        with self.assertRaisesRegex(ValueError, "rolloverEpoch"):
            self._load(payload)

    def test_load_rejects_stage2_failure_class_mismatch(self) -> None:
        payload = _base_payload()
        payload["evidenceStage2Authority"]["failureClasses"]["unbound"] = (
            "unification.evidence_stage2.not_bound"
        )
        with self.assertRaisesRegex(ValueError, "canonical Stage 2 classes"):
            self._load(payload)

    def test_load_rejects_stage2_bidir_route_obligation_mismatch(self) -> None:
        payload = _base_payload()
        payload["evidenceStage2Authority"]["bidirEvidenceRoute"]["requiredObligations"] = [
            "stability"
        ]
        with self.assertRaisesRegex(ValueError, "canonical Stage 2 kernel obligations"):
            self._load(payload)

    def test_load_rejects_rollover_without_cadence(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"].pop("rolloverCadenceMonths", None)
        with self.assertRaisesRegex(ValueError, "rolloverCadenceMonths"):
            self._load(payload)

    def test_load_rejects_freeze_with_aliases(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"] = {
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze",
        }
        with self.assertRaisesRegex(ValueError, "mode=freeze requires no active compatibility aliases"):
            self._load(payload)

    def test_load_accepts_freeze_without_aliases(self) -> None:
        payload = _base_payload()
        payload["schemaLifecycle"]["governance"] = {
            "mode": "freeze",
            "decisionRef": "decision-0105",
            "owner": "premath-core",
            "freezeReason": "release-freeze",
        }
        for family in payload["schemaLifecycle"]["kindFamilies"].values():
            family["compatibilityAliases"] = []
        payload.pop("evidenceStage2Authority", None)
        loaded = self._load(payload)
        self.assertEqual(loaded["schemaLifecycle"]["governance"]["mode"], "freeze")
        self.assertEqual(
            loaded["schemaLifecycle"]["governance"]["freezeReason"],
            "release-freeze",
        )


if __name__ == "__main__":
    unittest.main()
