#!/usr/bin/env python3
"""Unit tests for docs coherence checker parser helpers."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_docs_coherence


class DocsCoherenceParserTests(unittest.TestCase):
    def test_parse_capability_registry(self) -> None:
        payload = {
            "schema": 1,
            "registryKind": "premath.capability_registry.v1",
            "executableCapabilities": [
                "capabilities.alpha",
                "capabilities.beta",
            ],
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-cap-registry-") as tmp:
            path = Path(tmp) / "CAPABILITY-REGISTRY.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            values = check_docs_coherence.parse_capability_registry(path)
            self.assertEqual(values, ["capabilities.alpha", "capabilities.beta"])

    def test_extract_section_between(self) -> None:
        text = "prefix START body END suffix"
        self.assertEqual(
            check_docs_coherence.extract_section_between(text, "START", "END").strip(),
            "body",
        )

    def test_parse_mise_task_commands(self) -> None:
        text = """
[tasks.baseline]
run = [
  "mise run fmt",
  "mise run test",
]

[tasks.other]
run = "echo ok"
"""
        commands = check_docs_coherence.parse_mise_task_commands(text, "baseline")
        self.assertEqual(commands, ["mise run fmt", "mise run test"])
        task_ids = check_docs_coherence.parse_baseline_task_ids_from_commands(commands)
        self.assertEqual(task_ids, ["fmt", "test"])

    def test_parse_control_plane_projection_checks(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline", "build"],
            },
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            checks = check_docs_coherence.parse_control_plane_projection_checks(path)
            self.assertEqual(checks, ["baseline", "build"])

    def test_parse_control_plane_stage1_contract(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline"],
            },
            "schemaLifecycle": {
                "kindFamilies": {
                    "controlPlaneContractKind": {
                        "compatibilityAliases": [
                            {"supportUntilEpoch": "2026-06"}
                        ]
                    }
                }
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
                "kernelComplianceSentinel": {
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
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-stage1-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            stage1 = check_docs_coherence.parse_control_plane_stage1_contract(path)
            self.assertEqual(stage1["parity"]["profileKind"], "ev.stage1.core.v1")
            self.assertEqual(stage1["rollback"]["witnessKind"], "ev.stage1.rollback.witness.v1")
            self.assertEqual(stage1["stage2"]["activeStage"], "stage2")
            self.assertIn("stability", stage1["stage2"]["requiredObligations"])

    def test_parse_control_plane_stage1_contract_rejects_missing_trigger_class(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline"],
            },
            "schemaLifecycle": {
                "kindFamilies": {
                    "controlPlaneContractKind": {
                        "compatibilityAliases": [
                            {"supportUntilEpoch": "2026-06"}
                        ]
                    }
                }
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
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-stage1-invalid-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "triggerFailureClasses missing canonical"):
                check_docs_coherence.parse_control_plane_stage1_contract(path)

    def test_parse_control_plane_stage1_contract_rejects_stage2_alias_epoch_mismatch(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline"],
            },
            "schemaLifecycle": {
                "kindFamilies": {
                    "controlPlaneContractKind": {
                        "compatibilityAliases": [
                            {"supportUntilEpoch": "2026-06"}
                        ]
                    }
                }
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
                    "supportUntilEpoch": "2026-07",
                },
                "kernelComplianceSentinel": {
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
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-stage2-invalid-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "supportUntilEpoch must align"):
                check_docs_coherence.parse_control_plane_stage1_contract(path)

    def test_parse_control_plane_stage1_contract_rejects_stage2_kernel_sentinel_mismatch(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline"],
            },
            "schemaLifecycle": {
                "kindFamilies": {
                    "controlPlaneContractKind": {
                        "compatibilityAliases": [
                            {"supportUntilEpoch": "2026-06"}
                        ]
                    }
                }
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
                "kernelComplianceSentinel": {
                    "requiredObligations": ["stability"],
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
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-stage2-sentinel-invalid-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "canonical Stage 2 kernel obligations"):
                check_docs_coherence.parse_control_plane_stage1_contract(path)

    def test_parse_doctrine_check_commands(self) -> None:
        text = """
[tasks.doctrine-check]
run = [
  "python3 tools/conformance/check_doctrine_site.py",
  "python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf",
]
"""
        commands = check_docs_coherence.parse_mise_task_commands(text, "doctrine-check")
        self.assertEqual(commands, list(check_docs_coherence.EXPECTED_DOCTRINE_CHECK_COMMANDS))

    def test_conditional_normative_entry(self) -> None:
        section = """
- `raw/SQUEAK-SITE` — runtime-location site contracts
  (normative only when `capabilities.squeak_site` is claimed).
"""
        self.assertTrue(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/SQUEAK-SITE",
                "capabilities.squeak_site",
            )
        )
        self.assertFalse(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/PREMATH-CI",
                "capabilities.ci_witnesses",
            )
        )

    def test_find_missing_markers(self) -> None:
        text = "alpha beta gamma"
        missing = check_docs_coherence.find_missing_markers(text, ("alpha", "delta", "gamma"))
        self.assertEqual(missing, ["delta"])

    def test_find_missing_markers_all_present(self) -> None:
        text = "alpha beta gamma"
        missing = check_docs_coherence.find_missing_markers(text, ("alpha", "beta"))
        self.assertEqual(missing, [])

    def test_unification_evidence_markers_all_present(self) -> None:
        text = """
