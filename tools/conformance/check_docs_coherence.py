#!/usr/bin/env python3
"""Validate docs coherence against executable capability and gate surfaces."""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Sequence, Tuple

import doctrine_site_contract
import generate_doctrine_site_inventory


BACKTICK_CAP_RE = re.compile(r"`(capabilities\.[a-z0-9_]+)`")
BACKTICK_TASK_RE = re.compile(r"`([a-z][a-z0-9-]*)`")
CAPABILITY_REGISTRY_KIND = "premath.capability_registry.v1"
DOCTRINE_SITE_GENERATION_DIGEST_KIND = "premath.doctrine_site_generation_digest.v1"
PROFILE_CLAIM_RE = re.compile(r"`(profile\.[a-z0-9_.]+)`")
README_WORKSPACE_CRATE_RE = re.compile(r"`(crates/premath-[a-z0-9_-]+)`")
TRACKED_ISSUE_RE = re.compile(r"tracked by issue\s+`?(bd-\d+)`?", re.IGNORECASE)
STEEL_HOST_ACTION_SECTION_START = "### 5.1 Exact command/tool mapping (host id -> CLI/MCP)"
STEEL_HOST_ACTION_SECTION_END = "## 6. Deterministic Effect Row Contract"
HOST_ACTION_FAILURE_CLASS_KEYS: Tuple[str, ...] = (
    "unregisteredHostId",
    "bindingMismatch",
    "duplicateBinding",
    "contractUnbound",
)
HOST_ACTION_HARNESS_SESSION_OPERATION_IDS: Dict[str, str] = {
    "harness.session.read": "op/harness.session_read",
    "harness.session.write": "op/harness.session_write",
    "harness.session.bootstrap": "op/harness.session_bootstrap",
}


