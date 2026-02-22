#!/usr/bin/env python3
"""Tests for deterministic pre-execution instruction reject witnesses."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import Any, Dict


REPO_ROOT = Path(__file__).resolve().parents[2]
RUN_INSTRUCTION = REPO_ROOT / "tools" / "ci" / "run_instruction.py"
POLICY_CI_SMOKE = "pol1_23a57a68a45e0c428868cce4b657206fc0bf100f4fd5b303eb0034ff29d92c9f"


def _base_envelope() -> Dict[str, Any]:
    return {
        "schema": 1,
        "intent": "Reject-witness deterministic test.",
        "scope": {"kind": "repo", "target": "premath"},
        "normalizerId": "normalizer.ci.v1",
        "policyDigest": POLICY_CI_SMOKE,
        "requestedChecks": ["ci-wiring-check"],
    }


def _run_instruction(tmp: Path, envelope: Dict[str, Any], name: str) -> Dict[str, Any]:
    instruction = tmp / f"{name}.json"
    out_dir = tmp / "witness"
    out_dir.mkdir(parents=True, exist_ok=True)
    instruction.write_text(json.dumps(envelope, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    proc = subprocess.run(
        ["python3", str(RUN_INSTRUCTION), str(instruction), "--out-dir", str(out_dir)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    witness_path = out_dir / f"{name}.json"
    payload = json.loads(witness_path.read_text(encoding="utf-8"))
    return {
        "proc": proc,
        "witness": payload,
        "witness_path": witness_path,
    }


class InstructionRejectWitnessTests(unittest.TestCase):
    def test_missing_normalizer_emits_typed_reject_witness(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-instr-reject-") as tmp:
            root = Path(tmp)
            envelope = _base_envelope()
            envelope.pop("normalizerId")
            result = _run_instruction(root, envelope, "20260222T000001Z-invalid-normalizer")

            proc = result["proc"]
            self.assertEqual(proc.returncode, 2)
            self.assertTrue(result["witness_path"].exists())

            witness = result["witness"]
            self.assertEqual(witness["witnessKind"], "ci.instruction.v1")
            self.assertEqual(witness["verdictClass"], "rejected")
            self.assertEqual(witness["rejectStage"], "pre_execution")
            self.assertEqual(witness["failureClasses"], ["instruction_invalid_normalizer"])
            self.assertEqual(witness["executedChecks"], [])
            self.assertEqual(
                witness["instructionClassification"],
                {"state": "unknown", "reason": "pre_execution_invalid"},
            )

    def test_policy_reject_emits_typed_failure_class(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-instr-reject-") as tmp:
            root = Path(tmp)
            envelope = _base_envelope()
            envelope["requestedChecks"] = ["nonexistent-check"]
            result = _run_instruction(root, envelope, "20260222T000002Z-invalid-policy")

            proc = result["proc"]
            self.assertEqual(proc.returncode, 2)
            witness = result["witness"]
            self.assertEqual(witness["rejectStage"], "pre_execution")
            self.assertEqual(witness["failureClasses"], ["instruction_check_not_allowed"])

    def test_whitespace_normalizer_rejects_pre_execution(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-instr-reject-") as tmp:
            root = Path(tmp)
            envelope = _base_envelope()
            envelope["normalizerId"] = "normalizer.ci.v1 "
            result = _run_instruction(root, envelope, "20260222T000002Z-whitespace-normalizer")

            proc = result["proc"]
            self.assertEqual(proc.returncode, 2)
            witness = result["witness"]
            self.assertEqual(witness["rejectStage"], "pre_execution")
            self.assertEqual(witness["failureClasses"], ["instruction_invalid_normalizer"])

    def test_proposal_binding_mismatch_emits_typed_failure_class(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-instr-reject-") as tmp:
            root = Path(tmp)
            envelope = _base_envelope()
            envelope["proposal"] = {
                "proposalKind": "value",
                "targetCtxRef": "ctx:repo.main",
                "targetJudgment": {"kind": "obj", "shape": "obj:type"},
                "candidateRefs": ["ref:ci.wiring"],
                "binding": {
                    "normalizerId": "normalizer.ci.v1",
                    "policyDigest": "pol1_deadbeef",
                },
            }
            result = _run_instruction(root, envelope, "20260222T000003Z-proposal-binding-mismatch")

            proc = result["proc"]
            self.assertEqual(proc.returncode, 2)
            witness = result["witness"]
            self.assertEqual(witness["rejectStage"], "pre_execution")
            self.assertEqual(witness["failureClasses"], ["proposal_binding_mismatch"])


if __name__ == "__main__":
    unittest.main()
