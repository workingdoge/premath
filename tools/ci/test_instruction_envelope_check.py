#!/usr/bin/env python3
"""Unit tests for instruction-envelope command materialization."""

from __future__ import annotations

import unittest
from pathlib import Path

from control_plane_contract import INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT
from pipeline_instruction import _materialize_instruction_command


class InstructionEnvelopeCheckTests(unittest.TestCase):
    def test_canonical_entrypoint_is_core_instruction_check(self) -> None:
        self.assertIn("instruction-check", INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT)
        self.assertIn("$INSTRUCTION_PATH", INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT)
        self.assertIn("$REPO_ROOT", INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT)

    def test_materialize_replaces_instruction_and_repo_placeholders(self) -> None:
        repo_root = Path("/repo")
        instruction = Path("/repo/instructions/sample.json")
        cmd = _materialize_instruction_command(
            INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT,
            instruction_path=instruction,
            repo_root=repo_root,
            append_instruction_when_missing=True,
        )
        self.assertIn(str(instruction), cmd)
        self.assertIn(str(repo_root), cmd)
        self.assertEqual(cmd.count(str(instruction)), 1)

    def test_materialize_appends_instruction_when_placeholder_missing(self) -> None:
        repo_root = Path("/repo")
        instruction = Path("/repo/instructions/sample.json")
        cmd = _materialize_instruction_command(
            ("python3", "tools/ci/run_instruction.py"),
            instruction_path=instruction,
            repo_root=repo_root,
            append_instruction_when_missing=True,
        )
        self.assertEqual(cmd[-1], str(instruction))


if __name__ == "__main__":
    unittest.main()