SPEC_INDEX_RAW_LIFECYCLE_MARKERS: Tuple[str, ...] = (
    "Raw capability-spec lifecycle policy:",
    "Promotion from raw to draft for capability-scoped specs requires:",
    "`raw/SQUEAK-SITE` — retained raw per Decision 0040",
    "`raw/TUSK-CORE` — retained raw per Decision 0041",
)
ROADMAP_AUTHORITY_MARKERS: Tuple[str, ...] = (
    "authoritative source of active work",
    "If this file conflicts with those surfaces",
    "`.premath/issues.jsonl`",
    "`specs/process/decision-log.md`",
)
README_FOUNDATIONS_MARKERS: Tuple[str, ...] = (
    "`docs/foundations/` — explanatory foundations notes (non-normative)",
)
SPEC_INDEX_FOUNDATIONS_MARKERS: Tuple[str, ...] = (
    "`docs/foundations/` — explanatory notes (non-normative).",
)
FOUNDATIONS_README_MARKERS: Tuple[str, ...] = (
    "# Foundations Notes (Non-Normative)",
    "These notes are explanatory and non-normative.",
    "## Foundations to Spec Mapping",
    "| Foundations note | Normative anchor(s) |",
)
README_DOCTRINE_MARKERS: Tuple[str, ...] = (
    "doctrine-to-operation site coherence validation (including MCP",
    "mise run doctrine-check",
)
ARCHITECTURE_DOCTRINE_MARKERS: Tuple[str, ...] = (
    "`tools/conformance/check_runtime_orchestration.py`",
    "`tools/conformance/check_doctrine_mcp_parity.py`",
    "doctrine-operation parity + runtime-route parity",
    "`check_runtime_orchestration.py`,",
    "`check_doctrine_mcp_parity.py`),",
)
EXPECTED_DOCTRINE_CHECK_COMMANDS: Tuple[str, ...] = (
    "python3 tools/conformance/check_doctrine_site.py",
    "cargo run --package premath-cli -- runtime-orchestration-check --control-plane-contract specs/premath/draft/CONTROL-PLANE-CONTRACT.json --doctrine-op-registry specs/premath/draft/DOCTRINE-OP-REGISTRY.json --harness-runtime specs/premath/draft/HARNESS-RUNTIME.md --doctrine-site-input specs/premath/draft/DOCTRINE-SITE-INPUT.json --json",
    "python3 tools/conformance/check_doctrine_mcp_parity.py",
    "python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf",
)
CI_CLOSURE_DOCTRINE_MARKERS: Tuple[str, ...] = (
    "`doctrine-check` (site coherence + runtime orchestration route parity +",
    "doctrine-inf vectors)",
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
UNIFICATION_INTERNALIZATION_MARKERS: Tuple[str, ...] = (
    "### 10.6 Typed evidence-object internalization stages (v0)",
    "Stage 0 (projection-locked):",
    "Stage 1 (typed-core dual projection):",
    "Stage 2 (canonical typed authority with compatibility alias):",
    "Stage 3 (typed-first cleanup):",
    "Rollback requirements:",
    "rollback MUST NOT introduce a second authority artifact,",
)
UNIFICATION_STAGE1_PROFILE_MARKERS: Tuple[str, ...] = (
    "#### 10.6.1 Stage 1 typed-core profile (minimum)",
    "one profile kind identifier (for example `ev.stage1.core.v1`),",
    "one canonical typed-core identity function over canonicalized profile bytes",
    "#### 10.6.2 Stage 1 dual-projection parity contract",
    "`unification.evidence_stage1.parity.missing`",
    "`unification.evidence_stage1.parity.mismatch`",
    "`unification.evidence_stage1.parity.unbound`",
    "#### 10.6.3 Stage 1 deterministic rollback witness contract",
    "`unification.evidence_stage1.rollback.precondition`",
    "`unification.evidence_stage1.rollback.identity_drift`",
    "`unification.evidence_stage1.rollback.unbound`",
)
UNIFICATION_STAGE3_CLOSURE_MARKERS: Tuple[str, ...] = (
    "#### 10.6.5 Stage 3 typed-first closure mapping (normative)",
    "`evidenceStage2Authority.bidirEvidenceRoute`",
    "`routeKind=direct_checker_discharge`",
    "`obligationFieldRef=bidirCheckerObligations`",
    "`bidirEvidenceRoute.fallback.mode=profile_gated_sentinel`",
    "Compatibility alias lookup MAY exist only behind an explicit",
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
STAGE1_PARITY_CANONICAL_CLASSES: Tuple[str, str, str] = (
    "unification.evidence_stage1.parity.missing",
    "unification.evidence_stage1.parity.mismatch",
    "unification.evidence_stage1.parity.unbound",
)
STAGE1_ROLLBACK_CANONICAL_CLASSES: Tuple[str, str, str] = (
    "unification.evidence_stage1.rollback.precondition",
    "unification.evidence_stage1.rollback.identity_drift",
    "unification.evidence_stage1.rollback.unbound",
)
STAGE2_AUTHORITY_CANONICAL_CLASSES: Tuple[str, str, str] = (
    "unification.evidence_stage2.authority_alias_violation",
    "unification.evidence_stage2.alias_window_violation",
    "unification.evidence_stage2.unbound",
)
STAGE2_KERNEL_COMPLIANCE_CANONICAL_CLASSES: Tuple[str, str] = (
    "unification.evidence_stage2.kernel_compliance_missing",
    "unification.evidence_stage2.kernel_compliance_drift",
)
STAGE2_REQUIRED_KERNEL_OBLIGATIONS: Tuple[str, ...] = (
    "stability",
    "locality",
    "descent_exists",
    "descent_contractible",
    "adjoint_triple",
    "ext_gap",
    "ext_ambiguous",
)
README_WORKSPACE_CRATE_ALLOWLIST: Tuple[str, ...] = ()


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


@dataclass(frozen=True)
class CapabilityRegistryContract:
    executable_capabilities: List[str]
    profile_overlay_claims: List[str]
    capability_doc_map: Dict[str, str]


@dataclass(frozen=True)
class TrackedIssueReference:
    path: Path
    line_number: int
    issue_id: str
    issue_status: str | None


def parse_capability_registry(contract_path: Path) -> CapabilityRegistryContract:
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
    overlay_claims = payload.get("profileOverlayClaims", [])
    if not isinstance(overlay_claims, list):
        raise ValueError(f"{contract_path}: profileOverlayClaims must be a list")
    parsed_overlay_claims: List[str] = []
    for idx, item in enumerate(overlay_claims):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{contract_path}: profileOverlayClaims[{idx}] must be a non-empty string")
        parsed_overlay_claims.append(item.strip())
    if len(set(parsed_overlay_claims)) != len(parsed_overlay_claims):
        raise ValueError(f"{contract_path}: profileOverlayClaims must not contain duplicates")
    raw_doc_bindings = payload.get("capabilityDocBindings", [])
    if not isinstance(raw_doc_bindings, list):
        raise ValueError(f"{contract_path}: capabilityDocBindings must be a list")
    capability_doc_map: Dict[str, str] = {}
    for idx, entry in enumerate(raw_doc_bindings):
        if not isinstance(entry, dict):
            raise ValueError(f"{contract_path}: capabilityDocBindings[{idx}] must be an object")
        doc_ref = entry.get("docRef")
        capability_id = entry.get("capabilityId")
        if not isinstance(doc_ref, str) or not doc_ref.strip():
            raise ValueError(
                f"{contract_path}: capabilityDocBindings[{idx}].docRef must be a non-empty string"
            )
        if not isinstance(capability_id, str) or not capability_id.strip():
            raise ValueError(
                f"{contract_path}: capabilityDocBindings[{idx}].capabilityId must be a non-empty string"
            )
        doc_key = doc_ref.strip()
        cap_value = capability_id.strip()
        if cap_value not in parsed:
            raise ValueError(
                f"{contract_path}: capabilityDocBindings[{idx}].capabilityId {cap_value!r} "
                "must be declared in executableCapabilities"
            )
        previous = capability_doc_map.get(doc_key)
        if previous is not None and previous != cap_value:
            raise ValueError(
                f"{contract_path}: capabilityDocBindings docRef {doc_key!r} maps to multiple capabilities"
            )
        capability_doc_map[doc_key] = cap_value
    if not capability_doc_map:
        raise ValueError(f"{contract_path}: capabilityDocBindings must contain at least one mapping")
    return CapabilityRegistryContract(
        executable_capabilities=parsed,
        profile_overlay_claims=parsed_overlay_claims,
        capability_doc_map=capability_doc_map,
    )


def parse_conformance_overlay_claims(conformance_path: Path) -> List[str]:
    text = load_text(conformance_path)
    section_24 = extract_heading_section(text, "2.4")
    claims = [claim.strip() for claim in PROFILE_CLAIM_RE.findall(section_24)]
    return sorted(set(claims))


def parse_workspace_members(cargo_toml_path: Path, repo_root: Path) -> List[str]:
    text = cargo_toml_path.read_text(encoding="utf-8")
    workspace_match = re.search(r"^\[workspace\]\n(.*?)(?=^\[|\Z)", text, re.MULTILINE | re.DOTALL)
    if workspace_match is None:
        raise ValueError(f"{cargo_toml_path}: missing [workspace] section")
    workspace_section = workspace_match.group(1)
    members_match = re.search(r"members\s*=\s*\[(.*?)\]", workspace_section, re.DOTALL)
    if members_match is None:
        raise ValueError(f"{cargo_toml_path}: workspace.members list missing")
    members = re.findall(r"\"([^\"]+)\"", members_match.group(1))
    if not members:
        raise ValueError(f"{cargo_toml_path}: workspace.members must be a non-empty list")
    resolved_members: List[str] = []
    for idx, member in enumerate(members):
        member_value = member.strip()
        if not member_value:
            raise ValueError(f"{cargo_toml_path}: workspace.members[{idx}] must be a non-empty string")
        if any(ch in member_value for ch in "*?[]"):
            matches = sorted(
                path for path in repo_root.glob(member_value) if path.is_dir() and (path / "Cargo.toml").is_file()
            )
            if not matches:
                raise ValueError(
                    f"{cargo_toml_path}: workspace.members[{idx}] glob {member_value!r} matched no crates"
                )
            resolved_members.extend(path.relative_to(repo_root).as_posix() for path in matches)
            continue
        resolved_members.append(Path(member_value).as_posix())
    deduped = sorted(set(resolved_members))
    if not deduped:
        raise ValueError(f"{cargo_toml_path}: workspace.members resolved to empty set")
    return deduped


def parse_readme_workspace_crates(readme_text: str) -> List[str]:
    section = extract_section_between(readme_text, "## Workspace layering", "## Baseline gate")
    crates = sorted(set(README_WORKSPACE_CRATE_RE.findall(section)))
    if not crates:
        raise ValueError("README.md: workspace layering section must enumerate crates/premath-* entries")
    return crates


def parse_issue_statuses(issues_path: Path) -> Dict[str, str]:
    statuses: Dict[str, str] = {}
    for idx, line in enumerate(issues_path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{issues_path}:{idx}: invalid JSONL row: {exc}") from exc
        if not isinstance(payload, dict):
            raise ValueError(f"{issues_path}:{idx}: issue row must be an object")
        issue_id = payload.get("id")
        status = payload.get("status")
        if not isinstance(issue_id, str) or not issue_id.strip():
            raise ValueError(f"{issues_path}:{idx}: issue id must be a non-empty string")
        if not isinstance(status, str) or not status.strip():
            raise ValueError(f"{issues_path}:{idx}: issue status must be a non-empty string")
        statuses[issue_id.strip()] = status.strip()
    if not statuses:
        raise ValueError(f"{issues_path}: no issues found")
    return statuses


def find_stale_tracked_issue_references(
    roots: Sequence[Path],
    issue_statuses: Dict[str, str],
    excluded_paths: Sequence[Path] = (),
) -> List[TrackedIssueReference]:
    excluded = {path.resolve() for path in excluded_paths}
    refs: List[TrackedIssueReference] = []
    for root in roots:
        for path in sorted(root.rglob("*.md")):
            resolved = path.resolve()
            if resolved in excluded:
                continue
            for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
                match = TRACKED_ISSUE_RE.search(line)
                if match is None:
                    continue
                issue_id = match.group(1)
                issue_status = issue_statuses.get(issue_id)
                if issue_status is None or issue_status == "closed":
                    refs.append(
                        TrackedIssueReference(
                            path=path,
                            line_number=line_number,
                            issue_id=issue_id,
                            issue_status=issue_status,
                        )
                    )
    return refs


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


def _parse_mapping_cell(
    value: str, *, label: str, allow_na: bool
) -> str | None:
    trimmed = value.strip()
    if allow_na and trimmed.lower() == "n/a":
        return None
    if not (trimmed.startswith("`") and trimmed.endswith("`")):
        raise ValueError(f"{label} must be backtick-wrapped command/tool text or `n/a`")
    inner = trimmed[1:-1].strip()
    if not inner:
        raise ValueError(f"{label} must not be empty")
    return inner


def parse_control_plane_host_action_contract(
    contract_path: Path,
) -> Dict[str, Tuple[str | None, str | None, str | None]]:
    payload = json.loads(contract_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{contract_path}: root must be an object")
    if payload.get("schema") != 1:
        raise ValueError(f"{contract_path}: schema must be 1")
    if payload.get("contractKind") != "premath.control_plane.contract.v1":
        raise ValueError(f"{contract_path}: contractKind mismatch")
    host_action_surface = payload.get("hostActionSurface")
    if not isinstance(host_action_surface, dict):
        raise ValueError(f"{contract_path}: hostActionSurface must be an object")
    required_actions = host_action_surface.get("requiredActions")
    if not isinstance(required_actions, dict) or not required_actions:
        raise ValueError(f"{contract_path}: hostActionSurface.requiredActions must be a non-empty object")
    out: Dict[str, Tuple[str | None, str | None, str | None]] = {}
    for host_action_id, row in sorted(required_actions.items()):
        if not isinstance(host_action_id, str) or not host_action_id.strip():
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions key must be a non-empty string"
            )
        if not isinstance(row, dict):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id} must be an object"
            )
        canonical_cli = row.get("canonicalCli")
        if canonical_cli is not None and (
            not isinstance(canonical_cli, str) or not canonical_cli.strip()
        ):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id}.canonicalCli must be a non-empty string or null"
            )
        mcp_tool = row.get("mcpTool")
        if mcp_tool is not None and (
            not isinstance(mcp_tool, str) or not mcp_tool.strip()
        ):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id}.mcpTool must be a non-empty string or null"
            )
        operation_id = row.get("operationId")
        if operation_id is not None and (
            not isinstance(operation_id, str) or not operation_id.strip()
        ):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id}.operationId must be a non-empty string or null"
            )
        if canonical_cli is None and mcp_tool is None:
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id} must bind canonicalCli or mcpTool"
            )
        expected_mcp_operation = (
            f"op/mcp.{mcp_tool.strip()}" if isinstance(mcp_tool, str) else None
        )
        if expected_mcp_operation is not None and (
            not isinstance(operation_id, str) or operation_id.strip() != expected_mcp_operation
        ):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id}.operationId must match mcpTool binding ({expected_mcp_operation})"
            )
        expected_harness_operation = HOST_ACTION_HARNESS_SESSION_OPERATION_IDS.get(
            host_action_id.strip()
        )
        if expected_harness_operation is not None and (
            not isinstance(operation_id, str) or operation_id.strip() != expected_harness_operation
        ):
            raise ValueError(
                f"{contract_path}: hostActionSurface.requiredActions.{host_action_id}.operationId must match harness session binding ({expected_harness_operation})"
            )
        out[host_action_id.strip()] = (
            canonical_cli.strip() if isinstance(canonical_cli, str) else None,
            mcp_tool.strip() if isinstance(mcp_tool, str) else None,
            operation_id.strip() if isinstance(operation_id, str) else None,
        )
    mcp_only_host_actions = host_action_surface.get("mcpOnlyHostActions")
    if not isinstance(mcp_only_host_actions, list) or not mcp_only_host_actions:
        raise ValueError(
            f"{contract_path}: hostActionSurface.mcpOnlyHostActions must be a non-empty list"
        )
    parsed_mcp_only: List[str] = []
    for idx, host_action_id in enumerate(mcp_only_host_actions):
        if not isinstance(host_action_id, str) or not host_action_id.strip():
            raise ValueError(
                f"{contract_path}: hostActionSurface.mcpOnlyHostActions[{idx}] must be a non-empty string"
            )
        parsed_mcp_only.append(host_action_id.strip())
    if len(set(parsed_mcp_only)) != len(parsed_mcp_only):
        raise ValueError(
            f"{contract_path}: hostActionSurface.mcpOnlyHostActions must not contain duplicates"
        )
    for host_action_id in parsed_mcp_only:
        row = out.get(host_action_id)
        if row is None:
            raise ValueError(
                f"{contract_path}: hostActionSurface.mcpOnlyHostActions references unknown action: {host_action_id!r}"
            )
        if row[0] is not None:
            raise ValueError(
                f"{contract_path}: hostActionSurface.mcpOnlyHostActions action {host_action_id!r} must have canonicalCli=null"
            )
        if row[1] is None:
            raise ValueError(
                f"{contract_path}: hostActionSurface.mcpOnlyHostActions action {host_action_id!r} must bind mcpTool"
            )
    failure_classes = host_action_surface.get("failureClasses")
    if not isinstance(failure_classes, dict):
        raise ValueError(f"{contract_path}: hostActionSurface.failureClasses must be an object")
    missing_failure_keys = sorted(set(HOST_ACTION_FAILURE_CLASS_KEYS) - set(failure_classes))
    if missing_failure_keys:
        raise ValueError(
            f"{contract_path}: hostActionSurface.failureClasses missing required keys: {missing_failure_keys}"
        )
    unknown_failure_keys = sorted(set(failure_classes) - set(HOST_ACTION_FAILURE_CLASS_KEYS))
    if unknown_failure_keys:
        raise ValueError(
            f"{contract_path}: hostActionSurface.failureClasses includes unknown keys: {unknown_failure_keys}"
        )
    return out


