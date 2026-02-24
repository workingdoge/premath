#!/usr/bin/env python3
"""Unit tests for provider-neutral pipeline wiring checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_pipeline_wiring


def _write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _minimal_repo(root: Path) -> None:
    required_cmd = check_pipeline_wiring._render_entrypoint(
        check_pipeline_wiring.PIPELINE_WRAPPER_REQUIRED_ENTRYPOINT
    )
    instruction_cmd = check_pipeline_wiring._render_entrypoint(
        check_pipeline_wiring.PIPELINE_WRAPPER_INSTRUCTION_ENTRYPOINT
    )
    _write(
        root / ".github/workflows/baseline.yml",
        f"""
name: baseline
jobs:
  baseline:
    steps:
      - name: run
        run: {required_cmd}
""".strip()
        + "\n",
    )
    _write(
        root / ".github/workflows/instruction.yml",
        f"""
name: instruction
jobs:
  instruction:
    steps:
      - name: run
        run: {instruction_cmd}
""".strip()
        + "\n",
    )
    _write(
        root / "tools/ci/pipeline_required.py",
        (
            "from governance_gate import "
            f"{check_pipeline_wiring.PIPELINE_WRAPPER_REQUIRED_GOVERNANCE_HOOK}\n"
            "from kcir_mapping_gate import "
            f"{check_pipeline_wiring.PIPELINE_WRAPPER_REQUIRED_KCIR_MAPPING_HOOK}\n"
        ),
    )
    _write(
        root / "tools/ci/pipeline_instruction.py",
        (
            "from governance_gate import "
            f"{check_pipeline_wiring.PIPELINE_WRAPPER_INSTRUCTION_GOVERNANCE_HOOK}\n"
            "from kcir_mapping_gate import "
            f"{check_pipeline_wiring.PIPELINE_WRAPPER_INSTRUCTION_KCIR_MAPPING_HOOK}\n"
        ),
    )


class PipelineWiringTests(unittest.TestCase):
    def test_evaluate_pipeline_wiring_accepts_contract_bound_wrappers(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-ok-") as tmp:
            root = Path(tmp)
            _minimal_repo(root)
            errors, failure_classes = check_pipeline_wiring.evaluate_pipeline_wiring(root)
            self.assertEqual(errors, [])
            self.assertEqual(failure_classes, [])

    def test_evaluate_pipeline_wiring_reports_missing_wrapper_entrypoint(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-missing-entry-") as tmp:
            root = Path(tmp)
            _minimal_repo(root)
            _write(
                root / ".github/workflows/baseline.yml",
                """
name: baseline
jobs:
  baseline:
    steps:
      - name: run
        run: python3 tools/ci/run_required_checks.py
""".strip()
                + "\n",
            )
            errors, failure_classes = check_pipeline_wiring.evaluate_pipeline_wiring(root)
            self.assertTrue(any("baseline.yml: missing required pipeline entrypoint" in err for err in errors))
            self.assertIn(
                check_pipeline_wiring._failure_class("unbound"),
                failure_classes,
            )

    def test_evaluate_pipeline_wiring_reports_forbidden_legacy_surface(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-legacy-") as tmp:
            root = Path(tmp)
            _minimal_repo(root)
            _write(
                root / ".github/workflows/baseline.yml",
                """
name: baseline
jobs:
  baseline:
    steps:
      - name: run
        run: mise run ci-required-attested
""".strip()
                + "\n",
            )
            errors, failure_classes = check_pipeline_wiring.evaluate_pipeline_wiring(root)
            self.assertTrue(any("forbidden legacy required gate task call" in err for err in errors))
            self.assertIn(
                check_pipeline_wiring._failure_class("parityDrift"),
                failure_classes,
            )

    def test_evaluate_pipeline_wiring_reports_missing_governance_hook(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-governance-") as tmp:
            root = Path(tmp)
            _minimal_repo(root)
            _write(
                root / "tools/ci/pipeline_required.py",
                "from kcir_mapping_gate import evaluate_required_mapping\n",
            )
            errors, failure_classes = check_pipeline_wiring.evaluate_pipeline_wiring(root)
            self.assertTrue(any("pipeline_required.py: missing governance gate hook" in err for err in errors))
            self.assertIn(
                check_pipeline_wiring._failure_class("governanceGateMissing"),
                failure_classes,
            )

    def test_evaluate_pipeline_wiring_reports_missing_kcir_mapping_hook(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-kcir-") as tmp:
            root = Path(tmp)
            _minimal_repo(root)
            _write(
                root / "tools/ci/pipeline_instruction.py",
                "from governance_gate import governance_failure_classes\n",
            )
            errors, failure_classes = check_pipeline_wiring.evaluate_pipeline_wiring(root)
            self.assertTrue(any("pipeline_instruction.py: missing kcir mapping gate hook" in err for err in errors))
            self.assertIn(
                check_pipeline_wiring._failure_class("kcirMappingGateMissing"),
                failure_classes,
            )


if __name__ == "__main__":
    unittest.main()
