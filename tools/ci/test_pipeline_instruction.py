#!/usr/bin/env python3
"""Unit tests for provider-neutral instruction pipeline helpers."""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

from harness_escalation import EscalationResult
from harness_retry_policy import RetryDecision

import pipeline_instruction


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PipelineInstructionTests(unittest.TestCase):
    def test_render_summary_writes_witness_digest(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-") as tmp:
            root = Path(tmp)
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)

            instruction_id = "20260221T010000Z-ci-wiring-golden"
            witness_path = ciwitness / f"{instruction_id}.json"
            witness_path.write_text(
                json.dumps(
                    {
                        "instructionId": instruction_id,
                        "instructionDigest": "instr_digest",
                        "normalizerId": "normalizer.test.v1",
                        "verdictClass": "accepted",
                        "requiredChecks": ["ci-wiring-check"],
                        "executedChecks": ["ci-wiring-check"],
                    },
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )

            summary_a = pipeline_instruction.render_summary(root, instruction_id)
            summary_b = pipeline_instruction.render_summary(root, instruction_id)
            self.assertEqual(summary_a, summary_b)

            self.assertIn("### CI Instruction Witness", summary_a)
            self.assertIn(f"- instruction id: `{instruction_id}`", summary_a)
            self.assertIn("- instruction digest: `instr_digest`", summary_a)
            self.assertIn("- normalizer id: `normalizer.test.v1`", summary_a)
            self.assertIn("- verdict: `accepted`", summary_a)
            self.assertIn("- required checks: `ci-wiring-check`", summary_a)
            self.assertIn("- executed checks: `ci-wiring-check`", summary_a)

            sha_path = ciwitness / f"{instruction_id}.sha256"
            self.assertEqual(sha_path.read_text(encoding="utf-8"), _sha256(witness_path) + "\n")

    def test_render_summary_reports_missing_witness(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-missing-") as tmp:
            root = Path(tmp)
            (root / "artifacts" / "ciwitness").mkdir(parents=True, exist_ok=True)

            summary = pipeline_instruction.render_summary(root, "missing-instruction")
            self.assertIn("witness: missing", summary)

    def test_resolve_instruction_and_instruction_id(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-resolve-") as tmp:
            root = Path(tmp)
            instruction = root / "instructions" / "sample.json"
            instruction.parent.mkdir(parents=True, exist_ok=True)
            instruction.write_text("{}", encoding="utf-8")

            resolved_rel = pipeline_instruction._resolve_instruction(root, Path("instructions/sample.json"))
            resolved_abs = pipeline_instruction._resolve_instruction(root, instruction)
            expected = instruction.resolve()
            self.assertEqual(resolved_rel, expected)
            self.assertEqual(resolved_abs, instruction)

            self.assertEqual(pipeline_instruction._instruction_id(instruction), "sample")
            with self.assertRaises(ValueError):
                pipeline_instruction._instruction_id(root / "instructions" / "sample.txt")

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

            mapped = pipeline_instruction.apply_provider_env()
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
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-retry-") as tmp:
            root = Path(tmp)
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)

            instruction_id = "20260221T020000Z-retry"
            (ciwitness / f"{instruction_id}.json").write_text(
                json.dumps(
                    {
                        "instructionId": instruction_id,
                        "instructionDigest": "instr_retry",
                        "normalizerId": "normalizer.retry.v1",
                        "verdictClass": "rejected",
                        "requiredChecks": ["ci-wiring-check"],
                        "executedChecks": ["ci-wiring-check"],
                        "failureClasses": ["pipeline_missing_witness"],
                        "operationalFailureClasses": ["pipeline_missing_witness"],
                        "semanticFailureClasses": [],
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
            summary = pipeline_instruction.render_summary(
                root,
                instruction_id,
                retry_history=history,
                retry_policy_digest="pol1_retry",
                retry_policy_id="policy.harness.retry.v1",
                escalation=EscalationResult(
                    action="issue_discover",
                    outcome="applied",
                    issue_id="bd-10",
                    created_issue_id="bd-11",
                    note_digest="note1_abc",
                    witness_ref=f"artifacts/ciwitness/{instruction_id}.json",
                    details="issuesPath=.premath/issues.jsonl",
                ),
            )
            self.assertIn("- retry policy: `policy.harness.retry.v1` (`pol1_retry`)", summary)
            self.assertIn("rule=operational_retry", summary)
            self.assertIn("matched=pipeline_missing_witness", summary)
            self.assertIn("- escalation: action=`issue_discover` outcome=`applied`", summary)
            self.assertIn("- escalation created issue id: `bd-11`", summary)


if __name__ == "__main__":
    unittest.main()
