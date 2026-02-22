#!/usr/bin/env python3
"""Unit tests for CI observation surface reducer/query."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import observation_surface


def _write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


class ObservationSurfaceTests(unittest.TestCase):
    def test_build_surface_accepted_state(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-observation-accept-") as tmp:
            root = Path(tmp)
            ci = root / "artifacts" / "ciwitness"
            ci.mkdir(parents=True, exist_ok=True)

            _write_json(
                ci / "latest-delta.json",
                {
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_alpha",
                    "deltaSource": "explicit",
                    "changedPaths": ["crates/premath-kernel/src/lib.rs"],
                },
            )
            _write_json(
                ci / "latest-required.json",
                {
                    "witnessKind": "ci.required.v1",
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_alpha",
                    "verdictClass": "accepted",
                    "requiredChecks": ["build", "test"],
                    "executedChecks": ["build", "test"],
                    "failureClasses": [],
                },
            )
            _write_json(
                ci / "latest-decision.json",
                {
                    "decisionKind": "ci.required.decision.v1",
                    "projectionDigest": "proj1_alpha",
                    "decision": "accept",
                    "reasonClass": "verified_accept",
                },
            )
            _write_json(
                ci / "20260221T010000Z-ci-wiring-golden.json",
                {
                    "witnessKind": "ci.instruction.v1",
                    "instructionId": "20260221T010000Z-ci-wiring-golden",
                    "instructionDigest": "instr1_alpha",
                    "verdictClass": "accepted",
                    "requiredChecks": ["ci-wiring-check"],
                    "executedChecks": ["ci-wiring-check"],
                    "failureClasses": [],
                },
            )

            surface = observation_surface.build_surface(root, ci)
            summary = surface["summary"]
            self.assertEqual(summary["state"], "accepted")
            self.assertFalse(summary["needsAttention"])
            self.assertEqual(summary["latestProjectionDigest"], "proj1_alpha")
            self.assertEqual(summary["requiredCheckCount"], 2)
            self.assertEqual(summary["changedPathCount"], 1)
            self.assertIn("coherence", summary)
            self.assertFalse(summary["coherence"]["needsAttention"])
            self.assertEqual(summary["coherence"]["instructionTyping"]["unknownCount"], 0)
            self.assertEqual(summary["coherence"]["proposalRejectClasses"]["totalRejectCount"], 0)

            events = observation_surface.build_events(surface)
            kinds = [row["kind"] for row in events]
            self.assertIn("ci.required.v1.summary", kinds)
            self.assertIn("ci.required.decision.v1.summary", kinds)
            self.assertIn("ci.instruction.v1.summary", kinds)
            self.assertIn("ci.observation.surface.v0.summary", kinds)
            self.assertIn("ci.observation.surface.v0.coherence", kinds)

    def test_build_surface_rejected_needs_attention(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-observation-reject-") as tmp:
            root = Path(tmp)
            ci = root / "artifacts" / "ciwitness"
            ci.mkdir(parents=True, exist_ok=True)

            _write_json(
                ci / "latest-required.json",
                {
                    "witnessKind": "ci.required.v1",
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_beta",
                    "verdictClass": "rejected",
                    "requiredChecks": ["baseline"],
                    "executedChecks": ["baseline"],
                    "failureClasses": ["gate_failure"],
                },
            )

            surface = observation_surface.build_surface(root, ci)
            summary = surface["summary"]
            self.assertEqual(summary["state"], "rejected")
            self.assertTrue(summary["needsAttention"])
            self.assertEqual(summary["topFailureClass"], "gate_failure")
            self.assertIn("coherence", summary)

    def test_query_modes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-observation-query-") as tmp:
            root = Path(tmp)
            ci = root / "artifacts" / "ciwitness"
            ci.mkdir(parents=True, exist_ok=True)

            _write_json(
                ci / "latest-required.json",
                {
                    "witnessKind": "ci.required.v1",
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_gamma",
                    "verdictClass": "accepted",
                    "requiredChecks": ["build"],
                    "executedChecks": ["build"],
                    "failureClasses": [],
                },
            )
            _write_json(
                ci / "20260221T020000Z-sample.json",
                {
                    "witnessKind": "ci.instruction.v1",
                    "instructionId": "20260221T020000Z-sample",
                    "instructionDigest": "instr1_gamma",
                    "verdictClass": "accepted",
                    "requiredChecks": ["build"],
                    "executedChecks": ["build"],
                    "failureClasses": [],
                },
            )

            surface = observation_surface.build_surface(root, ci)
            latest = observation_surface.query_surface(surface, mode="latest")
            self.assertEqual(latest["summary"]["state"], "running")
            self.assertIn("coherence", latest["summary"])

            needs = observation_surface.query_surface(surface, mode="needs_attention")
            self.assertIn("coherence", needs)
            self.assertIsInstance(needs["coherence"], dict)

            by_instruction = observation_surface.query_surface(
                surface,
                mode="instruction",
                instruction_id="20260221T020000Z-sample",
            )
            self.assertEqual(by_instruction["instruction"]["instructionDigest"], "instr1_gamma")

            by_projection = observation_surface.query_surface(
                surface,
                mode="projection",
                projection_digest="proj1_gamma",
            )
            self.assertEqual(by_projection["required"]["projectionDigest"], "proj1_gamma")

            with self.assertRaises(ValueError):
                observation_surface.query_surface(surface, mode="instruction")
            with self.assertRaises(ValueError):
                observation_surface.query_surface(surface, mode="projection")

    def test_coherence_projections_drive_attention(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-observation-coherence-") as tmp:
            root = Path(tmp)
            ci = root / "artifacts" / "ciwitness"
            ci.mkdir(parents=True, exist_ok=True)
            premath_dir = root / ".premath"
            premath_dir.mkdir(parents=True, exist_ok=True)

            _write_json(
                ci / "latest-delta.json",
                {
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_delta",
                    "changedPaths": ["README.md"],
                },
            )
            _write_json(
                ci / "latest-required.json",
                {
                    "witnessKind": "ci.required.v1",
                    "projectionPolicy": "ci-topos-v0",
                    "projectionDigest": "proj1_required",
                    "verdictClass": "accepted",
                    "requiredChecks": ["baseline"],
                    "executedChecks": ["baseline"],
                    "failureClasses": [],
                },
            )
            _write_json(
                ci / "latest-decision.json",
                {
                    "decisionKind": "ci.required.decision.v1",
                    "projectionDigest": "proj1_required",
                    "decision": "accept",
                    "reasonClass": "verified_accept",
                },
            )
            _write_json(
                ci / "20260221T030000Z-unknown.json",
                {
                    "witnessKind": "ci.instruction.v1",
                    "instructionId": "20260221T030000Z-unknown",
                    "instructionDigest": "instr1_unknown",
                    "instructionClassification": {
                        "state": "unknown",
                        "reason": "unrecognized_requested_checks",
                    },
                    "policyDigest": "pol1_alpha",
                    "verdictClass": "rejected",
                    "requiredChecks": [],
                    "executedChecks": [],
                    "failureClasses": ["instruction_unknown_unroutable"],
                    "runFinishedAt": "2026-02-22T01:00:00Z",
                },
            )
            _write_json(
                ci / "20260221T040000Z-proposal-reject.json",
                {
                    "witnessKind": "ci.instruction.v1",
                    "instructionId": "20260221T040000Z-proposal-reject",
                    "instructionDigest": "instr1_proposal",
                    "instructionClassification": {
                        "state": "typed",
                        "kind": "ci.gate.check",
                    },
                    "policyDigest": "pol1_beta",
                    "verdictClass": "rejected",
                    "requiredChecks": [],
                    "executedChecks": [],
                    "failureClasses": ["proposal_unbound_policy"],
                    "runFinishedAt": "2026-02-22T01:05:00Z",
                },
            )

            issue_rows = [
                {
                    "id": "bd-root",
                    "status": "open",
                    "assignee": "",
                    "updated_at": "2026-02-22T01:04:00Z",
                    "dependencies": [],
                },
                {
                    "id": "bd-stale",
                    "status": "in_progress",
                    "assignee": "agent.alpha",
                    "updated_at": "2026-02-22T01:04:00Z",
                    "lease": {
                        "lease_id": "lease1_bd-stale",
                        "owner": "agent.alpha",
                        "acquired_at": "2026-02-22T00:00:00Z",
                        "expires_at": "2026-02-22T01:00:00Z",
                    },
                    "dependencies": [],
                },
            ]
            issues_path = premath_dir / "issues.jsonl"
            with issues_path.open("w", encoding="utf-8") as f:
                for row in issue_rows:
                    f.write(json.dumps(row, ensure_ascii=False))
                    f.write("\n")

            surface = observation_surface.build_surface(root, ci, issues_path=issues_path)
            summary = surface["summary"]
            coherence = summary["coherence"]

            self.assertEqual(summary["state"], "accepted")
            self.assertTrue(summary["needsAttention"])
            self.assertIn("policy_drift", coherence["attentionReasons"])
            self.assertIn("instruction_unknown_classification", coherence["attentionReasons"])
            self.assertIn("proposal_reject_classes_present", coherence["attentionReasons"])
            self.assertIn("stale_claims", coherence["attentionReasons"])
            self.assertTrue(coherence["policyDrift"]["driftDetected"])
            self.assertEqual(coherence["instructionTyping"]["unknownCount"], 1)
            self.assertEqual(coherence["proposalRejectClasses"]["classCounts"]["proposal_unbound_policy"], 1)
            self.assertEqual(coherence["leaseHealth"]["staleCount"], 1)


if __name__ == "__main__":
    unittest.main()
