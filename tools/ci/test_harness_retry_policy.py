#!/usr/bin/env python3
"""Unit tests for deterministic harness retry-policy helpers."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

import harness_retry_policy


class HarnessRetryPolicyTests(unittest.TestCase):
    def test_load_retry_policy_accepts_repository_policy(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        policy = harness_retry_policy.load_retry_policy(repo_root)
        self.assertEqual(policy.get("policyKind"), "ci.harness.retry.policy.v1")
        self.assertEqual(policy.get("policyId"), "policy.harness.retry.v1")
        self.assertEqual(
            policy.get("policyDigest"),
            "pol1_85f35a1dd21bbfd1bd8c0b1e999303dba5f905e75ffe7eb8be25d581296ec0c7",
        )

    def test_resolve_retry_decision_semantic_no_retry(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        policy = harness_retry_policy.load_retry_policy(repo_root)
        decision = harness_retry_policy.resolve_retry_decision(
            policy,
            ("check_failed",),
            attempt=1,
        )
        self.assertFalse(decision.retry)
        self.assertEqual(decision.max_attempts, 1)
        self.assertEqual(decision.rule_id, "semantic_no_retry")
        self.assertEqual(decision.escalation_action, "mark_blocked")

    def test_resolve_retry_decision_operational_retry_then_escalate(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        policy = harness_retry_policy.load_retry_policy(repo_root)
        first = harness_retry_policy.resolve_retry_decision(
            policy,
            ("pipeline_missing_witness",),
            attempt=1,
        )
        second = harness_retry_policy.resolve_retry_decision(
            policy,
            ("pipeline_missing_witness",),
            attempt=2,
        )
        self.assertTrue(first.retry)
        self.assertEqual(first.max_attempts, 2)
        self.assertEqual(first.rule_id, "operational_retry")
        self.assertFalse(second.retry)
        self.assertEqual(second.escalation_action, "issue_discover")

    def test_resolve_retry_decision_prefers_input_order_for_rule_match(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        policy = harness_retry_policy.load_retry_policy(repo_root)
        decision = harness_retry_policy.resolve_retry_decision(
            policy,
            ("proposal_binding_mismatch", "pipeline_missing_witness"),
            attempt=1,
        )
        self.assertFalse(decision.retry)
        self.assertEqual(decision.rule_id, "semantic_no_retry")
        self.assertEqual(decision.matched_failure_class, "proposal_binding_mismatch")
        self.assertEqual(decision.escalation_action, "mark_blocked")

    def test_resolve_retry_decision_kcir_mapping_no_retry(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        policy = harness_retry_policy.load_retry_policy(repo_root)
        decision = harness_retry_policy.resolve_retry_decision(
            policy,
            (
                "kcir_mapping_legacy_encoding_authority_violation",
                "kcir_mapping_legacy_encoding_authority_violation",
            ),
            attempt=1,
        )
        self.assertFalse(decision.retry)
        self.assertEqual(decision.max_attempts, 1)
        self.assertEqual(decision.rule_id, "kcir_mapping_no_retry")
        self.assertEqual(
            decision.matched_failure_class,
            "kcir_mapping_legacy_encoding_authority_violation",
        )
        self.assertEqual(decision.escalation_action, "mark_blocked")
        self.assertEqual(
            decision.failure_classes,
            ("kcir_mapping_legacy_encoding_authority_violation",),
        )

    def test_failure_classes_from_witness_payload_union(self) -> None:
        payload = {
            "verdictClass": "rejected",
            "failureClasses": ["check_failed"],
            "operationalFailureClasses": ["check_failed", "pipeline_missing_witness"],
            "semanticFailureClasses": ["proposal_unbound_policy"],
        }
        classes = harness_retry_policy.failure_classes_from_witness_payload(payload)
        self.assertEqual(
            classes,
            ("check_failed", "pipeline_missing_witness", "proposal_unbound_policy"),
        )

    def test_failure_classes_from_witness_path_handles_missing_and_invalid(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-retry-") as tmp:
            root = Path(tmp)
            witness = root / "witness.json"
            missing_classes = harness_retry_policy.failure_classes_from_witness_path(witness)
            self.assertEqual(missing_classes, ("pipeline_missing_witness",))

            witness.write_text("{bad json", encoding="utf-8")
            invalid_classes = harness_retry_policy.failure_classes_from_witness_path(witness)
            self.assertEqual(invalid_classes, ("pipeline_invalid_witness_json",))

            witness.write_text(json.dumps({"verdictClass": "rejected"}) + "\n", encoding="utf-8")
            missing_failure_class = harness_retry_policy.failure_classes_from_witness_path(witness)
            self.assertEqual(missing_failure_class, ("pipeline_missing_failure_class",))

    def test_failure_classes_from_completed_process_extracts_typed_classes(self) -> None:
        completed = subprocess.CompletedProcess(
            args=["mise", "run", "ci-required-attested"],
            returncode=2,
            stdout=json.dumps(
                {
                    "decision": "reject",
                    "reasonClass": "required_witness_decide_invalid",
                },
                indent=2,
                ensure_ascii=False,
            )
            + "\n",
            stderr=(
                "[verify-decision] core verify failed: "
                "required_decision_verify_invalid: digest mismatch\n"
            ),
        )
        classes = harness_retry_policy.failure_classes_from_completed_process(completed)
        self.assertEqual(
            classes,
            (
                "required_decision_verify_invalid",
                "required_witness_decide_invalid",
            ),
        )

    def test_classify_failure_classes_prefers_process_failure_class(self) -> None:
        classes = harness_retry_policy.classify_failure_classes(
            ("pipeline_missing_witness",),
            ("proposal_binding_mismatch",),
        )
        self.assertEqual(classes, ("proposal_binding_mismatch", "pipeline_missing_witness"))


if __name__ == "__main__":
    unittest.main()
