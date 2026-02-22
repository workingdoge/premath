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

        kernel_sentinel = stage2_authority.get("kernelComplianceSentinel")
        if not isinstance(kernel_sentinel, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel must be an object"
            )
        required_obligations = kernel_sentinel.get("requiredObligations")
        if not isinstance(required_obligations, list) or not required_obligations:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must be a non-empty list"
            )
        parsed_required_obligations: List[str] = []
        for idx, item in enumerate(required_obligations):
            if not isinstance(item, str) or not item.strip():
                raise ValueError(
                    f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations[{idx}] must be a non-empty string"
                )
            parsed_required_obligations.append(item.strip())
        if len(set(parsed_required_obligations)) != len(parsed_required_obligations):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must not contain duplicates"
            )
        if set(parsed_required_obligations) != set(STAGE2_REQUIRED_KERNEL_OBLIGATIONS):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must match canonical Stage 2 kernel obligations"
            )
        kernel_sentinel_failure_classes = kernel_sentinel.get("failureClasses")
        if not isinstance(kernel_sentinel_failure_classes, dict):
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.failureClasses must be an object"
            )
        parsed_kernel_sentinel_classes = (
            kernel_sentinel_failure_classes.get("missing"),
            kernel_sentinel_failure_classes.get("drift"),
        )
        if parsed_kernel_sentinel_classes != STAGE2_KERNEL_COMPLIANCE_CANONICAL_CLASSES:
            raise ValueError(
                f"{contract_path}: evidenceStage2Authority.kernelComplianceSentinel.failureClasses must map to canonical Stage 2 kernel-compliance classes"
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
            "requiredObligations": parsed_required_obligations,
            "failureClasses": parsed_stage2_classes,
            "kernelComplianceFailureClasses": parsed_kernel_sentinel_classes,
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
    parse_control_plane_stage1_contract(control_plane_contract)

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
