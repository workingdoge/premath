#!/usr/bin/env python3
"""Unit tests for provider-neutral required pipeline helpers."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from harness_escalation import EscalationResult
from harness_retry_policy import RetryDecision

import pipeline_required


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PipelineRequiredTests(unittest.TestCase):
    @staticmethod
    def _routing_policy() -> dict:
        return {
            "defaultRule": {
                "ruleId": "default",
                "maxAttempts": 1,
                "backoffClass": "none",
                "escalationAction": "stop",
                "failureClasses": tuple(),
            },
            "rulesByFailureClass": {
                "pipeline_missing_witness": {
                    "ruleId": "operational_retry",
                    "maxAttempts": 2,
                    "backoffClass": "fixed_short",
                    "escalationAction": "issue_discover",
                    "failureClasses": ("pipeline_missing_witness",),
                },
                "instruction_envelope_invalid": {
                    "ruleId": "semantic_no_retry",
                    "maxAttempts": 1,
                    "backoffClass": "none",
                    "escalationAction": "mark_blocked",
                    "failureClasses": ("instruction_envelope_invalid",),
                },
            },
        }

    def test_render_summary_writes_digest_sidecars_deterministically(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-required-") as tmp:
            root = Path(tmp)
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)

            witness_path = ciwitness / "latest-required.json"
            delta_path = ciwitness / "latest-delta.json"
            decision_path = ciwitness / "latest-decision.json"

            witness_path.write_text(
                json.dumps(
                    {
                        "projectionDigest": "proj1_test_digest",
                        "typedCoreProjectionDigest": "ev1_test_digest",
                        "authorityPayloadDigest": "proj1_test_digest",
                        "verdictClass": "accepted",
                        "requiredChecks": ["baseline", "test"],
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
            delta_path.write_text(
                json.dumps(
                    {
                        "deltaSource": "explicit",
                        "changedPaths": ["README.md", "tools/ci/pipeline_required.py"],
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
            decision_path.write_text(
                json.dumps(
                    {
                        "decision": "accept",
                        "reasonClass": "verified_accept",
                        "typedCoreProjectionDigest": "ev1_test_digest",
                        "authorityPayloadDigest": "proj1_test_digest",
                        "witnessSha256": "witness_hash",
                        "deltaSha256": "delta_hash",
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )

            summary_a = pipeline_required.render_summary(root)
            summary_b = pipeline_required.render_summary(root)
            self.assertEqual(summary_a, summary_b)

            self.assertIn("### CI Required Attestation", summary_a)
            self.assertIn("- typed authority digest: `ev1_test_digest`", summary_a)
            self.assertIn("- compatibility alias digest: `proj1_test_digest`", summary_a)
            self.assertIn("- witness verdict: `accepted`", summary_a)
            self.assertIn("- required checks: `baseline, test`", summary_a)
            self.assertIn("- delta source: `explicit`", summary_a)
            self.assertIn("- delta changed paths: `2`", summary_a)
            self.assertIn("- decision: `accept`", summary_a)
            self.assertIn("- decision reason: `verified_accept`", summary_a)
            self.assertIn("- decision typed authority: `ev1_test_digest`", summary_a)

            witness_sha_path = ciwitness / "latest-required.sha256"
            delta_sha_path = ciwitness / "latest-delta.sha256"
            decision_sha_path = ciwitness / "latest-decision.sha256"

            self.assertEqual(witness_sha_path.read_text(encoding="utf-8"), _sha256(witness_path) + "\n")
            self.assertEqual(delta_sha_path.read_text(encoding="utf-8"), _sha256(delta_path) + "\n")
            self.assertEqual(decision_sha_path.read_text(encoding="utf-8"), _sha256(decision_path) + "\n")

    def test_render_summary_reports_missing_artifacts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-required-missing-") as tmp:
            root = Path(tmp)
            (root / "artifacts" / "ciwitness").mkdir(parents=True, exist_ok=True)

            summary = pipeline_required.render_summary(root)
            self.assertIn("witness: missing", summary)
            self.assertIn("delta snapshot: missing", summary)
            self.assertIn("decision: missing", summary)

    def test_apply_provider_env_maps_github_refs(self) -> None:
        original_base = os.environ.get("PREMATH_CI_BASE_REF")
        original_head = os.environ.get("PREMATH_CI_HEAD_REF")
        original_gh_base = os.environ.get("GITHUB_BASE_REF")
        original_gh_sha = os.environ.get("GITHUB_SHA")
        try:
            os.environ.pop("PREMATH_CI_BASE_REF", None)
            os.environ.pop("PREMATH_CI_HEAD_REF", None)
            os.environ["GITHUB_BASE_REF"] = "main"
            os.environ["GITHUB_SHA"] = "abc123"

            mapped = pipeline_required.apply_provider_env()
            self.assertEqual(mapped.get("PREMATH_CI_BASE_REF"), "origin/main")
            self.assertEqual(mapped.get("PREMATH_CI_HEAD_REF"), "abc123")
            self.assertEqual(os.environ.get("PREMATH_CI_BASE_REF"), "origin/main")
            self.assertEqual(os.environ.get("PREMATH_CI_HEAD_REF"), "abc123")
        finally:
            if original_base is None:
                os.environ.pop("PREMATH_CI_BASE_REF", None)
            else:
                os.environ["PREMATH_CI_BASE_REF"] = original_base
            if original_head is None:
                os.environ.pop("PREMATH_CI_HEAD_REF", None)
            else:
                os.environ["PREMATH_CI_HEAD_REF"] = original_head
            if original_gh_base is None:
                os.environ.pop("GITHUB_BASE_REF", None)
            else:
                os.environ["GITHUB_BASE_REF"] = original_gh_base
            if original_gh_sha is None:
                os.environ.pop("GITHUB_SHA", None)
            else:
                os.environ["GITHUB_SHA"] = original_gh_sha

    def test_render_summary_includes_retry_policy_and_history(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-required-retry-") as tmp:
            root = Path(tmp)
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)
            (ciwitness / "latest-required.json").write_text(
                json.dumps(
                    {
                        "projectionDigest": "proj1_retry",
                        "typedCoreProjectionDigest": "ev1_retry",
                        "authorityPayloadDigest": "proj1_retry",
                        "verdictClass": "accepted",
                        "requiredChecks": ["baseline"],
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )

            history = (
                RetryDecision(
                    attempt=1,
                    retry=True,
                    max_attempts=2,
                    backoff_class="fixed_short",
                    escalation_action="issue_discover",
                    rule_id="operational_retry",
                    matched_failure_class="pipeline_missing_witness",
                    failure_classes=("pipeline_missing_witness",),
                ),
            )

            summary = pipeline_required.render_summary(
                root,
                retry_history=history,
                retry_policy_digest="pol1_retry",
                retry_policy_id="policy.harness.retry.v1",
                escalation=EscalationResult(
                    action="mark_blocked",
                    outcome="applied",
                    issue_id="bd-10",
                    created_issue_id=None,
                    note_digest="note1_abc",
                    witness_ref="artifacts/ciwitness/latest-required.json",
                    details="issuesPath=.premath/issues.jsonl",
                ),
            )
            self.assertIn("- retry policy: `policy.harness.retry.v1` (`pol1_retry`)", summary)
            self.assertIn("rule=operational_retry", summary)
            self.assertIn("matched=pipeline_missing_witness", summary)
            self.assertIn("- escalation: action=`mark_blocked` outcome=`applied`", summary)
            self.assertIn("- escalation issue id: `bd-10`", summary)

    def test_run_required_with_retry_prefers_process_failure_class(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-required-route-") as tmp:
            root = Path(tmp)
            expected_witness = root / "artifacts/ciwitness/latest-required.json"
            policy = self._routing_policy()
            completed = subprocess.CompletedProcess(
                args=["mise", "run", "ci-required-attested"],
                returncode=1,
                stdout="",
                stderr="instruction_envelope_invalid: malformed envelope\n",
            )
            seen: dict[str, object] = {}

            def _fake_escalation(
                _repo_root: Path,
                *,
                scope: str,
                decision: RetryDecision,
                policy: dict,
                witness_path: Path,
                **kwargs: object,
            ) -> EscalationResult:
                seen["scope"] = scope
                seen["decision"] = decision
                seen["witness_path"] = witness_path
                return EscalationResult(
                    action=decision.escalation_action,
                    outcome="applied",
                    issue_id="bd-190",
                    created_issue_id=None,
                    note_digest="note1_test",
                    witness_ref=str(witness_path),
                    details="test",
                )

            with patch("pipeline_required.subprocess.run", return_value=completed):
                with patch("pipeline_required.apply_terminal_escalation", side_effect=_fake_escalation):
                    exit_code, history, escalation = pipeline_required.run_required_with_retry(root, policy)

            self.assertEqual(exit_code, 1)
            self.assertEqual(len(history), 1)
            self.assertIsNotNone(escalation)
            decision = history[0]
            self.assertEqual(decision.rule_id, "semantic_no_retry")
            self.assertEqual(decision.matched_failure_class, "instruction_envelope_invalid")
            self.assertEqual(decision.escalation_action, "mark_blocked")
            self.assertEqual(seen.get("scope"), "required")
            self.assertEqual(seen.get("witness_path"), expected_witness)

    def test_run_required_with_retry_falls_back_to_witness_class_when_process_untyped(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-required-witness-fallback-") as tmp:
            root = Path(tmp)
            policy = self._routing_policy()
            completed = subprocess.CompletedProcess(
                args=["mise", "run", "ci-required-attested"],
                returncode=1,
                stdout="",
                stderr="[pipeline-required] command failed\n",
            )

            with patch("pipeline_required.subprocess.run", side_effect=[completed, completed]):
                with patch(
                    "pipeline_required.apply_terminal_escalation",
                    return_value=EscalationResult(
                        action="issue_discover",
                        outcome="applied",
                        issue_id="bd-190",
                        created_issue_id="bd-191",
                        note_digest="note1_test",
                        witness_ref="artifacts/ciwitness/latest-required.json",
                        details="test",
                    ),
                ):
                    exit_code, history, escalation = pipeline_required.run_required_with_retry(root, policy)

            self.assertEqual(exit_code, 1)
            self.assertIsNotNone(escalation)
            self.assertEqual(len(history), 2)
            self.assertTrue(history[0].retry)
            self.assertEqual(history[0].matched_failure_class, "pipeline_missing_witness")
            self.assertFalse(history[1].retry)
            self.assertEqual(history[1].escalation_action, "issue_discover")


if __name__ == "__main__":
    unittest.main()
