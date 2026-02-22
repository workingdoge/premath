#!/usr/bin/env python3
"""Unit tests for deterministic harness escalation bridge."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Sequence

import harness_escalation
from harness_retry_policy import RetryDecision


def _decision(action: str) -> RetryDecision:
    return RetryDecision(
        attempt=1,
        retry=False,
        max_attempts=1,
        backoff_class="none",
        escalation_action=action,
        rule_id="semantic_no_retry",
        matched_failure_class="check_failed",
        failure_classes=("check_failed",),
    )


class _RunRecorder:
    def __init__(self, payload: dict, returncode: int = 0, stderr: str = "") -> None:
        self.payload = payload
        self.returncode = returncode
        self.stderr = stderr
        self.calls: list[list[str]] = []

    def __call__(
        self,
        cmd: Sequence[str],
        cwd: Path,
        capture_output: bool,
        text: bool,
    ) -> subprocess.CompletedProcess[str]:
        self.calls.append(list(cmd))
        stdout = json.dumps(self.payload) + "\n" if self.returncode == 0 else ""
        return subprocess.CompletedProcess(
            list(cmd),
            self.returncode,
            stdout=stdout,
            stderr=self.stderr,
        )


class HarnessEscalationTests(unittest.TestCase):
    def test_stop_action_produces_terminal_result(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        result = harness_escalation.apply_terminal_escalation(
            repo_root,
            scope="required",
            decision=_decision("stop"),
            policy={"policyId": "policy.harness.retry.v1", "policyDigest": "pol1_x"},
            witness_path=repo_root / "artifacts/ciwitness/latest-required.json",
            env={},
        )
        self.assertEqual(result.action, "stop")
        self.assertEqual(result.outcome, "stop")
        self.assertIsNone(result.issue_id)

    def test_issue_discover_applies_when_active_issue_present(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-escalate-discover-") as tmp:
            root = Path(tmp)
            witness = root / "artifacts/ciwitness/latest-required.json"
            witness.parent.mkdir(parents=True, exist_ok=True)
            witness.write_text("{}", encoding="utf-8")
            issues = root / ".premath/issues.jsonl"
            issues.parent.mkdir(parents=True, exist_ok=True)
            issues.write_text(
                json.dumps({"id": "bd-parent", "title": "Parent", "notes": ""}) + "\n",
                encoding="utf-8",
            )

            recorder = _RunRecorder(
                {
                    "action": "issue.discover",
                    "issue": {"id": "bd-child"},
                }
            )
            result = harness_escalation.apply_terminal_escalation(
                root,
                scope="required",
                decision=_decision("issue_discover"),
                policy={"policyId": "policy.harness.retry.v1", "policyDigest": "pol1_x"},
                witness_path=witness,
                env={"PREMATH_ACTIVE_ISSUE_ID": "bd-parent"},
                run_process=recorder,
                issues_path=issues,
            )
            self.assertEqual(result.outcome, "applied")
            self.assertEqual(result.issue_id, "bd-parent")
            self.assertEqual(result.created_issue_id, "bd-child")
            self.assertTrue(any("discover" in call for call in recorder.calls))

    def test_mark_blocked_updates_notes_and_status(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-escalate-blocked-") as tmp:
            root = Path(tmp)
            witness = root / "artifacts/ciwitness/latest-required.json"
            witness.parent.mkdir(parents=True, exist_ok=True)
            witness.write_text("{}", encoding="utf-8")
            issues = root / ".premath/issues.jsonl"
            issues.parent.mkdir(parents=True, exist_ok=True)
            issues.write_text(
                json.dumps({"id": "bd-parent", "title": "Parent", "notes": "seed"}) + "\n",
                encoding="utf-8",
            )
            recorder = _RunRecorder(
                {
                    "action": "issue.update",
                    "issue": {"id": "bd-parent", "status": "blocked"},
                }
            )
            result = harness_escalation.apply_terminal_escalation(
                root,
                scope="instruction",
                decision=_decision("mark_blocked"),
                policy={"policyId": "policy.harness.retry.v1", "policyDigest": "pol1_x"},
                witness_path=witness,
                env={"PREMATH_ISSUE_ID": "bd-parent"},
                run_process=recorder,
                issues_path=issues,
            )
            self.assertEqual(result.outcome, "applied")
            self.assertEqual(result.issue_id, "bd-parent")
            update_cmd = recorder.calls[0]
            self.assertIn("update", update_cmd)
            self.assertIn("--status", update_cmd)
            self.assertIn("blocked", update_cmd)
            self.assertIn("--notes", update_cmd)
            notes_arg = update_cmd[update_cmd.index("--notes") + 1]
            self.assertIn("seed", notes_arg)
            self.assertIn("[harness-escalation]", notes_arg)

    def test_missing_active_issue_context_is_skipped(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        witness = repo_root / "artifacts/ciwitness/latest-required.json"
        result = harness_escalation.apply_terminal_escalation(
            repo_root,
            scope="required",
            decision=_decision("issue_discover"),
            policy={"policyId": "policy.harness.retry.v1", "policyDigest": "pol1_x"},
            witness_path=witness,
            env={},
        )
        self.assertEqual(result.outcome, "skipped_missing_issue_context")
        self.assertIsNone(result.issue_id)

    def test_issue_command_failure_raises(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-harness-escalate-fail-") as tmp:
            root = Path(tmp)
            witness = root / "artifacts/ciwitness/latest-required.json"
            witness.parent.mkdir(parents=True, exist_ok=True)
            witness.write_text("{}", encoding="utf-8")
            issues = root / ".premath/issues.jsonl"
            issues.parent.mkdir(parents=True, exist_ok=True)
            issues.write_text(
                json.dumps({"id": "bd-parent", "title": "Parent", "notes": ""}) + "\n",
                encoding="utf-8",
            )
            recorder = _RunRecorder({}, returncode=1, stderr="error: failed")
            with self.assertRaises(harness_escalation.EscalationError):
                harness_escalation.apply_terminal_escalation(
                    root,
                    scope="required",
                    decision=_decision("issue_discover"),
                    policy={"policyId": "policy.harness.retry.v1", "policyDigest": "pol1_x"},
                    witness_path=witness,
                    env={"PREMATH_ACTIVE_ISSUE_ID": "bd-parent"},
                    run_process=recorder,
                    issues_path=issues,
                )


if __name__ == "__main__":
    unittest.main()
