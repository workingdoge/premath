#!/usr/bin/env python3
"""Unit tests for contract-bound provider pipeline wiring checks."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_pipeline_wiring
from control_plane_contract import PROVIDER_PIPELINE_WRAPPERS


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _command(wrapper_id: str) -> str:
    row = PROVIDER_PIPELINE_WRAPPERS[wrapper_id]
    return check_pipeline_wiring._render_shell_command(row["workflowEntrypoint"])


def _wrapper_path(root: Path, wrapper_id: str) -> Path:
    row = PROVIDER_PIPELINE_WRAPPERS[wrapper_id]
    return root / row["wrapperPath"]


def _workflow_path(root: Path, wrapper_id: str) -> Path:
    row = PROVIDER_PIPELINE_WRAPPERS[wrapper_id]
    return root / row["workflowPath"]


def _write_valid_tree(root: Path) -> None:
    _write(
        _workflow_path(root, "requiredPipeline"),
        (
            "name: baseline\njobs:\n  test:\n    steps:\n"
            "      - name: Run required pipeline\n"
            f"        run: {_command('requiredPipeline')}\n"
        ),
    )
    _write(
        _workflow_path(root, "instructionPipeline"),
        (
            "name: instruction\njobs:\n  test:\n    steps:\n"
            "      - name: Run instruction pipeline\n"
            f"        run: {_command('instructionPipeline')}\n"
        ),
    )
    _write(
        _wrapper_path(root, "requiredPipeline"),
        "\n".join(
            [
                "from control_plane_contract import REQUIRED_DECISION_CANONICAL_ENTRYPOINT",
                "from governance_gate import governance_failure_classes",
                "from kcir_mapping_gate import evaluate_required_mapping",
                "cmd = REQUIRED_DECISION_CANONICAL_ENTRYPOINT",
                "governance_failure_classes(Path('.'))",
                "evaluate_required_mapping(Path('.'), strict=True)",
            ]
        ),
    )
    _write(
        _wrapper_path(root, "instructionPipeline"),
        "\n".join(
            [
                "from control_plane_contract import INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
                "from control_plane_contract import INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT",
                "from governance_gate import governance_failure_classes",
                "from kcir_mapping_gate import evaluate_instruction_mapping",
                "cmd_a = INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT",
                "cmd_b = INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
                "governance_failure_classes(Path('.'))",
                "evaluate_instruction_mapping(Path('.'), instruction_path=Path('x'), instruction_id='x', strict=True)",
            ]
        ),
    )


class PipelineWiringTests(unittest.TestCase):
    def test_valid_contract_bound_pipeline_wiring_accepts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-") as tmp:
            root = Path(tmp)
            _write_valid_tree(root)
            findings = check_pipeline_wiring.evaluate_pipeline_wiring(root)
        self.assertEqual(findings, [])

    def test_workflow_drift_reports_contract_failure_class(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-") as tmp:
            root = Path(tmp)
            _write_valid_tree(root)
            _write(
                _workflow_path(root, "requiredPipeline"),
                (
                    "name: baseline\njobs:\n  test:\n    steps:\n"
                    "      - name: Run required pipeline\n"
                    "        run: mise run ci-required-attested\n"
                ),
            )
            findings = check_pipeline_wiring.evaluate_pipeline_wiring(root)

        self.assertTrue(
            any(
                finding.failure_class == "provider_pipeline_workflow_drift"
                for finding in findings
            )
        )

    def test_missing_contract_bound_command_surface_reports_drift(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-") as tmp:
            root = Path(tmp)
            _write_valid_tree(root)
            _write(
                _wrapper_path(root, "instructionPipeline"),
                "\n".join(
                    [
                        "from control_plane_contract import INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
                        "from governance_gate import governance_failure_classes",
                        "from kcir_mapping_gate import evaluate_instruction_mapping",
                        "cmd_b = INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
                        "governance_failure_classes(Path('.'))",
                        "evaluate_instruction_mapping(Path('.'), instruction_path=Path('x'), instruction_id='x', strict=True)",
                    ]
                ),
            )
            findings = check_pipeline_wiring.evaluate_pipeline_wiring(root)

        self.assertTrue(
            any(
                finding.failure_class
                == "provider_pipeline_canonical_entrypoint_drift"
                for finding in findings
            )
        )

    def test_missing_gate_reports_drift(self) -> None:
        with tempfile.TemporaryDirectory(prefix="premath-pipeline-wiring-") as tmp:
            root = Path(tmp)
            _write_valid_tree(root)
            _write(
                _wrapper_path(root, "requiredPipeline"),
                "\n".join(
                    [
                        "from control_plane_contract import REQUIRED_DECISION_CANONICAL_ENTRYPOINT",
                        "from governance_gate import governance_failure_classes",
                        "cmd = REQUIRED_DECISION_CANONICAL_ENTRYPOINT",
                        "governance_failure_classes(Path('.'))",
                    ]
                ),
            )
            findings = check_pipeline_wiring.evaluate_pipeline_wiring(root)

        self.assertTrue(
            any(
                finding.failure_class == "provider_pipeline_gate_drift"
                for finding in findings
            )
        )


if __name__ == "__main__":
    unittest.main()