### 10.2 Universal factoring rule
there MUST be one deterministic natural transformation:
`eta_F : F => Ev`
### 10.5 Fail-closed factorization boundary
`unification.evidence_factorization.missing`
`unification.evidence_factorization.ambiguous`
`unification.evidence_factorization.unbound`
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.UNIFICATION_EVIDENCE_MARKERS
        )
        self.assertEqual(missing, [])

    def test_spec_index_unified_factoring_regex_matches(self) -> None:
        text = (
            "Unified evidence factoring MUST route control-plane artifact families through\n"
            "one attested surface."
        )
        self.assertIsNotNone(
            check_docs_coherence.SPEC_INDEX_UNIFIED_FACTORIZATION_RE.search(text)
        )

    def test_unification_internalization_markers_all_present(self) -> None:
        text = """
### 10.6 Typed evidence-object internalization stages (v0)
Stage 0 (projection-locked):
Stage 1 (typed-core dual projection):
Stage 2 (canonical typed authority with compatibility alias):
Stage 3 (typed-first cleanup):
Rollback requirements:
rollback MUST NOT introduce a second authority artifact,
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.UNIFICATION_INTERNALIZATION_MARKERS
        )
        self.assertEqual(missing, [])

    def test_unification_stage1_profile_markers_all_present(self) -> None:
        text = """
#### 10.6.1 Stage 1 typed-core profile (minimum)
one profile kind identifier (for example `ev.stage1.core.v1`),
one canonical typed-core identity function over canonicalized profile bytes
#### 10.6.2 Stage 1 dual-projection parity contract
`unification.evidence_stage1.parity.missing`
`unification.evidence_stage1.parity.mismatch`
`unification.evidence_stage1.parity.unbound`
#### 10.6.3 Stage 1 deterministic rollback witness contract
`unification.evidence_stage1.rollback.precondition`
`unification.evidence_stage1.rollback.identity_drift`
`unification.evidence_stage1.rollback.unbound`
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.UNIFICATION_STAGE1_PROFILE_MARKERS
        )
        self.assertEqual(missing, [])

    def test_span_square_composition_markers_all_present(self) -> None:
        text = """
## 4. Composition Law Surface (Bicategory Profile)
`compositionLaws`
`span_identity`
`square_interchange`
digest = "sqlw1_" + SHA256(JCS(LawCore))
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.SPAN_SQUARE_COMPOSITION_MARKERS
        )
        self.assertEqual(missing, [])

    def test_premath_coherence_span_composition_regex_matches(self) -> None:
        text = (
            "accepted coverage includes span identity/associativity and square\n"
            "identity/associativity (horizontal + vertical), horizontal/vertical\n"
            "compatibility, and interchange."
        )
        self.assertIsNotNone(
            check_docs_coherence.PREMATH_COHERENCE_SPAN_COMPOSITION_RE.search(text)
        )

    def test_adjoints_cwf_sigpi_bridge_markers_all_present(self) -> None:
        text = """
## 11. CwF <-> sig\\Pi Bridge Contract (Strict vs Semantic)
`bridge.reindex`
`bridge.comprehension`
`bridge.adjoint_reflection`
bridge rules MUST NOT add new coherence
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.ADJOINTS_CWF_SIGPI_BRIDGE_MARKERS
        )
        self.assertEqual(missing, [])

    def test_premath_coherence_cwf_sigpi_bridge_regex_matches(self) -> None:
        text = "bridge routing MUST NOT introduce new coherence obligation IDs."
        self.assertIsNotNone(
            check_docs_coherence.PREMATH_COHERENCE_CWF_SIGPI_BRIDGE_RE.search(text)
        )

    def test_spec_index_cwf_sigpi_bridge_regex_matches(self) -> None:
        text = (
            "CwF<->sig\\Pi bridge mapping is normative in\n"
            "`profile/ADJOINTS-AND-SITES` §11 and MUST preserve existing obligation vocabularies."
        )
        self.assertIsNotNone(
            check_docs_coherence.SPEC_INDEX_CWF_SIGPI_BRIDGE_RE.search(text)
        )

    def test_unification_obstruction_markers_all_present(self) -> None:
        text = """
## 11. Cross-layer Obstruction Algebra (v0)
`semantic(tag)`
`structural(tag)`
`lifecycle(tag)`
`commutation(tag)`
`project_obstruction(sourceFailureClass) -> constructor`
`canonical_obstruction_class(constructor) -> canonicalFailureClass`
commutation(span_square_commutation)
`obs.<family>.<tag>`
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.UNIFICATION_OBSTRUCTION_MARKERS
        )
        self.assertEqual(missing, [])

    def test_capability_vectors_obstruction_regex_matches(self) -> None:
        text = "cross-layer obstruction rows roundtrip deterministically."
        self.assertIsNotNone(
            check_docs_coherence.CAPABILITY_VECTORS_OBSTRUCTION_RE.search(text)
        )


if __name__ == "__main__":
    unittest.main()