def parse_steel_host_action_mapping_table(
    design_doc_path: Path,
) -> Dict[str, Tuple[str | None, str | None, str | None]]:
    text = load_text(design_doc_path)
    section = extract_section_between(
        text,
        STEEL_HOST_ACTION_SECTION_START,
        STEEL_HOST_ACTION_SECTION_END,
    )
    out: Dict[str, Tuple[str | None, str | None, str | None]] = {}
    for line_number, line in enumerate(section.splitlines(), start=1):
        line = line.strip()
        if not line.startswith("|"):
            continue
        if "Host function id" in line or line.startswith("|---"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 4:
            raise ValueError(
                f"{design_doc_path}: malformed host-action table row at local line {line_number}"
            )
        host_id_raw, cli_raw, mcp_raw, operation_raw = cells
        host_id = _parse_mapping_cell(
            host_id_raw,
            label=f"{design_doc_path}: host-action table host id (line {line_number})",
            allow_na=False,
        )
        assert host_id is not None
        if host_id in out:
            raise ValueError(f"{design_doc_path}: duplicate host-action id in table: {host_id!r}")
        cli = _parse_mapping_cell(
            cli_raw,
            label=f"{design_doc_path}: host-action table CLI surface for {host_id!r}",
            allow_na=True,
        )
        mcp = _parse_mapping_cell(
            mcp_raw,
            label=f"{design_doc_path}: host-action table MCP tool for {host_id!r}",
            allow_na=True,
        )
        operation_id = _parse_mapping_cell(
            operation_raw,
            label=f"{design_doc_path}: host-action table operation id for {host_id!r}",
            allow_na=True,
        )
        if cli is None and mcp is None:
            raise ValueError(
                f"{design_doc_path}: host-action table row for {host_id!r} must bind CLI or MCP"
            )
        if mcp is not None:
            expected_mcp_operation = f"op/mcp.{mcp}"
            if operation_id != expected_mcp_operation:
                raise ValueError(
                    f"{design_doc_path}: host-action table operation id for {host_id!r} must match mcp tool binding ({expected_mcp_operation})"
                )
        expected_harness_operation = HOST_ACTION_HARNESS_SESSION_OPERATION_IDS.get(host_id)
        if expected_harness_operation is not None and operation_id != expected_harness_operation:
            raise ValueError(
                f"{design_doc_path}: host-action table operation id for {host_id!r} must match harness session binding ({expected_harness_operation})"
            )
        out[host_id] = (cli, mcp, operation_id)
    if not out:
        raise ValueError(f"{design_doc_path}: host-action mapping table has no data rows")
    return out


def parse_control_plane_stage1_contract(contract_path: Path) -> Dict[str, Dict[str, object]]:
    payload = json.loads(contract_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{contract_path}: root must be an object")
    if payload.get("schema") != 1:
        raise ValueError(f"{contract_path}: schema must be 1")
    if payload.get("contractKind") != "premath.control_plane.contract.v1":
        raise ValueError(f"{contract_path}: contractKind mismatch")
    lifecycle_rollover_epoch: str | None = None
    schema_lifecycle = payload.get("schemaLifecycle")
    if isinstance(schema_lifecycle, dict):
        kind_families = schema_lifecycle.get("kindFamilies")
        if isinstance(kind_families, dict):
            support_epochs = set()
            for family in kind_families.values():
                if not isinstance(family, dict):
                    continue
                aliases = family.get("compatibilityAliases")
                if not isinstance(aliases, list):
                    continue
                for alias in aliases:
                    if not isinstance(alias, dict):
                        continue
                    epoch = alias.get("supportUntilEpoch")
                    if isinstance(epoch, str) and epoch.strip():
                        support_epochs.add(epoch.strip())
            if len(support_epochs) == 1:
                lifecycle_rollover_epoch = next(iter(support_epochs))

    stage1_parity = payload.get("evidenceStage1Parity")
    if not isinstance(stage1_parity, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Parity must be an object")
    profile_kind = stage1_parity.get("profileKind")
    route = stage1_parity.get("authorityToTypedCoreRoute")
    if not isinstance(profile_kind, str) or not profile_kind.strip():
        raise ValueError(f"{contract_path}: evidenceStage1Parity.profileKind must be a non-empty string")
    if not isinstance(route, str) or not route.strip():
        raise ValueError(
            f"{contract_path}: evidenceStage1Parity.authorityToTypedCoreRoute must be a non-empty string"
        )
    comparison_tuple = stage1_parity.get("comparisonTuple")
    if not isinstance(comparison_tuple, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Parity.comparisonTuple must be an object")
    for key in ("authorityDigestRef", "typedCoreDigestRef", "normalizerIdRef", "policyDigestRef"):
        value = comparison_tuple.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(
                f"{contract_path}: evidenceStage1Parity.comparisonTuple.{key} must be a non-empty string"
            )
    if comparison_tuple.get("normalizerIdRef") != "normalizerId":
        raise ValueError(
            f"{contract_path}: evidenceStage1Parity.comparisonTuple.normalizerIdRef must be `normalizerId`"
        )
    if comparison_tuple.get("policyDigestRef") != "policyDigest":
        raise ValueError(
            f"{contract_path}: evidenceStage1Parity.comparisonTuple.policyDigestRef must be `policyDigest`"
        )
    parity_classes = stage1_parity.get("failureClasses")
    if not isinstance(parity_classes, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Parity.failureClasses must be an object")
    parsed_parity_classes = (
        parity_classes.get("missing"),
        parity_classes.get("mismatch"),
        parity_classes.get("unbound"),
    )
    if parsed_parity_classes != STAGE1_PARITY_CANONICAL_CLASSES:
        raise ValueError(
            f"{contract_path}: evidenceStage1Parity.failureClasses must map to canonical Stage 1 parity classes"
        )

    stage1_rollback = payload.get("evidenceStage1Rollback")
    if not isinstance(stage1_rollback, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Rollback must be an object")
    for key in ("profileKind", "witnessKind", "fromStage", "toStage"):
        value = stage1_rollback.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(f"{contract_path}: evidenceStage1Rollback.{key} must be a non-empty string")
    if stage1_rollback.get("fromStage") != "stage1":
        raise ValueError(f"{contract_path}: evidenceStage1Rollback.fromStage must be `stage1`")
    if stage1_rollback.get("toStage") != "stage0":
        raise ValueError(f"{contract_path}: evidenceStage1Rollback.toStage must be `stage0`")
    trigger_failure_classes = stage1_rollback.get("triggerFailureClasses")
    if not isinstance(trigger_failure_classes, list) or not trigger_failure_classes:
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.triggerFailureClasses must be a non-empty list"
        )
    parsed_trigger_classes: List[str] = []
    for idx, item in enumerate(trigger_failure_classes):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(
                f"{contract_path}: evidenceStage1Rollback.triggerFailureClasses[{idx}] must be a non-empty string"
            )
        parsed_trigger_classes.append(item.strip())
    if len(set(parsed_trigger_classes)) != len(parsed_trigger_classes):
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.triggerFailureClasses must not contain duplicates"
        )
    missing_trigger_classes = sorted(set(STAGE1_PARITY_CANONICAL_CLASSES) - set(parsed_trigger_classes))
    if missing_trigger_classes:
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.triggerFailureClasses missing canonical Stage 1 parity classes: {missing_trigger_classes}"
        )

    identity_refs = stage1_rollback.get("identityRefs")
    if not isinstance(identity_refs, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Rollback.identityRefs must be an object")
    for key in ("authorityDigestRef", "rollbackAuthorityDigestRef", "normalizerIdRef", "policyDigestRef"):
        value = identity_refs.get(key)
        if not isinstance(value, str) or not value.strip():
            raise ValueError(
                f"{contract_path}: evidenceStage1Rollback.identityRefs.{key} must be a non-empty string"
            )
    if identity_refs.get("authorityDigestRef") == identity_refs.get("rollbackAuthorityDigestRef"):
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.identityRefs authority/rollback refs must differ"
        )
    if identity_refs.get("normalizerIdRef") != "normalizerId":
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.identityRefs.normalizerIdRef must be `normalizerId`"
        )
    if identity_refs.get("policyDigestRef") != "policyDigest":
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.identityRefs.policyDigestRef must be `policyDigest`"
        )

    rollback_classes = stage1_rollback.get("failureClasses")
    if not isinstance(rollback_classes, dict):
        raise ValueError(f"{contract_path}: evidenceStage1Rollback.failureClasses must be an object")
    parsed_rollback_classes = (
        rollback_classes.get("precondition"),
        rollback_classes.get("identityDrift"),
        rollback_classes.get("unbound"),
    )
    if parsed_rollback_classes != STAGE1_ROLLBACK_CANONICAL_CLASSES:
        raise ValueError(
            f"{contract_path}: evidenceStage1Rollback.failureClasses must map to canonical Stage 1 rollback classes"
        )

    out: Dict[str, Dict[str, object]] = {
        "parity": {
            "profileKind": profile_kind.strip(),
            "authorityToTypedCoreRoute": route.strip(),
            "failureClasses": parsed_parity_classes,
        },
        "rollback": {
            "profileKind": str(stage1_rollback.get("profileKind", "")).strip(),
            "witnessKind": str(stage1_rollback.get("witnessKind", "")).strip(),
            "triggerFailureClasses": parsed_trigger_classes,
            "failureClasses": parsed_rollback_classes,
        },
    }

    stage2_authority = payload.get("evidenceStage2Authority")
    if stage2_authority is not None:
        if not isinstance(stage2_authority, dict):
            raise ValueError(f"{contract_path}: evidenceStage2Authority must be an object")
        stage2_profile_kind = stage2_authority.get("profileKind")
        stage2_active_stage = stage2_authority.get("activeStage")
        if not isinstance(stage2_profile_kind, str) or not stage2_profile_kind.strip():
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.profileKind must be a non-empty string"
            )
        if stage2_active_stage != "stage2":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.activeStage must be `stage2`"
            )
        typed_authority = stage2_authority.get("typedAuthority")
        if not isinstance(typed_authority, dict):
            raise ValueError(f"{contract_path}: evidenceStage2Authority.typedAuthority must be an object")
        for key in ("kindRef", "digestRef", "normalizerIdRef", "policyDigestRef"):
            value = typed_authority.get(key)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.typedAuthority.{key} must be a non-empty string"
                )
        if typed_authority.get("normalizerIdRef") != "normalizerId":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.typedAuthority.normalizerIdRef must be `normalizerId`"
            )
        if typed_authority.get("policyDigestRef") != "policyDigest":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.typedAuthority.policyDigestRef must be `policyDigest`"
            )

        compatibility_alias = stage2_authority.get("compatibilityAlias")
        if not isinstance(compatibility_alias, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.compatibilityAlias must be an object"
            )
        for key in ("kindRef", "digestRef", "role", "supportUntilEpoch"):
            value = compatibility_alias.get(key)
            if not isinstance(value, str) or not value.strip():
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.compatibilityAlias.{key} must be a non-empty string"
                )
        if compatibility_alias.get("role") != "projection_only":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.compatibilityAlias.role must be `projection_only`"
            )
        if lifecycle_rollover_epoch is None:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority requires one lifecycle rollover epoch"
            )
        if compatibility_alias.get("supportUntilEpoch") != lifecycle_rollover_epoch:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.compatibilityAlias.supportUntilEpoch must align with lifecycle rollover epoch"
            )

        bidir_route = stage2_authority.get("bidirEvidenceRoute")
        if not isinstance(bidir_route, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute must be an object"
            )
        route_kind = bidir_route.get("routeKind")
        if route_kind != "direct_checker_discharge":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.routeKind must be `direct_checker_discharge`"
            )
        obligation_field_ref = bidir_route.get("obligationFieldRef")
        if obligation_field_ref != "bidirCheckerObligations":
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.obligationFieldRef must be `bidirCheckerObligations`"
            )
        required_obligations = bidir_route.get("requiredObligations")
        if not isinstance(required_obligations, list) or not required_obligations:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must be a non-empty list"
            )
        parsed_required_obligations: List[str] = []
        for idx, item in enumerate(required_obligations):
            if not isinstance(item, str) or not item.strip():
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.requiredObligations[{idx}] must be a non-empty string"
                )
            parsed_required_obligations.append(item.strip())
        if len(set(parsed_required_obligations)) != len(parsed_required_obligations):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must not contain duplicates"
            )
        if set(parsed_required_obligations) != set(STAGE2_REQUIRED_KERNEL_OBLIGATIONS):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must match canonical Stage 2 kernel obligations"
            )
        bidir_failure_classes = bidir_route.get("failureClasses")
        if not isinstance(bidir_failure_classes, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.failureClasses must be an object"
            )
        parsed_bidir_classes = (
            bidir_failure_classes.get("missing"),
            bidir_failure_classes.get("drift"),
        )
        if parsed_bidir_classes != STAGE2_KERNEL_COMPLIANCE_CANONICAL_CLASSES:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.bidirEvidenceRoute.failureClasses must map to canonical Stage 2 kernel-compliance classes"
            )
        kernel_sentinel = stage2_authority.get("kernelComplianceSentinel")
        if kernel_sentinel is not None:
            if not isinstance(kernel_sentinel, dict):
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel must be an object when present"
                )
            sentinel_required_obligations = kernel_sentinel.get("requiredObligations")
            if not isinstance(sentinel_required_obligations, list) or not sentinel_required_obligations:
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must be a non-empty list when present"
                )
            parsed_sentinel_required: List[str] = []
            for idx, item in enumerate(sentinel_required_obligations):
                if not isinstance(item, str) or not item.strip():
                    raise ValueError(
                        f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations[{idx}] must be a non-empty string"
                    )
                parsed_sentinel_required.append(item.strip())
            if set(parsed_sentinel_required) != set(parsed_required_obligations):
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must match evidenceStage2Authority.bidirEvidenceRoute.requiredObligations"
                )
            sentinel_failure_classes = kernel_sentinel.get("failureClasses")
            if not isinstance(sentinel_failure_classes, dict):
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.failureClasses must be an object"
                )
            parsed_sentinel_classes = (
                sentinel_failure_classes.get("missing"),
                sentinel_failure_classes.get("drift"),
            )
            if parsed_sentinel_classes != parsed_bidir_classes:
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.failureClasses must match evidenceStage2Authority.bidirEvidenceRoute.failureClasses"
                )

        stage2_failure_classes = stage2_authority.get("failureClasses")
        if not isinstance(stage2_failure_classes, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.failureClasses must be an object"
            )
        parsed_stage2_classes = (
            stage2_failure_classes.get("authorityAliasViolation"),
            stage2_failure_classes.get("aliasWindowViolation"),
            stage2_failure_classes.get("unbound"),
        )
        if parsed_stage2_classes != STAGE2_AUTHORITY_CANONICAL_CLASSES:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.failureClasses must map to canonical Stage 2 classes"
            )
        out["stage2"] = {
            "profileKind": stage2_profile_kind.strip(),
            "activeStage": "stage2",
            "routeKind": route_kind,
            "obligationFieldRef": obligation_field_ref,
            "requiredObligations": parsed_required_obligations,
            "failureClasses": parsed_stage2_classes,
            "bidirFailureClasses": parsed_bidir_classes,
        }

    return out


