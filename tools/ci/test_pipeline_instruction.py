#!/usr/bin/env python3
"""Unit tests for provider-neutral instruction pipeline helpers."""

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

import pipeline_instruction


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PipelineInstructionTests(unittest.TestCase):
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
                "instruction_envelope_invalid_shape": {
                    "ruleId": "semantic_no_retry",
                    "maxAttempts": 1,
                    "backoffClass": "none",
                    "escalationAction": "mark_blocked",
                    "failureClasses": ("instruction_envelope_invalid_shape",),
                },
                "governance.eval_gate_unmet": {
                    "ruleId": "governance_no_retry",
                    "maxAttempts": 1,
                    "backoffClass": "none",
                    "escalationAction": "mark_blocked",
                    "failureClasses": ("governance.eval_gate_unmet",),
                },
                "kcir_mapping_legacy_encoding_authority_violation": {
                    "ruleId": "kcir_mapping_no_retry",
                    "maxAttempts": 1,
                    "backoffClass": "none",
                    "escalationAction": "mark_blocked",
                    "failureClasses": ("kcir_mapping_legacy_encoding_authority_violation",),
                },
            },
        }

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

    def test_run_instruction_once_collects_validate_and_reject_failure_classes(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        instruction = repo_root / "instructions" / "sample.json"
        validate = subprocess.CompletedProcess(
            args=["python3", "tools/ci/check_instruction_envelope.py", str(instruction)],
            returncode=1,
            stdout="",
            stderr=f"{instruction}: proposal_binding_mismatch: mismatch\n",
        )
        reject = subprocess.CompletedProcess(
            args=["python3", "tools/ci/run_instruction.py", str(instruction)],
            returncode=2,
            stdout="",
            stderr=(
                "[error] invalid instruction envelope: "
                "instruction_envelope_invalid: malformed envelope\n"
            ),
        )

        with patch("pipeline_instruction.subprocess.run", side_effect=[validate, reject]):
            exit_code, failure_classes = pipeline_instruction.run_instruction_once(
                repo_root,
                instruction,
                allow_failure=False,
            )

        self.assertEqual(exit_code, 2)
        self.assertEqual(
            failure_classes,
            (
                "instruction_envelope_invalid",
                "proposal_binding_mismatch",
            ),
        )

    def test_run_instruction_with_retry_prefers_process_failure_class(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-route-") as tmp:
            root = Path(tmp)
            policy = self._routing_policy()
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

            with patch(
                "pipeline_instruction.run_instruction_once",
                return_value=(1, ("instruction_envelope_invalid_shape",)),
            ):
                with patch("pipeline_instruction.apply_terminal_escalation", side_effect=_fake_escalation):
                    exit_code, history, escalation = pipeline_instruction.run_instruction_with_retry(
                        root,
                        root / "instructions" / "sample.json",
                        "sample",
                        policy,
                        allow_failure=False,
                    )

            self.assertEqual(exit_code, 1)
            self.assertEqual(len(history), 1)
            self.assertIsNotNone(escalation)
            decision = history[0]
            self.assertEqual(decision.rule_id, "semantic_no_retry")
            self.assertEqual(decision.matched_failure_class, "instruction_envelope_invalid_shape")
            self.assertEqual(decision.escalation_action, "mark_blocked")
            self.assertEqual(seen.get("scope"), "instruction")
            self.assertEqual(
                seen.get("witness_path"),
                root / "artifacts/ciwitness/sample.json",
            )

    def test_run_instruction_with_retry_falls_back_to_witness_class_when_process_untyped(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-witness-fallback-") as tmp:
            root = Path(tmp)
            policy = self._routing_policy()
            with patch(
                "pipeline_instruction.run_instruction_once",
                side_effect=[(1, tuple()), (1, tuple())],
            ):
                with patch(
                    "pipeline_instruction.apply_terminal_escalation",
                    return_value=EscalationResult(
                        action="issue_discover",
                        outcome="applied",
                        issue_id="bd-190",
                        created_issue_id="bd-191",
                        note_digest="note1_test",
                        witness_ref="artifacts/ciwitness/sample.json",
                        details="test",
                    ),
                ):
                    with patch(
                        "pipeline_instruction.evaluate_instruction_mapping",
                        return_value=pipeline_instruction.MappingGateReport(
                            profile_id="cp.kcir.mapping.v0",
                            declared_rows=(
                                "instructionEnvelope",
                                "proposalPayload",
                                "coherenceObligations",
                                "coherenceCheckPayload",
                                "doctrineRouteBinding",
                                "requiredDecisionInput",
                            ),
                            checked_rows=(
                                "instructionEnvelope",
                                "coherenceObligations",
                                "doctrineRouteBinding",
                                "proposalPayload",
                            ),
                            failure_classes=tuple(),
                        ),
                    ):
                        exit_code, history, escalation = pipeline_instruction.run_instruction_with_retry(
                            root,
                            root / "instructions" / "sample.json",
                            "sample",
                            policy,
                            allow_failure=False,
                        )

            self.assertEqual(exit_code, 1)
            self.assertIsNotNone(escalation)
            self.assertEqual(len(history), 2)
            self.assertTrue(history[0].retry)
            self.assertEqual(history[0].matched_failure_class, "pipeline_missing_witness")
            self.assertFalse(history[1].retry)
            self.assertEqual(history[1].escalation_action, "issue_discover")

    def test_run_instruction_with_retry_applies_governance_gate_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-governance-") as tmp:
            root = Path(tmp)
            policy = self._routing_policy()
            with patch(
                "pipeline_instruction.run_instruction_once",
                return_value=(0, tuple()),
            ):
                with patch(
                    "pipeline_instruction.governance_failure_classes",
                    return_value=("governance.eval_gate_unmet",),
                ):
                    with patch(
                        "pipeline_instruction.apply_terminal_escalation",
                        return_value=EscalationResult(
                            action="mark_blocked",
                            outcome="applied",
                            issue_id="bd-190",
                            created_issue_id=None,
                            note_digest="note1_test",
                            witness_ref="artifacts/ciwitness/sample.json",
                            details="governance gate unmet",
                        ),
                    ):
                        exit_code, history, escalation = pipeline_instruction.run_instruction_with_retry(
                            root,
                            root / "instructions" / "sample.json",
                            "sample",
                            policy,
                            allow_failure=False,
                        )

            self.assertEqual(exit_code, 1)
            self.assertIsNotNone(escalation)
            self.assertEqual(len(history), 1)
            self.assertEqual(history[0].rule_id, "governance_no_retry")
            self.assertEqual(history[0].matched_failure_class, "governance.eval_gate_unmet")
            self.assertEqual(history[0].escalation_action, "mark_blocked")

    def test_run_instruction_with_retry_applies_kcir_mapping_gate_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-instruction-kcir-gate-") as tmp:
            root = Path(tmp)
            instruction_path = root / "instructions" / "sample.json"
            instruction_path.parent.mkdir(parents=True, exist_ok=True)
            instruction_path.write_text(
                json.dumps({"llmProposal": {"proposalKind": "value"}}, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            ciwitness = root / "artifacts" / "ciwitness"
            ciwitness.mkdir(parents=True, exist_ok=True)
            (ciwitness / "sample.json").write_text(
                json.dumps(
                    {
                        "instructionDigest": "instr_demo",
                        "normalizerId": "normalizer.demo.v1",
                        "policyDigest": "pol1_demo",
                        "proposalIngest": {
                            "proposalDigest": "prop_demo",
                            "proposalKcirRef": "kcir1_demo",
                        },
                    },
                    ensure_ascii=False,
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            policy = self._routing_policy()

            with patch(
                "pipeline_instruction.run_instruction_once",
                return_value=(0, tuple()),
            ):
                with patch(
                    "pipeline_instruction.governance_failure_classes",
                    return_value=tuple(),
                ):
                    with patch(
                        "pipeline_instruction.evaluate_instruction_mapping",
                        return_value=pipeline_instruction.MappingGateReport(
                            profile_id="cp.kcir.mapping.v0",
                            declared_rows=(
                                "instructionEnvelope",
                                "proposalPayload",
                                "coherenceObligations",
                                "coherenceCheckPayload",
                                "doctrineRouteBinding",
                                "requiredDecisionInput",
                            ),
                            checked_rows=(
                                "instructionEnvelope",
                                "coherenceObligations",
                                "doctrineRouteBinding",
                                "proposalPayload",
                            ),
                            failure_classes=(
                                "kcir_mapping_legacy_encoding_authority_violation",
                            ),
                        ),
                    ):
                        with patch(
                            "pipeline_instruction.apply_terminal_escalation",
                            return_value=EscalationResult(
                                action="mark_blocked",
                                outcome="applied",
                                issue_id="bd-190",
                                created_issue_id=None,
                                note_digest="note1_test",
                                witness_ref="artifacts/ciwitness/sample.json",
                                details="kcir mapping gate unmet",
                            ),
                        ):
                            exit_code, history, escalation = pipeline_instruction.run_instruction_with_retry(
                                root,
                                instruction_path,
                                "sample",
                                policy,
                                allow_failure=False,
                            )

            self.assertEqual(exit_code, 1)
            self.assertIsNotNone(escalation)
            self.assertEqual(len(history), 1)
            self.assertEqual(history[0].rule_id, "kcir_mapping_no_retry")
            self.assertEqual(
                history[0].matched_failure_class,
                "kcir_mapping_legacy_encoding_authority_violation",
            )
            self.assertEqual(history[0].escalation_action, "mark_blocked")


if __name__ == "__main__":
    unittest.main()
