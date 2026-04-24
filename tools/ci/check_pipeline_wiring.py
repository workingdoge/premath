#!/usr/bin/env python3
"""Validate provider-neutral CI workflow and wrapper pipeline entrypoints."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Tuple

from control_plane_contract import (
    PROVIDER_PIPELINE_FAILURE_CLASSES,
    PROVIDER_PIPELINE_WRAPPERS,
)


FORBIDDEN_PATTERNS: Tuple[Tuple[str, re.Pattern[str]], ...] = (
    (
        "legacy required gate task call",
        re.compile(r"^\s*run:\s*mise run ci-required-attested\s*$", re.MULTILINE),
    ),
    (
        "legacy required gate split call",
        re.compile(r"^\s*run:\s*mise run ci-required\s*$", re.MULTILINE),
    ),
    (
        "legacy strict verify call",
        re.compile(r"^\s*run:\s*mise run ci-verify-required-strict\s*$", re.MULTILINE),
    ),
    (
        "legacy decision call",
        re.compile(r"^\s*run:\s*mise run ci-decide-required\s*$", re.MULTILINE),
    ),
    (
        "legacy decision verify call",
        re.compile(r"^\s*run:\s*mise run ci-verify-decision\s*$", re.MULTILINE),
    ),
    (
        "legacy provider env export call",
        re.compile(r"^\s*run:\s*python3 tools/ci/providers/export_github_env.py", re.MULTILINE),
    ),
    (
        "legacy instruction check call",
        re.compile(r"^\s*run:\s*INSTRUCTION=.*mise run ci-instruction-check\s*$", re.MULTILINE),
    ),
    (
        "legacy run_instruction shell call",
        re.compile(r"tools/ci/run_instruction.sh"),
    ),
    (
        "inline summary script block",
        re.compile(r"python3 - <<'PY'"),
    ),
)

COMMAND_SURFACE_MARKERS = {
    "requiredDecision": "REQUIRED_DECISION_CANONICAL_ENTRYPOINT",
    "instructionEnvelopeCheck": "INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT",
    "instructionDecision": "INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
}

GATE_MARKERS = {
    "governance": {
        "requiredPipeline": "governance_failure_classes",
        "instructionPipeline": "governance_failure_classes",
    },
    "kcirMapping": {
        "requiredPipeline": "evaluate_required_mapping",
        "instructionPipeline": "evaluate_instruction_mapping",
    },
}


@dataclass(frozen=True)
class PipelineFinding:
    failure_class: str
    message: str


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check CI workflow pipeline wiring.")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    return parser.parse_args()


def _failure_class(key: str) -> str:
    value = PROVIDER_PIPELINE_FAILURE_CLASSES.get(key, "")
    return value or f"provider_pipeline_{key}"


def _render_shell_command(tokens: Iterable[str]) -> str:
    rendered: list[str] = []
    for token in tokens:
        if token.startswith("$"):
            rendered.append(f'"{token}"')
        else:
            rendered.append(token)
    return " ".join(rendered)


def _run_pattern(command: str) -> re.Pattern[str]:
    return re.compile(rf"^\s*run:\s*{re.escape(command)}\s*$", re.MULTILINE)


def _read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def _check_forbidden(
    text: str,
    label: str,
    forbidden: Iterable[Tuple[str, re.Pattern[str]]],
) -> list[PipelineFinding]:
    findings: list[PipelineFinding] = []
    for reason, pattern in forbidden:
        if pattern.search(text):
            findings.append(
                PipelineFinding(
                    _failure_class("workflowDrift"),
                    f"{label}: forbidden {reason}",
                )
            )
    return findings


def _check_workflow_entrypoint(
    text: str,
    *,
    label: str,
    command: str,
) -> list[PipelineFinding]:
    count = len(_run_pattern(command).findall(text))
    if count == 0:
        return [
            PipelineFinding(
                _failure_class("workflowDrift"),
                f"{label}: missing provider pipeline entrypoint `{command}`",
            )
        ]
    if count > 1:
        return [
            PipelineFinding(
                _failure_class("workflowDrift"),
                f"{label}: expected exactly one `{command}`, found {count}",
            )
        ]
    return []


def _check_wrapper_source(
    text: str,
    *,
    wrapper_id: str,
    label: str,
    bound_surfaces: Iterable[str],
    enforced_gates: Iterable[str],
) -> list[PipelineFinding]:
    findings: list[PipelineFinding] = []
    for surface_id in bound_surfaces:
        marker = COMMAND_SURFACE_MARKERS.get(surface_id)
        if marker is None or marker not in text:
            findings.append(
                PipelineFinding(
                    _failure_class("canonicalEntrypointDrift"),
                    f"{label}: missing contract-bound command surface `{surface_id}`",
                )
            )
    for gate_id in enforced_gates:
        marker = GATE_MARKERS.get(gate_id, {}).get(wrapper_id)
        if marker is None or marker not in text:
            findings.append(
                PipelineFinding(
                    _failure_class("gateDrift"),
                    f"{label}: missing enforced gate `{gate_id}`",
                )
            )
    return findings


def evaluate_pipeline_wiring(root: Path) -> list[PipelineFinding]:
    findings: list[PipelineFinding] = []
    for wrapper_id in sorted(
        key for key in PROVIDER_PIPELINE_WRAPPERS if key != "failureClasses"
    ):
        row = PROVIDER_PIPELINE_WRAPPERS[wrapper_id]
        if not isinstance(row, dict):
            findings.append(
                PipelineFinding(
                    _failure_class("workflowDrift"),
                    f"{wrapper_id}: contract row must be an object",
                )
            )
            continue
        workflow_rel = str(row.get("workflowPath", ""))
        wrapper_rel = str(row.get("wrapperPath", ""))
        command_tokens = row.get("workflowEntrypoint", [])
        bound_surfaces = row.get("boundCommandSurfaces", [])
        enforced_gates = row.get("enforcedGates", [])

        workflow_path = root / workflow_rel
        wrapper_path = root / wrapper_rel
        workflow_text = _read_text(workflow_path)
        wrapper_text = _read_text(wrapper_path)
        command = (
            _render_shell_command(command_tokens)
            if isinstance(command_tokens, list)
            else ""
        )

        if workflow_text is None:
            findings.append(
                PipelineFinding(
                    _failure_class("workflowDrift"),
                    f"{workflow_rel}: workflow file missing",
                )
            )
        elif not command:
            findings.append(
                PipelineFinding(
                    _failure_class("workflowDrift"),
                    f"{workflow_rel}: workflow entrypoint is unbound",
                )
            )
        else:
            findings.extend(
                _check_workflow_entrypoint(
                    workflow_text,
                    label=workflow_rel,
                    command=command,
                )
            )
            findings.extend(
                _check_forbidden(workflow_text, workflow_rel, FORBIDDEN_PATTERNS)
            )

        if wrapper_text is None:
            findings.append(
                PipelineFinding(
                    _failure_class("canonicalEntrypointDrift"),
                    f"{wrapper_rel}: wrapper file missing",
                )
            )
        else:
            findings.extend(
                _check_wrapper_source(
                    wrapper_text,
                    wrapper_id=wrapper_id,
                    label=wrapper_rel,
                    bound_surfaces=bound_surfaces if isinstance(bound_surfaces, list) else (),
                    enforced_gates=enforced_gates if isinstance(enforced_gates, list) else (),
                )
            )

    return findings


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()
    findings = evaluate_pipeline_wiring(root)

    if findings:
        print("[pipeline-wiring] FAIL")
        for finding in findings:
            print(f"  - {finding.failure_class}: {finding.message}")
        return 1

    commands = []
    for wrapper_id in sorted(
        key for key in PROVIDER_PIPELINE_WRAPPERS if key != "failureClasses"
    ):
        row = PROVIDER_PIPELINE_WRAPPERS[wrapper_id]
        if isinstance(row, dict):
            command = _render_shell_command(row.get("workflowEntrypoint", []))
            commands.append(f"{wrapper_id}={command}")
    print("[pipeline-wiring] OK (" + ", ".join(commands) + ")")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
