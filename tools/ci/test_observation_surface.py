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

            events = observation_surface.build_events(surface)
            kinds = [row["kind"] for row in events]
            self.assertIn("ci.required.v1.summary", kinds)
            self.assertIn("ci.required.decision.v1.summary", kinds)
            self.assertIn("ci.instruction.v1.summary", kinds)
            self.assertIn("ci.observation.surface.v0.summary", kinds)

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


if __name__ == "__main__":
    unittest.main()
