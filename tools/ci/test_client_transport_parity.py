#!/usr/bin/env python3
"""Parity tests for CI command wrapper transport surfaces."""

from __future__ import annotations

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]

CLIENTS = [
    "tools/ci/required_delta_client.py",
    "tools/ci/required_projection_client.py",
    "tools/ci/required_witness_client.py",
    "tools/ci/required_gate_ref_client.py",
    "tools/ci/required_witness_verify_client.py",
    "tools/ci/required_witness_decide_client.py",
    "tools/ci/required_decision_verify_client.py",
    "tools/ci/proposal_check_client.py",
    "tools/ci/instruction_check_client.py",
]

BANNED_PATTERNS = (
    "tempfile.NamedTemporaryFile(",
    "tempfile.TemporaryDirectory(",
    "unrecognized subcommand",
    "_extract_failure_message(",
)


class ClientTransportParityTests(unittest.TestCase):
    def test_clients_use_shared_core_transport(self) -> None:
        for rel_path in CLIENTS:
            path = ROOT / rel_path
            self.assertTrue(path.exists(), f"missing client file: {rel_path}")
            text = path.read_text(encoding="utf-8")
            self.assertIn("from core_cli_client import", text, rel_path)
            self.assertTrue(
                "run_core_json_command(" in text or "run_core_json_command_from_path(" in text,
                f"{rel_path}: expected shared core transport usage",
            )
            for pattern in BANNED_PATTERNS:
                self.assertNotIn(pattern, text, f"{rel_path}: found banned pattern {pattern!r}")

    def test_core_helper_exports_required_entrypoints(self) -> None:
        path = ROOT / "tools/ci/core_cli_client.py"
        text = path.read_text(encoding="utf-8")
        self.assertIn("def run_core_json_command(", text)
        self.assertIn("def run_core_json_command_from_path(", text)


if __name__ == "__main__":
    unittest.main()
