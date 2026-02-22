#!/usr/bin/env python3
"""Validate docs coherence against executable capability and gate surfaces."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Sequence, Tuple


CAP_ASSIGN_RE = re.compile(r"^(CAPABILITY_[A-Z0-9_]+)\s*=\s*\"(capabilities\.[a-z0-9_]+)\"$", re.MULTILINE)
CHECK_ASSIGN_RE = re.compile(r"^(CHECK_[A-Z0-9_]+)\s*=\s*\"([a-z0-9-]+)\"$", re.MULTILINE)
SYMBOL_RE = re.compile(r"\b([A-Z][A-Z0-9_]+)\b")
BACKTICK_CAP_RE = re.compile(r"`(capabilities\.[a-z0-9_]+)`")
BACKTICK_TASK_RE = re.compile(r"`([a-z][a-z0-9-]*)`")


EXPECTED_CONDITIONAL_CAPABILITY_DOCS: Tuple[Tuple[str, str], ...] = (
    ("draft/LLM-INSTRUCTION-DOCTRINE", "capabilities.instruction_typing"),
    ("draft/LLM-PROPOSAL-CHECKING", "capabilities.instruction_typing"),
    ("raw/SQUEAK-SITE", "capabilities.squeak_site"),
    ("raw/PREMATH-CI", "capabilities.ci_witnesses"),
)
SPEC_INDEX_RAW_LIFECYCLE_MARKERS: Tuple[str, ...] = (
    "Raw capability-spec lifecycle policy:",
    "Promotion from raw to draft for capability-scoped specs requires:",
    "`raw/SQUEAK-SITE` — tracked by issue `bd-44`",
    "`raw/TUSK-CORE` — tracked by issue `bd-45`",
)
ROADMAP_AUTHORITY_MARKERS: Tuple[str, ...] = (
    "authoritative source of active work",
    "If this file conflicts with those surfaces",
    "`.premath/issues.jsonl`",
    "`specs/process/decision-log.md`",
)


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description="Validate docs coherence against executable capability + gate surfaces."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=root,
        help=f"Repository root (default: {root})",
    )
    return parser.parse_args()


def load_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def extract_section_between(text: str, start_marker: str, end_marker: str) -> str:
    start = text.find(start_marker)
    if start < 0:
        raise ValueError(f"missing start marker: {start_marker!r}")
    start += len(start_marker)
    end = text.find(end_marker, start)
    if end < 0:
        raise ValueError(f"missing end marker after {start_marker!r}: {end_marker!r}")
    return text[start:end]


def extract_heading_section(text: str, heading_prefix: str) -> str:
    heading_re = re.compile(rf"^### {re.escape(heading_prefix)}.*?$", re.MULTILINE)
    match = heading_re.search(text)
    if match is None:
        raise ValueError(f"missing heading: {heading_prefix!r}")
    section_start = match.end()
    tail = text[section_start:]
    next_heading = re.search(r"^### ", tail, re.MULTILINE)
    if next_heading is None:
        return tail
    return tail[: next_heading.start()]


def parse_symbol_tuple_values(
    text: str,
    assign_pattern: re.Pattern[str],
    tuple_name: str,
) -> List[str]:
    symbol_map: Dict[str, str] = {symbol: value for symbol, value in assign_pattern.findall(text)}
    tuple_re = re.compile(rf"{re.escape(tuple_name)}[^\n]*=\s*\((.*?)\)", re.DOTALL)
    tuple_match = tuple_re.search(text)
    if tuple_match is None:
        raise ValueError(f"missing tuple definition: {tuple_name}")
    tuple_body = tuple_match.group(1)
    ordered_symbols: List[str] = []
    for symbol in SYMBOL_RE.findall(tuple_body):
        if symbol in symbol_map and symbol not in ordered_symbols:
            ordered_symbols.append(symbol)
    if not ordered_symbols:
        raise ValueError(f"tuple {tuple_name} does not reference known symbols")
    values: List[str] = []
    for symbol in ordered_symbols:
        if symbol not in symbol_map:
            raise ValueError(f"tuple {tuple_name} references unknown symbol: {symbol}")
        values.append(symbol_map[symbol])
    return values


def parse_mise_task_commands(text: str, task_name: str) -> List[str]:
    section_re = re.compile(rf"^\[tasks\.{re.escape(task_name)}\]\n(.*?)(?=^\[tasks\.|\Z)", re.MULTILINE | re.DOTALL)
    section_match = section_re.search(text)
    if section_match is None:
        raise ValueError(f"missing [tasks.{task_name}] section")
    section = section_match.group(1)
    run_match = re.search(r"run\s*=\s*\[(.*?)\]", section, re.DOTALL)
    if run_match is None:
        raise ValueError(f"[tasks.{task_name}] missing run list")
    run_body = run_match.group(1)
    commands = re.findall(r"\"([^\"]+)\"", run_body)
    if not commands:
        raise ValueError(f"[tasks.{task_name}] run list has no commands")
    return commands


def parse_baseline_task_ids_from_commands(commands: Sequence[str]) -> List[str]:
    out: List[str] = []
    cmd_re = re.compile(r"^mise run ([a-z][a-z0-9-]*)$")
    for command in commands:
        match = cmd_re.match(command.strip())
        if match is None:
            raise ValueError(f"[tasks.baseline] unsupported command shape: {command!r}")
        out.append(match.group(1))
    return out


def parse_manifest_capabilities(fixtures_root: Path) -> List[str]:
    manifests = sorted(fixtures_root.glob("capabilities.*/manifest.json"))
    if not manifests:
        raise ValueError(f"no capability manifests found under {fixtures_root}")
    capability_ids: List[str] = []
    for manifest in manifests:
        payload = json.loads(manifest.read_text(encoding="utf-8"))
        capability_id = payload.get("capabilityId")
        if not isinstance(capability_id, str) or not capability_id:
            raise ValueError(f"{manifest}: capabilityId must be non-empty string")
        capability_ids.append(capability_id)
    return capability_ids


def parse_spec_index_capability_doc_map(section_54: str) -> Dict[str, str]:
    pattern = re.compile(r"- `([^`]+)`\s+\(for `([^`]+)`\)")
    out: Dict[str, str] = {}
    for doc_ref, capability_id in pattern.findall(section_54):
        out[doc_ref] = capability_id
    return out


def verify_conditional_normative_entry(section_55: str, doc_ref: str, capability_id: str) -> bool:
    pattern = re.compile(
        rf"`{re.escape(doc_ref)}`[\s\S]*?normative\s+only\s+when\s+`{re.escape(capability_id)}`\s+is\s+claimed",
        re.IGNORECASE,
    )
    return pattern.search(section_55) is not None


def sorted_csv(values: Sequence[str]) -> str:
    return ", ".join(sorted(values))


def find_missing_markers(text: str, markers: Sequence[str]) -> List[str]:
    return [marker for marker in markers if marker not in text]


def main() -> int:
    args = parse_args()
    root = args.repo_root.resolve()
    errors: List[str] = []

    run_capability_vectors = root / "tools" / "conformance" / "run_capability_vectors.py"
    change_projection = root / "tools" / "ci" / "change_projection.py"
    mise_toml = root / ".mise.toml"
    readme = root / "README.md"
    conformance_readme = root / "tools" / "conformance" / "README.md"
    ci_closure = root / "docs" / "design" / "CI-CLOSURE.md"
    spec_index = root / "specs" / "premath" / "draft" / "SPEC-INDEX.md"
    roadmap = root / "specs" / "premath" / "raw" / "ROADMAP.md"
    fixtures_root = root / "tests" / "conformance" / "fixtures" / "capabilities"

    capability_text = load_text(run_capability_vectors)
    executable_capabilities = parse_symbol_tuple_values(
        capability_text,
        CAP_ASSIGN_RE,
        "DEFAULT_EXECUTABLE_CAPABILITIES",
    )
    executable_capability_set = set(executable_capabilities)

    manifest_capabilities = parse_manifest_capabilities(fixtures_root)
    manifest_capability_set = set(manifest_capabilities)
    if manifest_capability_set != executable_capability_set:
        missing = sorted(executable_capability_set - manifest_capability_set)
        extra = sorted(manifest_capability_set - executable_capability_set)
        if missing:
            errors.append(f"capability manifests missing executable capabilities: {missing}")
        if extra:
            errors.append(f"capability manifests include non-executable capabilities: {extra}")

    readme_caps = set(BACKTICK_CAP_RE.findall(load_text(readme)))
    conformance_readme_caps = set(BACKTICK_CAP_RE.findall(load_text(conformance_readme)))

    spec_index_text = load_text(spec_index)
    section_54 = extract_heading_section(spec_index_text, "5.4")
    section_55 = extract_heading_section(spec_index_text, "5.5")
    spec_index_caps = set(BACKTICK_CAP_RE.findall(section_54))
    spec_index_doc_map = parse_spec_index_capability_doc_map(section_54)

    if readme_caps != executable_capability_set:
        errors.append(
            "README capability list mismatch with executable capabilities: "
            f"expected=[{sorted_csv(executable_capabilities)}], got=[{sorted_csv(readme_caps)}]"
        )
    if conformance_readme_caps != executable_capability_set:
        errors.append(
            "tools/conformance/README capability list mismatch with executable capabilities: "
            f"expected=[{sorted_csv(executable_capabilities)}], got=[{sorted_csv(conformance_readme_caps)}]"
        )
    if spec_index_caps != executable_capability_set:
        errors.append(
            "SPEC-INDEX §5.4 capability list mismatch with executable capabilities: "
            f"expected=[{sorted_csv(executable_capabilities)}], got=[{sorted_csv(spec_index_caps)}]"
        )

    informative_clause = "unless they are\nexplicitly claimed under §5.4 or §5.6"
    if informative_clause not in section_55:
        errors.append(
            "SPEC-INDEX §5.5 must explicitly state informative/default status unless claimed under §5.4 or §5.6"
        )

    for doc_ref, capability_id in EXPECTED_CONDITIONAL_CAPABILITY_DOCS:
        mapped = spec_index_doc_map.get(doc_ref)
        if mapped != capability_id:
            errors.append(
                f"SPEC-INDEX §5.4 capability mapping mismatch for {doc_ref}: expected {capability_id!r}, got {mapped!r}"
            )
        if not verify_conditional_normative_entry(section_55, doc_ref, capability_id):
            errors.append(
                f"SPEC-INDEX §5.5 missing conditional normative clause for {doc_ref} ({capability_id})"
            )

    missing_raw_lifecycle_markers = find_missing_markers(section_55, SPEC_INDEX_RAW_LIFECYCLE_MARKERS)
    for marker in missing_raw_lifecycle_markers:
        errors.append(f"SPEC-INDEX §5.5 raw lifecycle policy missing marker: {marker}")

    mise_text = load_text(mise_toml)
    baseline_commands = parse_mise_task_commands(mise_text, "baseline")
    baseline_task_ids = parse_baseline_task_ids_from_commands(baseline_commands)
    baseline_task_set = set(baseline_task_ids)

    ci_closure_text = load_text(ci_closure)
    ci_baseline_section = extract_section_between(
        ci_closure_text,
        "Current full baseline gate (`mise run baseline`) includes:",
        "Local command:",
    )
    ci_baseline_tasks = {token for token in BACKTICK_TASK_RE.findall(ci_baseline_section)}
    if ci_baseline_tasks != baseline_task_set:
        errors.append(
            "CI-CLOSURE baseline task list mismatch with .mise baseline: "
            f"expected=[{sorted_csv(baseline_task_ids)}], got=[{sorted_csv(ci_baseline_tasks)}]"
        )

    projection_text = load_text(change_projection)
    projection_checks = parse_symbol_tuple_values(projection_text, CHECK_ASSIGN_RE, "CHECK_ORDER")
    projection_check_set = set(projection_checks)

    ci_projection_section = extract_section_between(
        ci_closure_text,
        "Current deterministic projected check IDs include:",
        "## 5. Variants and capability projection",
    )
    ci_projection_checks = {token for token in BACKTICK_TASK_RE.findall(ci_projection_section)}
    if ci_projection_checks != projection_check_set:
        errors.append(
            "CI-CLOSURE projected check ID list mismatch with tools/ci/change_projection.py CHECK_ORDER: "
            f"expected=[{sorted_csv(projection_checks)}], got=[{sorted_csv(ci_projection_checks)}]"
        )

    roadmap_text = load_text(roadmap)
    missing_roadmap_markers = find_missing_markers(roadmap_text, ROADMAP_AUTHORITY_MARKERS)
    for marker in missing_roadmap_markers:
        errors.append(f"ROADMAP authority contract missing marker: {marker}")

    if errors:
        print(f"[docs-coherence-check] FAIL (errors={len(errors)})")
        for error in errors:
            print(f"  - {error}")
        return 1

    print(
        "[docs-coherence-check] OK "
        f"(capabilities={len(executable_capabilities)}, baselineTasks={len(baseline_task_ids)}, "
        f"projectionChecks={len(projection_checks)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
