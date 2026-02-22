#!/usr/bin/env python3
"""Unit tests for GitHub branch/ruleset policy checker."""

from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

import check_branch_policy


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "specs" / "process" / "GITHUB-BRANCH-POLICY.json"
GOLDEN_RULES_PATH = ROOT / "tests" / "ci" / "fixtures" / "branch-policy" / "effective-main-rules-golden.json"


class BranchPolicyTests(unittest.TestCase):
    def test_parse_policy_accepts_tracked_policy(self) -> None:
        policy = check_branch_policy.parse_policy(POLICY_PATH)
        self.assertEqual(policy["policyKind"], "premath.github.branch_policy.v1")
        self.assertEqual(policy["repository"], "workingdoge/premath")
        self.assertEqual(policy["branch"], "main")
        self.assertIn("ci-required", policy["requiredStatusChecks"])

    def test_evaluate_policy_accepts_golden_fixture(self) -> None:
        policy = check_branch_policy.parse_policy(POLICY_PATH)
        payload = check_branch_policy.load_json(GOLDEN_RULES_PATH)
        errors, details = check_branch_policy.evaluate_policy(policy, payload)
        self.assertEqual(errors, [])
        self.assertIn("required_status_checks", details["ruleTypes"])
        self.assertIn("ci-required", details["requiredStatusChecks"])
        self.assertTrue(details["strictStatusChecks"])
        self.assertEqual(details["bypassActors"], [])

    def test_evaluate_policy_rejects_missing_required_status_check(self) -> None:
        policy = check_branch_policy.parse_policy(POLICY_PATH)
        payload = check_branch_policy.load_json(GOLDEN_RULES_PATH)
        broken = copy.deepcopy(payload)
        rules = broken["rules"]
        for rule in rules:
            if rule.get("type") == "required_status_checks":
                rule["parameters"]["required_status_checks"] = [{"context": "other-check"}]
        errors, _details = check_branch_policy.evaluate_policy(policy, broken)
        self.assertIn("missing required status check context: ci-required", errors)

    def test_collect_bypass_actors_detects_nested_surfaces(self) -> None:
        payload = {
            "rules": [
                {
                    "type": "pull_request",
                    "parameters": {
                        "bypass_actors": [
                            {"actor_type": "RepositoryRole", "actor_id": 5},
                        ],
                        "bypass_pull_request_allowances": {
                            "users": [{"login": "alice"}],
                            "teams": [],
                            "apps": [],
                        },
                    },
                }
            ]
        }
        actors = check_branch_policy.collect_bypass_actors(payload)
        self.assertIn("RepositoryRole:5", actors)
        self.assertIn("users:alice", actors)

    def test_parse_policy_rejects_invalid_kind(self) -> None:
        with tempfile.TemporaryDirectory(prefix="branch-policy-test-") as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(
                '{"schema":1,"policyKind":"wrong","policyId":"x","repository":"o/r","branch":"main",'
                '"requiredRuleTypes":["pull_request"],"requiredStatusChecks":["ci-required"],'
                '"strictStatusChecks":true,"requirePullRequest":true,"forbidBypassActors":true}',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "policyKind"):
                check_branch_policy.parse_policy(path)


if __name__ == "__main__":
    unittest.main()
