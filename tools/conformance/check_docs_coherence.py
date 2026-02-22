#!/usr/bin/env python3
"""Validate docs coherence against executable capability and gate surfaces."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, List, Sequence, Tuple


BACKTICK_CAP_RE = re.compile(r"`(capabilities\.[a-z0-9_]+)`")
BACKTICK_TASK_RE = re.compile(r"`([a-z][a-z0-9-]*)`")
CAPABILITY_REGISTRY_KIND = "premath.capability_registry.v1"


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
EXPECTED_DOCTRINE_CHECK_COMMANDS: Tuple[str, ...] = (
    "python3 tools/conformance/check_doctrine_site.py",
    "python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf",
)
UNIFICATION_EVIDENCE_MARKERS: Tuple[str, ...] = (
    "### 10.2 Universal factoring rule",
    "there MUST be one deterministic natural transformation:",
    "`eta_F : F => Ev`",
    "### 10.5 Fail-closed factorization boundary",
    "`unification.evidence_factorization.missing`",
    "`unification.evidence_factorization.ambiguous`",
    "`unification.evidence_factorization.unbound`",
)
SPEC_INDEX_UNIFIED_FACTORIZATION_RE = re.compile(
    r"Unified evidence factoring MUST route control-plane artifact families through\s+"
    r"one attested surface"
)
SPAN_SQUARE_COMPOSITION_MARKERS: Tuple[str, ...] = (
    "## 4. Composition Law Surface (Bicategory Profile)",
    "`compositionLaws`",
    "`span_identity`",
    "`square_interchange`",
    "digest = \"sqlw1_\" + SHA256(JCS(LawCore))",
)
PREMATH_COHERENCE_SPAN_COMPOSITION_RE = re.compile(
    r"accepted coverage includes span identity/associativity and square\s+"
    r"identity/associativity \(horizontal \+ vertical\), horizontal/vertical\s+"
    r"compatibility, and interchange",
    re.IGNORECASE,
)
ADJOINTS_CWF_SIGPI_BRIDGE_MARKERS: Tuple[str, ...] = (
    "## 11. CwF <-> sig\\Pi Bridge Contract (Strict vs Semantic)",
    "`bridge.reindex`",
    "`bridge.comprehension`",
    "`bridge.adjoint_reflection`",
    "bridge rules MUST NOT add new coherence",
)
PREMATH_COHERENCE_CWF_SIGPI_BRIDGE_RE = re.compile(
    r"bridge routing MUST NOT introduce new coherence obligation IDs",
    re.IGNORECASE,
)
SPEC_INDEX_CWF_SIGPI_BRIDGE_RE = re.compile(
    r"CwF<->sig\\Pi bridge mapping is normative in\s+"
    r"`profile/ADJOINTS-AND-SITES` §11",
    re.IGNORECASE,
)
UNIFICATION_OBSTRUCTION_MARKERS: Tuple[str, ...] = (
    "## 11. Cross-layer Obstruction Algebra (v0)",
    "`semantic(tag)`",
    "`structural(tag)`",
    "`lifecycle(tag)`",
    "`commutation(tag)`",
    "`project_obstruction(sourceFailureClass) -> constructor`",
    "`canonical_obstruction_class(constructor) -> canonicalFailureClass`",
    "commutation(span_square_commutation)",
    "`obs.<family>.<tag>`",
)
CAPABILITY_VECTORS_OBSTRUCTION_RE = re.compile(
    r"cross-layer obstruction rows roundtrip deterministically",
    re.IGNORECASE,
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


def parse_capability_registry(contract_path: Path) -> List[str]:
    payload = json.loads(contract_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{contract_path}: root must be an object")
    if payload.get("schema") != 1:
        raise ValueError(f"{contract_path}: schema must be 1")
    if payload.get("registryKind") != CAPABILITY_REGISTRY_KIND:
        raise ValueError(f"{contract_path}: registryKind mismatch")
    capabilities = payload.get("executableCapabilities")
    if not isinstance(capabilities, list) or not capabilities:
        raise ValueError(f"{contract_path}: executableCapabilities must be a non-empty list")
    parsed: List[str] = []
    for idx, item in enumerate(capabilities):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{contract_path}: executableCapabilities[{idx}] must be a non-empty string")
        parsed.append(item.strip())
    if len(set(parsed)) != len(parsed):
        raise ValueError(f"{contract_path}: executableCapabilities must not contain duplicates")
    return parsed


def parse_control_plane_projection_checks(contract_path: Path) -> List[str]:
    payload = json.loads(contract_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{contract_path}: root must be an object")
    if payload.get("schema") != 1:
        raise ValueError(f"{contract_path}: schema must be 1")
    if payload.get("contractKind") != "premath.control_plane.contract.v1":
        raise ValueError(f"{contract_path}: contractKind mismatch")
    required = payload.get("requiredGateProjection")
    if not isinstance(required, dict):
        raise ValueError(f"{contract_path}: requiredGateProjection must be an object")
    check_order = required.get("checkOrder")
    if not isinstance(check_order, list) or not check_order:
        raise ValueError(f"{contract_path}: requiredGateProjection.checkOrder must be a non-empty list")
    parsed: List[str] = []
    for idx, item in enumerate(check_order):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(
                f"{contract_path}: requiredGateProjection.checkOrder[{idx}] must be a non-empty string"
            )
        parsed.append(item.strip())
    if len(set(parsed)) != len(parsed):
        raise ValueError(f"{contract_path}: requiredGateProjection.checkOrder must not contain duplicates")
    return parsed


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

    capability_registry = root / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
    control_plane_contract = root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
    mise_toml = root / ".mise.toml"
    readme = root / "README.md"
    conformance_readme = root / "tools" / "conformance" / "README.md"
    ci_closure = root / "docs" / "design" / "CI-CLOSURE.md"
    spec_index = root / "specs" / "premath" / "draft" / "SPEC-INDEX.md"
    unification_doctrine = root / "specs" / "premath" / "draft" / "UNIFICATION-DOCTRINE.md"
    span_square_checking = root / "specs" / "premath" / "draft" / "SPAN-SQUARE-CHECKING.md"
    pre_math_coherence = root / "specs" / "premath" / "draft" / "PREMATH-COHERENCE.md"
    capability_vectors = root / "specs" / "premath" / "draft" / "CAPABILITY-VECTORS.md"
    adjoints_profile = root / "specs" / "premath" / "profile" / "ADJOINTS-AND-SITES.md"
    roadmap = root / "specs" / "premath" / "raw" / "ROADMAP.md"
    fixtures_root = root / "tests" / "conformance" / "fixtures" / "capabilities"

    executable_capabilities = parse_capability_registry(capability_registry)
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
    unification_text = load_text(unification_doctrine)
    span_square_text = load_text(span_square_checking)
    coherence_text = load_text(pre_math_coherence)
    capability_vectors_text = load_text(capability_vectors)
    adjoints_text = load_text(adjoints_profile)
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
    if SPEC_INDEX_UNIFIED_FACTORIZATION_RE.search(spec_index_text) is None:
        errors.append(
            "SPEC-INDEX lane ownership note must require Unified Evidence factoring as MUST"
        )
    missing_unification_markers = find_missing_markers(
        unification_text, UNIFICATION_EVIDENCE_MARKERS
    )
    for marker in missing_unification_markers:
        errors.append(f"UNIFICATION-DOCTRINE missing Unified Evidence marker: {marker}")
    missing_span_square_markers = find_missing_markers(
        span_square_text, SPAN_SQUARE_COMPOSITION_MARKERS
    )
    for marker in missing_span_square_markers:
        errors.append(f"SPAN-SQUARE-CHECKING missing composition marker: {marker}")
    if PREMATH_COHERENCE_SPAN_COMPOSITION_RE.search(coherence_text) is None:
        errors.append(
            "PREMATH-COHERENCE §4.7 must require composition-law coverage "
            "(identity/associativity/h-v/interchange)"
        )
    missing_adjoints_bridge_markers = find_missing_markers(
        adjoints_text, ADJOINTS_CWF_SIGPI_BRIDGE_MARKERS
    )
    for marker in missing_adjoints_bridge_markers:
        errors.append(f"ADJOINTS-AND-SITES missing CwF/SigPi bridge marker: {marker}")
    if PREMATH_COHERENCE_CWF_SIGPI_BRIDGE_RE.search(coherence_text) is None:
        errors.append(
            "PREMATH-COHERENCE must keep CwF/SigPi bridge fail-closed and "
            "vocabulary-preserving"
        )
    if SPEC_INDEX_CWF_SIGPI_BRIDGE_RE.search(spec_index_text) is None:
        errors.append(
            "SPEC-INDEX lane ownership note must include CwF<->sig\\Pi bridge "
            "normative reference"
        )
    missing_unification_obstruction_markers = find_missing_markers(
        unification_text, UNIFICATION_OBSTRUCTION_MARKERS
    )
    for marker in missing_unification_obstruction_markers:
        errors.append(f"UNIFICATION-DOCTRINE missing obstruction marker: {marker}")
    if CAPABILITY_VECTORS_OBSTRUCTION_RE.search(capability_vectors_text) is None:
        errors.append(
            "CAPABILITY-VECTORS must include cross-layer obstruction roundtrip "
            "coverage for capabilities.ci_witnesses"
        )

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

    projection_checks = parse_control_plane_projection_checks(control_plane_contract)
    projection_check_set = set(projection_checks)

    ci_projection_section = extract_section_between(
        ci_closure_text,
        "Current deterministic projected check IDs include:",
        "## 5. Variants and capability projection",
    )
    ci_projection_checks = {token for token in BACKTICK_TASK_RE.findall(ci_projection_section)}
    if ci_projection_checks != projection_check_set:
        errors.append(
            "CI-CLOSURE projected check ID list mismatch with CONTROL-PLANE-CONTRACT checkOrder: "
            f"expected=[{sorted_csv(projection_checks)}], got=[{sorted_csv(ci_projection_checks)}]"
        )

    doctrine_check_commands = parse_mise_task_commands(mise_text, "doctrine-check")
    if doctrine_check_commands != list(EXPECTED_DOCTRINE_CHECK_COMMANDS):
        errors.append(
            "doctrine-check command surface mismatch: "
            f"expected={list(EXPECTED_DOCTRINE_CHECK_COMMANDS)!r}, got={doctrine_check_commands!r}"
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
        f"projectionChecks={len(projection_checks)}, doctrineChecks={len(doctrine_check_commands)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