def parse_spec_index_capability_doc_map(section_54: str) -> Dict[str, str]:
    pattern = re.compile(r"- `([^`]+)`\s+\(for `([^`]+)`\)")
    out: Dict[str, str] = {}
    for doc_ref, capability_id in pattern.findall(section_54):
        out[doc_ref] = capability_id
    return out


def verify_conditional_normative_entry(section_55: str, doc_ref: str, capability_id: str) -> bool:
    pattern = re.compile(
        rf"`{re.escape(doc_ref)}`[\s\S]*?normative(?:\s+only)?\s+when\s+`{re.escape(capability_id)}`\s+is\s+claimed",
        re.IGNORECASE,
    )
    return pattern.search(section_55) is not None


def sorted_csv(values: Sequence[str]) -> str:
    return ", ".join(sorted(values))


def find_missing_markers(text: str, markers: Sequence[str]) -> List[str]:
    return [marker for marker in markers if marker not in text]


def check_doctrine_generation_digest_contract(path: Path, repo_root: Path) -> List[str]:
    errors: List[str] = []
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001
        return [f"{path.relative_to(repo_root).as_posix()}: unreadable JSON ({exc})"]
    if not isinstance(payload, dict):
        return [f"{path.relative_to(repo_root).as_posix()}: top-level object required"]
    if payload.get("schema") != 1:
        errors.append(f"{path.relative_to(repo_root).as_posix()}: schema must equal 1")
    digest_kind = payload.get("digestKind")
    if digest_kind != DOCTRINE_SITE_GENERATION_DIGEST_KIND:
        errors.append(
            f"{path.relative_to(repo_root).as_posix()}: digestKind must equal {DOCTRINE_SITE_GENERATION_DIGEST_KIND!r}"
        )
    source = payload.get("source")
    if not isinstance(source, dict):
        errors.append(f"{path.relative_to(repo_root).as_posix()}: source object required")
    else:
        for key in ("packagesRoot", "packageGlob", "generator", "cutoverContract"):
            value = source.get(key)
            if not isinstance(value, str) or not value.strip():
                errors.append(
                    f"{path.relative_to(repo_root).as_posix()}: source.{key} must be a non-empty string"
                )
        cutover_path_raw = source.get("cutoverContract")
        if isinstance(cutover_path_raw, str) and cutover_path_raw.strip():
            cutover_path = (repo_root / cutover_path_raw).resolve()
            if not cutover_path.exists():
                errors.append(
                    f"{path.relative_to(repo_root).as_posix()}: source.cutoverContract path missing: {cutover_path_raw}"
                )
    artifacts = payload.get("artifacts")
    if not isinstance(artifacts, dict):
        errors.append(f"{path.relative_to(repo_root).as_posix()}: artifacts object required")
        return errors

    expected_keys = ("siteInput", "siteMap", "operationRegistry")
    missing = sorted(set(expected_keys).difference(artifacts))
    if missing:
        errors.append(
            f"{path.relative_to(repo_root).as_posix()}: artifacts missing required keys {missing}"
        )
    for key in expected_keys:
        row = artifacts.get(key)
        if not isinstance(row, dict):
            continue
        artifact_path_raw = row.get("path")
        artifact_sha_raw = row.get("sha256")
        if not isinstance(artifact_path_raw, str) or not artifact_path_raw.strip():
            errors.append(
                f"{path.relative_to(repo_root).as_posix()}: artifacts.{key}.path must be a non-empty string"
            )
            continue
        if not isinstance(artifact_sha_raw, str) or not artifact_sha_raw.strip():
            errors.append(
                f"{path.relative_to(repo_root).as_posix()}: artifacts.{key}.sha256 must be a non-empty string"
            )
            continue
        artifact_path = (repo_root / artifact_path_raw).resolve()
        if not artifact_path.exists():
            errors.append(
                f"{path.relative_to(repo_root).as_posix()}: artifacts.{key}.path missing: {artifact_path_raw}"
            )
            continue
        try:
            artifact_payload = doctrine_site_contract.load_json_object(artifact_path)
            if key == "siteInput":
                digest = doctrine_site_contract.site_input_digest(artifact_payload)
            elif key == "siteMap":
                digest = doctrine_site_contract.site_map_digest(artifact_payload)
            else:
                digest = doctrine_site_contract.operation_registry_digest(artifact_payload)
        except Exception as exc:  # noqa: BLE001
            errors.append(
                f"{path.relative_to(repo_root).as_posix()}: failed to digest artifacts.{key} ({artifact_path_raw}): {exc}"
            )
            continue
        if digest != artifact_sha_raw:
            errors.append(
                f"{path.relative_to(repo_root).as_posix()}: artifacts.{key}.sha256 mismatch (expected {digest}, got {artifact_sha_raw})"
            )

    return errors


