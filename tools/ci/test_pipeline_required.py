#!/usr/bin/env python3
"""Unit tests for provider-neutral required pipeline helpers."""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from harness_escalation import EscalationResult
from harness_retry_policy import RetryDecision

import pipeline_required


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PipelineRequiredTests(unittest.TestCase):
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
            self.assertIn("- projection digest: `proj1_test_digest`", summary_a)
            self.assertIn("- witness verdict: `accepted`", summary_a)
            self.assertIn("- required checks: `baseline, test`", summary_a)
            self.assertIn("- delta source: `explicit`", summary_a)
            self.assertIn("- delta changed paths: `2`", summary_a)
            self.assertIn("- decision: `accept`", summary_a)
            self.assertIn("- decision reason: `verified_accept`", summary_a)

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


if __name__ == "__main__":
    unittest.main()
