#!/usr/bin/env python3
"""Validate provider-neutral CI workflow wrapper parity against contract bindings."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Set, Tuple

from control_plane_contract import (
    PIPELINE_WRAPPER_FAILURE_CLASSES,
    PIPELINE_WRAPPER_INSTRUCTION_ENTRYPOINT,
    PIPELINE_WRAPPER_INSTRUCTION_GOVERNANCE_HOOK,
    PIPELINE_WRAPPER_INSTRUCTION_KCIR_MAPPING_HOOK,
    PIPELINE_WRAPPER_REQUIRED_ENTRYPOINT,
    PIPELINE_WRAPPER_REQUIRED_GOVERNANCE_HOOK,
    PIPELINE_WRAPPER_REQUIRED_KCIR_MAPPING_HOOK,
)


DEFAULT_FAILURE_CLASSES: Dict[str, str] = {
    "unbound": "control_plane_pipeline_wrapper_unbound",
    "parityDrift": "control_plane_pipeline_wrapper_parity_drift",
    "governanceGateMissing": "control_plane_pipeline_governance_gate_missing",
    "kcirMappingGateMissing": "control_plane_pipeline_kcir_mapping_gate_missing",
}


FORBIDDEN_PATTERNS: Tuple[Tuple[str, re.Pattern[str]], ...] = (
    ("legacy required gate task call", re.compile(r"^\s*run:\s*mise run ci-required-attested\s*$", re.MULTILINE)),
    ("legacy required gate split call", re.compile(r"^\s*run:\s*mise run ci-required\s*$", re.MULTILINE)),
    ("legacy strict verify call", re.compile(r"^\s*run:\s*mise run ci-verify-required-strict\s*$", re.MULTILINE)),
    ("legacy decision call", re.compile(r"^\s*run:\s*mise run ci-decide-required\s*$", re.MULTILINE)),
    ("legacy decision verify call", re.compile(r"^\s*run:\s*mise run ci-verify-decision\s*$", re.MULTILINE)),
    ("legacy provider env export call", re.compile(r"^\s*run:\s*python3 tools/ci/providers/export_github_env.py", re.MULTILINE)),
    ("legacy instruction check call", re.compile(r"^\s*run:\s*INSTRUCTION=.*mise run ci-instruction-check\s*$", re.MULTILINE)),
    ("legacy run_instruction shell call", re.compile(r"tools/ci/run_instruction.sh")),
    ("inline summary script block", re.compile(r"python3 - <<'PY'")),
)


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
    value = PIPELINE_WRAPPER_FAILURE_CLASSES.get(key)
    if isinstance(value, str) and value.strip():
        return value.strip()
    return DEFAULT_FAILURE_CLASSES[key]


def _token_pattern(token: str) -> str:
    if token.startswith("$"):
        return rf'"?{re.escape(token)}"?'
    return re.escape(token)


def _entrypoint_pattern(tokens: Sequence[str]) -> re.Pattern[str]:
    parts = [_token_pattern(token) for token in tokens]
    return re.compile(r"^\s*run:\s*" + r"\s+".join(parts) + r"\s*$", re.MULTILINE)


def _render_entrypoint(tokens: Sequence[str]) -> str:
    rendered: List[str] = []
    for token in tokens:
        if token.startswith("$"):
            rendered.append(f'"{token}"')
        else:
            rendered.append(token)
    return " ".join(rendered)


def _check_required_entrypoint(
    *,
    text: str,
    label: str,
    required_pattern: re.Pattern[str],
    required_command: str,
) -> Tuple[List[str], Set[str]]:
    errors: List[str] = []
    failure_classes: Set[str] = set()
    count = len(required_pattern.findall(text))
    if count == 0:
        failure_classes.add(_failure_class("unbound"))
        errors.append(f"{label}: missing required pipeline entrypoint `{required_command}`")
    elif count > 1:
        failure_classes.add(_failure_class("parityDrift"))
        errors.append(
            f"{label}: expected exactly one `{required_command}`, found {count}"
        )
    return errors, failure_classes


def _check_forbidden(
    text: str,
    label: str,
    forbidden: Iterable[Tuple[str, re.Pattern[str]]],
) -> Tuple[List[str], Set[str]]:
    errors: List[str] = []
    failure_classes: Set[str] = set()
    for reason, pattern in forbidden:
        if pattern.search(text):
            failure_classes.add(_failure_class("parityDrift"))
            errors.append(f"{label}: forbidden {reason}")
    return errors, failure_classes


def _check_pipeline_hooks(
    script_path: Path,
    *,
    governance_hook: str,
    kcir_mapping_hook: str,
) -> Tuple[List[str], Set[str]]:
    errors: List[str] = []
    failure_classes: Set[str] = set()
    try:
        text = script_path.read_text(encoding="utf-8")
    except OSError as exc:
        failure_classes.add(_failure_class("unbound"))
        errors.append(f"{script_path.name}: unreadable ({exc})")
        return errors, failure_classes

    governance_missing = not governance_hook or governance_hook not in text
    if governance_missing:
        failure_classes.add(_failure_class("governanceGateMissing"))
        errors.append(
            f"{script_path.name}: missing governance gate hook `{governance_hook}`"
        )
    mapping_missing = not kcir_mapping_hook or kcir_mapping_hook not in text
    if mapping_missing:
        failure_classes.add(_failure_class("kcirMappingGateMissing"))
        errors.append(
            f"{script_path.name}: missing kcir mapping gate hook `{kcir_mapping_hook}`"
        )
    return errors, failure_classes


def evaluate_pipeline_wiring(root: Path) -> Tuple[List[str], List[str]]:
    errors: List[str] = []
    failure_classes: Set[str] = set()

    baseline = root / ".github/workflows/baseline.yml"
    instruction = root / ".github/workflows/instruction.yml"
    required_pipeline_script = root / "tools/ci/pipeline_required.py"
    instruction_pipeline_script = root / "tools/ci/pipeline_instruction.py"

    required_entrypoint = tuple(PIPELINE_WRAPPER_REQUIRED_ENTRYPOINT)
    instruction_entrypoint = tuple(PIPELINE_WRAPPER_INSTRUCTION_ENTRYPOINT)
    if not required_entrypoint:
        failure_classes.add(_failure_class("unbound"))
        errors.append("CONTROL-PLANE-CONTRACT missing pipelineWrapperSurface.requiredPipelineEntrypoint")
    if not instruction_entrypoint:
        failure_classes.add(_failure_class("unbound"))
        errors.append("CONTROL-PLANE-CONTRACT missing pipelineWrapperSurface.instructionPipelineEntrypoint")

    if not baseline.exists():
        failure_classes.add(_failure_class("unbound"))
        errors.append(f"missing workflow: {baseline}")
    if not instruction.exists():
        failure_classes.add(_failure_class("unbound"))
        errors.append(f"missing workflow: {instruction}")

    if baseline.exists() and required_entrypoint:
        baseline_text = baseline.read_text(encoding="utf-8")
        baseline_errors, baseline_failures = _check_required_entrypoint(
            text=baseline_text,
            label="baseline.yml",
            required_pattern=_entrypoint_pattern(required_entrypoint),
            required_command=_render_entrypoint(required_entrypoint),
        )
        errors.extend(baseline_errors)
        failure_classes.update(baseline_failures)
        forbidden_errors, forbidden_failures = _check_forbidden(
            baseline_text,
            "baseline.yml",
            FORBIDDEN_PATTERNS,
        )
        errors.extend(forbidden_errors)
        failure_classes.update(forbidden_failures)

    if instruction.exists() and instruction_entrypoint:
        instruction_text = instruction.read_text(encoding="utf-8")
        instruction_errors, instruction_failures = _check_required_entrypoint(
            text=instruction_text,
            label="instruction.yml",
            required_pattern=_entrypoint_pattern(instruction_entrypoint),
            required_command=_render_entrypoint(instruction_entrypoint),
        )
        errors.extend(instruction_errors)
        failure_classes.update(instruction_failures)
        forbidden_errors, forbidden_failures = _check_forbidden(
            instruction_text,
            "instruction.yml",
            FORBIDDEN_PATTERNS,
        )
        errors.extend(forbidden_errors)
        failure_classes.update(forbidden_failures)

    required_hook_errors, required_hook_failures = _check_pipeline_hooks(
        required_pipeline_script,
        governance_hook=PIPELINE_WRAPPER_REQUIRED_GOVERNANCE_HOOK,
        kcir_mapping_hook=PIPELINE_WRAPPER_REQUIRED_KCIR_MAPPING_HOOK,
    )
    errors.extend(required_hook_errors)
    failure_classes.update(required_hook_failures)

    instruction_hook_errors, instruction_hook_failures = _check_pipeline_hooks(
        instruction_pipeline_script,
        governance_hook=PIPELINE_WRAPPER_INSTRUCTION_GOVERNANCE_HOOK,
        kcir_mapping_hook=PIPELINE_WRAPPER_INSTRUCTION_KCIR_MAPPING_HOOK,
    )
    errors.extend(instruction_hook_errors)
    failure_classes.update(instruction_hook_failures)

    return errors, sorted(failure_classes)


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()

    errors, failure_classes = evaluate_pipeline_wiring(root)
    if errors:
        print(
            "[pipeline-wiring] FAIL "
            f"(failureClasses={failure_classes})"
        )
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[pipeline-wiring] OK "
        f"(baseline={_render_entrypoint(PIPELINE_WRAPPER_REQUIRED_ENTRYPOINT)}, "
        f"instruction={_render_entrypoint(PIPELINE_WRAPPER_INSTRUCTION_ENTRYPOINT)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