def check_doctrine_site_cutover_contract(path: Path, repo_root: Path) -> List[str]:
    errors: List[str] = []
    rel = path.relative_to(repo_root).as_posix()
    if not path.exists():
        return [f"{rel}: missing doctrine site cutover contract"]
    try:
        payload = doctrine_site_contract.load_json_object(path)
        canonical = doctrine_site_contract.canonicalize_cutover_contract(
            payload,
            label=rel,
        )
        phase = doctrine_site_contract.current_cutover_phase_policy(canonical)
    except Exception as exc:  # noqa: BLE001
        return [f"{rel}: invalid doctrine site cutover contract ({exc})"]

    if bool(phase.get("allowLegacySourceKind")):
        errors.append(
            f"{rel}: current cutover phase must disable legacy sourceKind fallback"
        )
    if bool(phase.get("allowOperationRegistryOverride")):
        errors.append(
            f"{rel}: current cutover phase must disable operation-registry override"
        )
    return errors


def check_doctrine_site_inventory(
    *,
    repo_root: Path,
    packages_root: Path,
    site_input_path: Path,
    site_map_path: Path,
    operation_registry_path: Path,
    control_plane_contract_path: Path,
    cutover_contract_path: Path,
    inventory_json_path: Path,
    inventory_docs_path: Path,
) -> List[str]:
    errors: List[str] = []
    try:
        inventory = generate_doctrine_site_inventory.build_inventory(
            repo_root=repo_root,
            packages_root=packages_root,
            site_input_path=site_input_path,
            site_map_path=site_map_path,
            operation_registry_path=operation_registry_path,
            control_plane_contract_path=control_plane_contract_path,
            cutover_contract_path=cutover_contract_path,
        )
        expected_json = json.dumps(inventory, indent=2, sort_keys=False) + "\n"
        expected_docs = generate_doctrine_site_inventory.render_markdown(inventory)
    except Exception as exc:  # noqa: BLE001
        return [f"doctrine site inventory generation failed: {exc}"]

    if not inventory_json_path.exists():
        errors.append(
            "missing doctrine site inventory JSON: "
            f"{inventory_json_path.relative_to(repo_root).as_posix()}"
        )
    else:
        current = inventory_json_path.read_text(encoding="utf-8")
        if current != expected_json:
            errors.append(
                "doctrine site inventory JSON drifted from generated output: "
                f"{inventory_json_path.relative_to(repo_root).as_posix()}"
            )
    if not inventory_docs_path.exists():
        errors.append(
            "missing doctrine site inventory docs index: "
            f"{inventory_docs_path.relative_to(repo_root).as_posix()}"
        )
    else:
        current = inventory_docs_path.read_text(encoding="utf-8")
        if current != expected_docs:
            errors.append(
                "doctrine site inventory docs index drifted from generated output: "
                f"{inventory_docs_path.relative_to(repo_root).as_posix()}"
            )
    return errors


def main() -> int:
    args = parse_args()
    root = args.repo_root.resolve()
    errors: List[str] = []

    capability_registry = root / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
    control_plane_contract = root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
    cargo_toml = root / "Cargo.toml"
    issues_jsonl = root / ".premath" / "issues.jsonl"
    mise_toml = root / ".mise.toml"
    readme = root / "README.md"
    conformance_readme = root / "tools" / "conformance" / "README.md"
    ci_closure = root / "docs" / "design" / "CI-CLOSURE.md"
    architecture_map = root / "docs" / "design" / "ARCHITECTURE-MAP.md"
    spec_index = root / "specs" / "premath" / "draft" / "SPEC-INDEX.md"
    doctrine_generation_digest = (
        root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-GENERATION-DIGEST.json"
    )
    doctrine_site_input = (
        root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json"
    )
    doctrine_site_map = root / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json"
    doctrine_operation_registry = (
        root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
    )
    doctrine_cutover_contract = (
        root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-CUTOVER.json"
    )
    doctrine_packages_root = root / "specs" / "premath" / "site-packages"
    doctrine_inventory_json = (
        root / "docs" / "design" / "generated" / "DOCTRINE-SITE-INVENTORY.json"
    )
    doctrine_inventory_docs = (
        root / "docs" / "design" / "generated" / "DOCTRINE-SITE-INVENTORY.md"
    )
    conformance = root / "specs" / "premath" / "draft" / "CONFORMANCE.md"
    unification_doctrine = root / "specs" / "premath" / "draft" / "UNIFICATION-DOCTRINE.md"
    span_square_checking = root / "specs" / "premath" / "draft" / "SPAN-SQUARE-CHECKING.md"
    pre_math_coherence = root / "specs" / "premath" / "draft" / "PREMATH-COHERENCE.md"
    capability_vectors = root / "specs" / "premath" / "draft" / "CAPABILITY-VECTORS.md"
    adjoints_profile = root / "specs" / "premath" / "profile" / "ADJOINTS-AND-SITES.md"
    steel_repl_descent_control = root / "docs" / "design" / "STEEL-REPL-DESCENT-CONTROL.md"
    roadmap = root / "specs" / "premath" / "raw" / "ROADMAP.md"
    fixtures_root = root / "tests" / "conformance" / "fixtures" / "capabilities"

    registry_contract = parse_capability_registry(capability_registry)
    executable_capabilities = registry_contract.executable_capabilities
    registry_overlay_claims = registry_contract.profile_overlay_claims
    registry_doc_map = registry_contract.capability_doc_map
    executable_capability_set = set(executable_capabilities)
    conformance_overlay_claims = parse_conformance_overlay_claims(conformance)
    workspace_members = set(parse_workspace_members(cargo_toml, root))
    issue_statuses = parse_issue_statuses(issues_jsonl)

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
    readme_text = load_text(readme)
    readme_workspace_crates = set(parse_readme_workspace_crates(readme_text))
    conformance_readme_caps = set(BACKTICK_CAP_RE.findall(load_text(conformance_readme)))
    architecture_text = load_text(architecture_map)

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
    expected_readme_workspace_crates = workspace_members - set(README_WORKSPACE_CRATE_ALLOWLIST)
    if readme_workspace_crates != expected_readme_workspace_crates:
        missing = sorted(expected_readme_workspace_crates - readme_workspace_crates)
        extra = sorted(readme_workspace_crates - expected_readme_workspace_crates)
        errors.append(
            "README workspace layering crate list mismatch with Cargo workspace members: "
            f"missing={missing}, extra={extra}"
        )
    missing_readme_doctrine_markers = find_missing_markers(readme_text, README_DOCTRINE_MARKERS)
    for marker in missing_readme_doctrine_markers:
        errors.append(f"README doctrine-check marker missing: {marker}")
    missing_architecture_doctrine_markers = find_missing_markers(
        architecture_text, ARCHITECTURE_DOCTRINE_MARKERS
    )
    for marker in missing_architecture_doctrine_markers:
        errors.append(f"ARCHITECTURE-MAP doctrine marker missing: {marker}")
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
    if set(conformance_overlay_claims) != set(registry_overlay_claims):
        missing_in_conformance = sorted(set(registry_overlay_claims) - set(conformance_overlay_claims))
        missing_in_registry = sorted(set(conformance_overlay_claims) - set(registry_overlay_claims))
        errors.append(
            "CONFORMANCE §2.4 profile-overlay claim list mismatch with CAPABILITY-REGISTRY profileOverlayClaims: "
            f"missingInConformance={missing_in_conformance}, missingInRegistry={missing_in_registry}"
        )

    informative_clause = "unless they are\nexplicitly claimed under §5.4 or §5.6"
    if informative_clause not in section_55:
        errors.append(
            "SPEC-INDEX §5.5 must explicitly state informative/default status unless claimed under §5.4 or §5.6"
        )

    if spec_index_doc_map != registry_doc_map:
        missing_in_spec_index = sorted(set(registry_doc_map) - set(spec_index_doc_map))
        missing_in_registry = sorted(set(spec_index_doc_map) - set(registry_doc_map))
        mismatched = sorted(
            doc_ref
            for doc_ref in set(spec_index_doc_map) & set(registry_doc_map)
            if spec_index_doc_map[doc_ref] != registry_doc_map[doc_ref]
        )
        errors.append(
            "SPEC-INDEX §5.4 capability doc mapping mismatch with CAPABILITY-REGISTRY capabilityDocBindings: "
            f"missingInSpecIndex={missing_in_spec_index}, missingInRegistry={missing_in_registry}, mismatched={mismatched}"
        )
    for doc_ref, capability_id in sorted(registry_doc_map.items()):
        if f"`{doc_ref}`" not in section_55:
            continue
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
    missing_internalization_markers = find_missing_markers(
        unification_text, UNIFICATION_INTERNALIZATION_MARKERS
    )
    for marker in missing_internalization_markers:
        errors.append(
            f"UNIFICATION-DOCTRINE missing typed evidence internalization marker: {marker}"
        )
    missing_stage1_markers = find_missing_markers(
        unification_text, UNIFICATION_STAGE1_PROFILE_MARKERS
    )
    for marker in missing_stage1_markers:
        errors.append(f"UNIFICATION-DOCTRINE missing Stage 1 profile marker: {marker}")
    missing_stage3_markers = find_missing_markers(
        unification_text, UNIFICATION_STAGE3_CLOSURE_MARKERS
    )
    for marker in missing_stage3_markers:
        errors.append(f"UNIFICATION-DOCTRINE missing Stage 3 closure marker: {marker}")
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
    contract_host_actions = parse_control_plane_host_action_contract(control_plane_contract)
    docs_host_actions = parse_steel_host_action_mapping_table(steel_repl_descent_control)
    parse_control_plane_stage1_contract(control_plane_contract)
    missing_host_actions_in_docs = sorted(
        set(contract_host_actions) - set(docs_host_actions)
    )
    missing_host_actions_in_contract = sorted(
        set(docs_host_actions) - set(contract_host_actions)
    )
    mismatched_host_action_bindings = sorted(
        host_action_id
        for host_action_id in set(contract_host_actions) & set(docs_host_actions)
        if contract_host_actions[host_action_id] != docs_host_actions[host_action_id]
    )
    if (
        missing_host_actions_in_docs
        or missing_host_actions_in_contract
        or mismatched_host_action_bindings
    ):
        errors.append(
            "STEEL-REPL-DESCENT-CONTROL §5.1 host-action mapping mismatch with CONTROL-PLANE-CONTRACT hostActionSurface.requiredActions: "
            f"missingInDocs={missing_host_actions_in_docs}, "
            f"missingInContract={missing_host_actions_in_contract}, "
            f"mismatched={mismatched_host_action_bindings}"
        )

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
    missing_ci_doctrine_markers = find_missing_markers(
        ci_closure_text, CI_CLOSURE_DOCTRINE_MARKERS
    )
    for marker in missing_ci_doctrine_markers:
        errors.append(f"CI-CLOSURE doctrine-check semantics missing marker: {marker}")

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
    if not doctrine_generation_digest.exists():
        errors.append(
            "missing doctrine generation digest contract: "
            f"{doctrine_generation_digest.relative_to(root).as_posix()}"
        )
    else:
        errors.extend(
            check_doctrine_generation_digest_contract(doctrine_generation_digest, root)
        )
    errors.extend(check_doctrine_site_cutover_contract(doctrine_cutover_contract, root))
    errors.extend(
        check_doctrine_site_inventory(
            repo_root=root,
            packages_root=doctrine_packages_root,
            site_input_path=doctrine_site_input,
            site_map_path=doctrine_site_map,
            operation_registry_path=doctrine_operation_registry,
            control_plane_contract_path=control_plane_contract,
            cutover_contract_path=doctrine_cutover_contract,
            inventory_json_path=doctrine_inventory_json,
            inventory_docs_path=doctrine_inventory_docs,
        )
    )
    stale_issue_refs = find_stale_tracked_issue_references(
        roots=[root / "docs", root / "specs"],
        issue_statuses=issue_statuses,
        excluded_paths=[root / "specs" / "process" / "decision-log.md"],
    )
    for ref in stale_issue_refs:
        rel_path = ref.path.relative_to(root).as_posix()
        if ref.issue_status is None:
            errors.append(
                f"{rel_path}:{ref.line_number} tracked issue reference points to missing issue id {ref.issue_id}"
            )
        else:
            errors.append(
                f"{rel_path}:{ref.line_number} stale tracked issue reference uses closed issue {ref.issue_id}"
            )

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
