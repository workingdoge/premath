#!/usr/bin/env python3
"""Shared typed control-plane contract loader."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, Optional, Tuple


CONTROL_PLANE_CONTRACT_KIND = "premath.control_plane.contract.v1"
CONTROL_PLANE_CONTRACT_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "CONTROL-PLANE-CONTRACT.json"
)
DOCTRINE_OP_REGISTRY_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "DOCTRINE-OP-REGISTRY.json"
)
_EPOCH_RE = re.compile(r"^\d{4}-(0[1-9]|1[0-2])$")
_REQUIRED_SCHEMA_KIND_FAMILIES = (
    "controlPlaneContractKind",
    "requiredWitnessKind",
    "requiredDecisionKind",
    "instructionWitnessKind",
    "instructionPolicyKind",
    "requiredProjectionPolicy",
    "requiredDeltaKind",
)
_MAX_ALIAS_RUNWAY_MONTHS = 12
_SCHEMA_LIFECYCLE_GOVERNANCE_MODES = ("rollover", "freeze")
_STAGE1_PARITY_FAILURE_CLASSES = (
    "unification.evidence_stage1.parity.missing",
    "unification.evidence_stage1.parity.mismatch",
    "unification.evidence_stage1.parity.unbound",
)
_STAGE1_ROLLBACK_FAILURE_CLASSES = (
    "unification.evidence_stage1.rollback.precondition",
    "unification.evidence_stage1.rollback.identity_drift",
    "unification.evidence_stage1.rollback.unbound",
)
_STAGE2_AUTHORITY_FAILURE_CLASSES = (
    "unification.evidence_stage2.authority_alias_violation",
    "unification.evidence_stage2.alias_window_violation",
    "unification.evidence_stage2.unbound",
)
_STAGE2_KERNEL_COMPLIANCE_FAILURE_CLASSES = (
    "unification.evidence_stage2.kernel_compliance_missing",
    "unification.evidence_stage2.kernel_compliance_drift",
)
_STAGE2_REQUIRED_KERNEL_OBLIGATIONS = (
    "stability",
    "locality",
    "descent_exists",
    "descent_contractible",
    "adjoint_triple",
    "ext_gap",
    "ext_ambiguous",
)
_STAGE2_COMPATIBILITY_ALIAS_ROLE = "projection_only"
_STAGE2_BIDIR_EVIDENCE_ROUTE_KIND = "direct_checker_discharge"
_STAGE2_BIDIR_EVIDENCE_OBLIGATION_FIELD_REF = "bidirCheckerObligations"
_STAGE2_BIDIR_EVIDENCE_FALLBACK_MODE = "profile_gated_sentinel"
_WORKER_DEFAULT_MUTATION_MODE = "instruction-linked"
_WORKER_ALLOWED_MUTATION_MODES = (
    "instruction-linked",
    "human-override",
)
_WORKER_MUTATION_ROUTE_BINDINGS = {
    "issueClaim": "capabilities.change_morphisms.issue_claim",
    "issueLeaseRenew": "capabilities.change_morphisms.issue_lease_renew",
    "issueLeaseRelease": "capabilities.change_morphisms.issue_lease_release",
    "issueDiscover": "capabilities.change_morphisms.issue_discover",
}
_WORKER_FAILURE_CLASSES = (
    "worker_lane_policy_drift",
    "worker_lane_mutation_mode_drift",
    "worker_lane_route_unbound",
)
_WORLD_DESCENT_CONTRACT_ID = "doctrine.world_descent.v1"
_WORLD_DESCENT_REQUIRED_FAILURE_CLASS_KEYS = (
    "identityMissing",
    "descentDataMissing",
    "kcirHandoffIdentityMissing",
)
_WORLD_DESCENT_DEFAULT_REQUIRED_ROUTE_FAMILIES = (
    "route.gate_execution",
    "route.instruction_execution",
    "route.required_decision_attestation",
    "route.fiber.lifecycle",
    "route.issue_claim_lease",
    "route.session_projection",
    "route.transport.dispatch",
)
_WORLD_DESCENT_DEFAULT_REQUIRED_ACTION_ROUTE_BINDINGS = {
    "route.instruction_execution": (
        "instruction.run",
    ),
    "route.required_decision_attestation": (
        "required.witness_verify",
        "required.witness_decide",
    ),
    "route.fiber.lifecycle": (
        "fiber.spawn",
        "fiber.join",
        "fiber.cancel",
    ),
    "route.issue_claim_lease": (
        "issue.claim_next",
        "issue.claim",
        "issue.lease_renew",
        "issue.lease_release",
        "issue.discover",
    ),
}
_WORLD_DESCENT_DEFAULT_REQUIRED_STATIC_OPERATION_BINDINGS = {
    "route.transport.dispatch": (
        "op/transport.world_route_binding",
    ),
}
_WORLD_DESCENT_DEFAULT_FAILURE_CLASSES = {
    "identityMissing": "world_route_identity_missing",
    "descentDataMissing": "world_descent_data_missing",
    "kcirHandoffIdentityMissing": "kcir_handoff_identity_missing",
}
_REQUIRED_RUNTIME_ROUTE_FAILURE_CLASS_KEYS = (
    "missingRoute",
    "morphismDrift",
    "contractUnbound",
)
_REQUIRED_COMMAND_SURFACE_IDS = (
    "requiredDecision",
    "instructionEnvelopeCheck",
    "instructionDecision",
    "governancePromotionCheck",
    "kcirMappingCheck",
)
_REQUIRED_COMMAND_SURFACE_FAILURE_CLASS_KEYS = ("unbound",)
_REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS = ("governance", "kcirMapping")
_REQUIRED_PIPELINE_WRAPPER_FAILURE_CLASS_KEYS = (
    "unbound",
    "parityDrift",
    "governanceGateMissing",
    "kcirMappingGateMissing",
)
_HOST_ACTION_ID_RE = re.compile(r"^[a-z][a-z0-9_.]*$")
_HOST_ACTION_MCP_TOOL_RE = re.compile(r"^[a-z][a-z0-9_]*$")
_HOST_ACTION_OPERATION_ID_RE = re.compile(r"^op/[a-z0-9_.]+$")
_HARNESS_SESSION_OPERATION_IDS = {
    "harness.session.read": "op/harness.session_read",
    "harness.session.write": "op/harness.session_write",
    "harness.session.bootstrap": "op/harness.session_bootstrap",
}
_REQUIRED_HOST_ACTION_SURFACE_FAILURE_CLASS_KEYS = (
    "unregisteredHostId",
    "bindingMismatch",
    "duplicateBinding",
    "contractUnbound",
)
_CONTROL_PLANE_BUNDLE_PROFILE_ID = "cp.bundle.v0"
_CONTROL_PLANE_BUNDLE_CONTEXT_FAMILY_ID = "C_cp"
_CONTROL_PLANE_BUNDLE_CONTEXT_KINDS = (
    "repo_head",
    "workspace_delta",
    "instruction_envelope",
    "policy_snapshot",
    "witness_projection",
)
_CONTROL_PLANE_BUNDLE_MORPHISM_KINDS = (
    "ctx.identity",
    "ctx.rebase",
    "ctx.patch",
    "ctx.policy_rollover",
)
_CONTROL_PLANE_BUNDLE_ARTIFACT_FAMILY_ID = "E_cp"
_CONTROL_PLANE_BUNDLE_ARTIFACT_REFS = {
    "controlPlaneContract": "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
    "coherenceContract": "specs/premath/draft/COHERENCE-CONTRACT.json",
    "capabilityRegistry": "specs/premath/draft/CAPABILITY-REGISTRY.json",
    "doctrineSiteInput": "specs/premath/draft/DOCTRINE-SITE-INPUT.json",
    "doctrineOpRegistry": "specs/premath/draft/DOCTRINE-OP-REGISTRY.json",
}
_CONTROL_PLANE_BUNDLE_REINDEXING_OBLIGATIONS = (
    "identity_preserved",
    "composition_preserved",
    "policy_digest_stable",
    "route_bindings_total",
)
_CONTROL_PLANE_BUNDLE_COMMUTATION_WITNESS = "span_square_commutation"
_CONTROL_PLANE_BUNDLE_WORKER_COVER_KIND = "worktree_partition_cover"
_CONTROL_PLANE_BUNDLE_REQUIRED_MERGE_ARTIFACTS = (
    "ci.required.v1",
    "ci.instruction.v1",
    "coherence_witness",
)
_CONTROL_PLANE_BUNDLE_SEMANTIC_AUTHORITY = (
    "PREMATH-KERNEL",
    "GATE",
    "BIDIR-DESCENT",
)
_CONTROL_PLANE_BUNDLE_CONTROL_PLANE_ROLE = "projection_and_parity_only"
_CONTROL_PLANE_BUNDLE_FORBIDDEN_ROLES = (
    "semantic_obligation_discharge",
    "admissibility_override",
)


def _require_non_empty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value.strip()


def _require_object(value: Any, label: str) -> Dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object")
    return value


def _require_string_list(value: Any, label: str) -> Tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label} must be a non-empty list")
    out = []
    for idx, item in enumerate(value):
        out.append(_require_non_empty_string(item, f"{label}[{idx}]"))
    if len(set(out)) != len(out):
        raise ValueError(f"{label} must not contain duplicates")
    return tuple(out)


def _require_optional_string_list(value: Any, label: str) -> Tuple[str, ...]:
    if value is None:
        return tuple()
    return _require_string_list(value, label)


def _load_doctrine_operation_ids(path: Path) -> frozenset[str]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"failed to read doctrine op registry at {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid doctrine op registry JSON at {path}: {exc}") from exc
    if not isinstance(payload, dict):
        raise ValueError(f"doctrine op registry at {path} must be an object")
    operations = payload.get("operations")
    if not isinstance(operations, list) or not operations:
        raise ValueError(f"doctrine op registry at {path} must include non-empty operations")
    out: set[str] = set()
    for idx, row in enumerate(operations):
        if not isinstance(row, dict):
            raise ValueError(
                f"doctrine op registry at {path} operations[{idx}] must be an object"
            )
        operation_id = _require_non_empty_string(
            row.get("id"),
            f"doctrine op registry at {path} operations[{idx}].id",
        )
        out.add(operation_id)
    return frozenset(out)


def _require_command_tokens(value: Any, label: str) -> Tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label} must be a non-empty list")
    out: list[str] = []
    for idx, item in enumerate(value):
        out.append(_require_non_empty_string(item, f"{label}[{idx}]"))
    return tuple(out)


def _require_command_aliases(value: Any, label: str) -> Tuple[Tuple[str, ...], ...]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: list[Tuple[str, ...]] = []
    seen: set[Tuple[str, ...]] = set()
    for idx, row in enumerate(value):
        tokens = _require_command_tokens(row, f"{label}[{idx}]")
        if tokens in seen:
            raise ValueError(f"{label} must not contain duplicate aliases")
        seen.add(tokens)
        out.append(tokens)
    return tuple(out)


def _require_exact_members(
    value: Tuple[str, ...],
    expected: Tuple[str, ...],
    label: str,
) -> Tuple[str, ...]:
    if set(value) != set(expected):
        raise ValueError(
            f"{label} must contain exactly: {', '.join(expected)}"
        )
    return value


def _require_epoch(value: Any, label: str) -> str:
    epoch = _require_non_empty_string(value, label)
    if _EPOCH_RE.fullmatch(epoch) is None:
        raise ValueError(f"{label} must use YYYY-MM with zero-padded month")
    return epoch


def _require_positive_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{label} must be an integer")
    if value < 1:
        raise ValueError(f"{label} must be >= 1")
    return value


def _epoch_to_month_index(epoch: str) -> int:
    year_str, month_str = epoch.split("-", 1)
    return int(year_str) * 12 + int(month_str)


def _validate_schema_lifecycle_epoch_discipline(
    active_epoch: str,
    kind_families: Dict[str, Dict[str, Any]],
) -> Dict[str, Any]:
    alias_support_epochs = []
    for family in kind_families.values():
        aliases = family.get("compatibilityAliases", {})
        if not isinstance(aliases, dict):
            continue
        for alias_row in aliases.values():
            if not isinstance(alias_row, dict):
                continue
            support_until_epoch = alias_row.get("supportUntilEpoch")
            if isinstance(support_until_epoch, str) and support_until_epoch:
                alias_support_epochs.append(support_until_epoch)

    if not alias_support_epochs:
        return {
            "rolloverEpoch": None,
            "aliasRunwayMonths": 0,
            "maxAliasRunwayMonths": _MAX_ALIAS_RUNWAY_MONTHS,
        }

    unique_support_epochs = sorted(set(alias_support_epochs))
    if len(unique_support_epochs) != 1:
        raise ValueError(
            "schemaLifecycle rollover policy requires one shared supportUntilEpoch "
            f"across all compatibility aliases (got {unique_support_epochs})"
        )

    rollover_epoch = unique_support_epochs[0]
    runway_months = _epoch_to_month_index(rollover_epoch) - _epoch_to_month_index(
        active_epoch
    )
    if runway_months < 1:
        raise ValueError(
            "schemaLifecycle rollover policy requires supportUntilEpoch to be after "
            f"activeEpoch (activeEpoch={active_epoch!r}, rolloverEpoch={rollover_epoch!r})"
        )
    if runway_months > _MAX_ALIAS_RUNWAY_MONTHS:
        raise ValueError(
            "schemaLifecycle rollover policy exceeds max runway "
            f"({_MAX_ALIAS_RUNWAY_MONTHS} months): "
            f"activeEpoch={active_epoch!r}, rolloverEpoch={rollover_epoch!r}"
        )

    return {
        "rolloverEpoch": rollover_epoch,
        "aliasRunwayMonths": runway_months,
        "maxAliasRunwayMonths": _MAX_ALIAS_RUNWAY_MONTHS,
    }


def _validate_schema_lifecycle_governance(
    governance: Dict[str, Any],
    *,
    schema_epoch_discipline: Dict[str, Any],
) -> Dict[str, Any]:
    mode = _require_non_empty_string(governance.get("mode"), "schemaLifecycle.governance.mode")
    if mode not in _SCHEMA_LIFECYCLE_GOVERNANCE_MODES:
        raise ValueError(
            "schemaLifecycle.governance.mode must be one of: "
            + ", ".join(_SCHEMA_LIFECYCLE_GOVERNANCE_MODES)
        )
    decision_ref = _require_non_empty_string(
        governance.get("decisionRef"),
        "schemaLifecycle.governance.decisionRef",
    )
    owner = _require_non_empty_string(
        governance.get("owner"),
        "schemaLifecycle.governance.owner",
    )

    rollover_cadence_raw = governance.get("rolloverCadenceMonths")
    freeze_reason_raw = governance.get("freezeReason")

    rollover_epoch = schema_epoch_discipline.get("rolloverEpoch")
    alias_runway_months = int(schema_epoch_discipline.get("aliasRunwayMonths", 0))

    if mode == "rollover":
        rollover_cadence_months = _require_positive_int(
            rollover_cadence_raw,
            "schemaLifecycle.governance.rolloverCadenceMonths",
        )
        if rollover_cadence_months > _MAX_ALIAS_RUNWAY_MONTHS:
            raise ValueError(
                "schemaLifecycle.governance.rolloverCadenceMonths must be <= "
                f"{_MAX_ALIAS_RUNWAY_MONTHS}"
            )
        if rollover_epoch is None:
            raise ValueError(
                "schemaLifecycle.governance.mode=rollover requires at least one "
                "compatibility alias with supportUntilEpoch"
            )
        if alias_runway_months > rollover_cadence_months:
            raise ValueError(
                "schemaLifecycle.governance.rolloverCadenceMonths must be >= alias runway "
                f"(runway={alias_runway_months}, cadence={rollover_cadence_months})"
            )
        if freeze_reason_raw is not None:
            raise ValueError(
                "schemaLifecycle.governance.freezeReason is only allowed when mode=freeze"
            )
        return {
            "mode": mode,
            "decisionRef": decision_ref,
            "owner": owner,
            "rolloverCadenceMonths": rollover_cadence_months,
            "freezeReason": None,
        }

    if rollover_cadence_raw is not None:
        raise ValueError(
            "schemaLifecycle.governance.rolloverCadenceMonths is only allowed when mode=rollover"
        )
    freeze_reason = _require_non_empty_string(
        freeze_reason_raw,
        "schemaLifecycle.governance.freezeReason",
    )
    if rollover_epoch is not None or alias_runway_months != 0:
        raise ValueError(
            "schemaLifecycle.governance.mode=freeze requires no active compatibility aliases"
        )
    return {
        "mode": mode,
        "decisionRef": decision_ref,
        "owner": owner,
        "rolloverCadenceMonths": None,
        "freezeReason": freeze_reason,
    }


def _require_schema_kind_family(value: Any, label: str) -> Dict[str, Any]:
    family = _require_object(value, label)
    canonical_kind = _require_non_empty_string(
        family.get("canonicalKind"), f"{label}.canonicalKind"
    )
    aliases_raw = family.get("compatibilityAliases", [])
    if not isinstance(aliases_raw, list):
        raise ValueError(f"{label}.compatibilityAliases must be a list")
    aliases: Dict[str, Dict[str, str]] = {}
    for idx, alias_row_raw in enumerate(aliases_raw):
        alias_row = _require_object(
            alias_row_raw, f"{label}.compatibilityAliases[{idx}]"
        )
        alias_kind = _require_non_empty_string(
            alias_row.get("aliasKind"),
            f"{label}.compatibilityAliases[{idx}].aliasKind",
        )
        support_until_epoch = _require_epoch(
            alias_row.get("supportUntilEpoch"),
            f"{label}.compatibilityAliases[{idx}].supportUntilEpoch",
        )
        replacement_kind = _require_non_empty_string(
            alias_row.get("replacementKind"),
            f"{label}.compatibilityAliases[{idx}].replacementKind",
        )
        if alias_kind == canonical_kind:
            raise ValueError(
                f"{label}.compatibilityAliases[{idx}].aliasKind must differ from canonicalKind"
            )
        if replacement_kind != canonical_kind:
            raise ValueError(
                f"{label}.compatibilityAliases[{idx}].replacementKind must match canonicalKind"
            )
        if alias_kind in aliases:
            raise ValueError(f"{label}.compatibilityAliases aliasKind values must be unique")
        aliases[alias_kind] = {
            "supportUntilEpoch": support_until_epoch,
            "replacementKind": replacement_kind,
        }
    return {
        "canonicalKind": canonical_kind,
        "compatibilityAliases": aliases,
    }


def _resolve_kind_in_family(
    family_id: str,
    *,
    family: Dict[str, Any],
    kind: str,
    active_epoch: str,
    label: str,
) -> str:
    canonical_kind = family["canonicalKind"]
    if kind == canonical_kind:
        return canonical_kind
    aliases = family.get("compatibilityAliases", {})
    alias_row = aliases.get(kind)
    if alias_row is None:
        raise ValueError(
            f"{label} kind {kind!r} is not supported for schemaLifecycle.kindFamilies.{family_id} "
            f"(canonicalKind={canonical_kind!r})"
        )
    support_until_epoch = alias_row["supportUntilEpoch"]
    if active_epoch > support_until_epoch:
        raise ValueError(
            f"{label} kind {kind!r} expired at supportUntilEpoch={support_until_epoch!r} "
            f"for schemaLifecycle.kindFamilies.{family_id} (activeEpoch={active_epoch!r}, "
            f"canonicalKind={canonical_kind!r})"
        )
    return canonical_kind


def _validate_stage1_parity_contract(payload: Any) -> Dict[str, Any]:
    stage1 = _require_object(payload, "evidenceStage1Parity")
    profile_kind = _require_non_empty_string(
        stage1.get("profileKind"),
        "evidenceStage1Parity.profileKind",
    )
    authority_to_typed_core_route = _require_non_empty_string(
        stage1.get("authorityToTypedCoreRoute"),
        "evidenceStage1Parity.authorityToTypedCoreRoute",
    )
    comparison_tuple = _require_object(
        stage1.get("comparisonTuple"),
        "evidenceStage1Parity.comparisonTuple",
    )
    authority_digest_ref = _require_non_empty_string(
        comparison_tuple.get("authorityDigestRef"),
        "evidenceStage1Parity.comparisonTuple.authorityDigestRef",
    )
    typed_core_digest_ref = _require_non_empty_string(
        comparison_tuple.get("typedCoreDigestRef"),
        "evidenceStage1Parity.comparisonTuple.typedCoreDigestRef",
    )
    normalizer_id_ref = _require_non_empty_string(
        comparison_tuple.get("normalizerIdRef"),
        "evidenceStage1Parity.comparisonTuple.normalizerIdRef",
    )
    policy_digest_ref = _require_non_empty_string(
        comparison_tuple.get("policyDigestRef"),
        "evidenceStage1Parity.comparisonTuple.policyDigestRef",
    )
    if normalizer_id_ref != "normalizerId":
        raise ValueError(
            "evidenceStage1Parity.comparisonTuple.normalizerIdRef must be `normalizerId`"
        )
    if policy_digest_ref != "policyDigest":
        raise ValueError(
            "evidenceStage1Parity.comparisonTuple.policyDigestRef must be `policyDigest`"
        )

    failure_classes = _require_object(
        stage1.get("failureClasses"),
        "evidenceStage1Parity.failureClasses",
    )
    parsed_failure_classes = (
        _require_non_empty_string(
            failure_classes.get("missing"),
            "evidenceStage1Parity.failureClasses.missing",
        ),
        _require_non_empty_string(
            failure_classes.get("mismatch"),
            "evidenceStage1Parity.failureClasses.mismatch",
        ),
        _require_non_empty_string(
            failure_classes.get("unbound"),
            "evidenceStage1Parity.failureClasses.unbound",
        ),
    )
    if parsed_failure_classes != _STAGE1_PARITY_FAILURE_CLASSES:
        raise ValueError(
            "evidenceStage1Parity.failureClasses must map to canonical Stage 1 parity classes"
        )

    return {
        "profileKind": profile_kind,
        "authorityToTypedCoreRoute": authority_to_typed_core_route,
        "comparisonTuple": {
            "authorityDigestRef": authority_digest_ref,
            "typedCoreDigestRef": typed_core_digest_ref,
            "normalizerIdRef": normalizer_id_ref,
            "policyDigestRef": policy_digest_ref,
        },
        "failureClasses": {
            "missing": parsed_failure_classes[0],
            "mismatch": parsed_failure_classes[1],
            "unbound": parsed_failure_classes[2],
        },
    }


def _validate_stage1_rollback_contract(payload: Any) -> Dict[str, Any]:
    rollback = _require_object(payload, "evidenceStage1Rollback")
    profile_kind = _require_non_empty_string(
        rollback.get("profileKind"),
        "evidenceStage1Rollback.profileKind",
    )
    witness_kind = _require_non_empty_string(
        rollback.get("witnessKind"),
        "evidenceStage1Rollback.witnessKind",
    )
    from_stage = _require_non_empty_string(
        rollback.get("fromStage"),
        "evidenceStage1Rollback.fromStage",
    )
    to_stage = _require_non_empty_string(
        rollback.get("toStage"),
        "evidenceStage1Rollback.toStage",
    )
    if from_stage != "stage1":
        raise ValueError("evidenceStage1Rollback.fromStage must be `stage1`")
    if to_stage != "stage0":
        raise ValueError("evidenceStage1Rollback.toStage must be `stage0`")

    trigger_failure_classes = _require_string_list(
        rollback.get("triggerFailureClasses"),
        "evidenceStage1Rollback.triggerFailureClasses",
    )
    if not set(_STAGE1_PARITY_FAILURE_CLASSES).issubset(set(trigger_failure_classes)):
        raise ValueError(
            "evidenceStage1Rollback.triggerFailureClasses must include canonical Stage 1 parity classes"
        )

    identity_refs = _require_object(
        rollback.get("identityRefs"),
        "evidenceStage1Rollback.identityRefs",
    )
    authority_digest_ref = _require_non_empty_string(
        identity_refs.get("authorityDigestRef"),
        "evidenceStage1Rollback.identityRefs.authorityDigestRef",
    )
    rollback_authority_digest_ref = _require_non_empty_string(
        identity_refs.get("rollbackAuthorityDigestRef"),
        "evidenceStage1Rollback.identityRefs.rollbackAuthorityDigestRef",
    )
    normalizer_id_ref = _require_non_empty_string(
        identity_refs.get("normalizerIdRef"),
        "evidenceStage1Rollback.identityRefs.normalizerIdRef",
    )
    policy_digest_ref = _require_non_empty_string(
        identity_refs.get("policyDigestRef"),
        "evidenceStage1Rollback.identityRefs.policyDigestRef",
    )
    if authority_digest_ref == rollback_authority_digest_ref:
        raise ValueError(
            "evidenceStage1Rollback.identityRefs authority/rollback refs must differ"
        )
    if normalizer_id_ref != "normalizerId":
        raise ValueError(
            "evidenceStage1Rollback.identityRefs.normalizerIdRef must be `normalizerId`"
        )
    if policy_digest_ref != "policyDigest":
        raise ValueError(
            "evidenceStage1Rollback.identityRefs.policyDigestRef must be `policyDigest`"
        )

    failure_classes = _require_object(
        rollback.get("failureClasses"),
        "evidenceStage1Rollback.failureClasses",
    )
    parsed_failure_classes = (
        _require_non_empty_string(
            failure_classes.get("precondition"),
            "evidenceStage1Rollback.failureClasses.precondition",
        ),
        _require_non_empty_string(
            failure_classes.get("identityDrift"),
            "evidenceStage1Rollback.failureClasses.identityDrift",
        ),
        _require_non_empty_string(
            failure_classes.get("unbound"),
            "evidenceStage1Rollback.failureClasses.unbound",
        ),
    )
    if parsed_failure_classes != _STAGE1_ROLLBACK_FAILURE_CLASSES:
        raise ValueError(
            "evidenceStage1Rollback.failureClasses must map to canonical Stage 1 rollback classes"
        )

    return {
        "profileKind": profile_kind,
        "witnessKind": witness_kind,
        "fromStage": from_stage,
        "toStage": to_stage,
        "triggerFailureClasses": trigger_failure_classes,
        "identityRefs": {
            "authorityDigestRef": authority_digest_ref,
            "rollbackAuthorityDigestRef": rollback_authority_digest_ref,
            "normalizerIdRef": normalizer_id_ref,
            "policyDigestRef": policy_digest_ref,
        },
        "failureClasses": {
            "precondition": parsed_failure_classes[0],
            "identityDrift": parsed_failure_classes[1],
            "unbound": parsed_failure_classes[2],
        },
    }


def _validate_stage2_authority_contract(
    payload: Any,
    *,
    active_epoch: str,
    schema_epoch_discipline: Dict[str, Any],
) -> Dict[str, Any]:
    stage2 = _require_object(payload, "evidenceStage2Authority")
    profile_kind = _require_non_empty_string(
        stage2.get("profileKind"),
        "evidenceStage2Authority.profileKind",
    )
    active_stage = _require_non_empty_string(
        stage2.get("activeStage"),
        "evidenceStage2Authority.activeStage",
    )
    if active_stage != "stage2":
        raise ValueError("evidenceStage2Authority.activeStage must be `stage2`")

    typed_authority = _require_object(
        stage2.get("typedAuthority"),
        "evidenceStage2Authority.typedAuthority",
    )
    typed_kind_ref = _require_non_empty_string(
        typed_authority.get("kindRef"),
        "evidenceStage2Authority.typedAuthority.kindRef",
    )
    typed_digest_ref = _require_non_empty_string(
        typed_authority.get("digestRef"),
        "evidenceStage2Authority.typedAuthority.digestRef",
    )
    typed_normalizer_id_ref = _require_non_empty_string(
        typed_authority.get("normalizerIdRef"),
        "evidenceStage2Authority.typedAuthority.normalizerIdRef",
    )
    typed_policy_digest_ref = _require_non_empty_string(
        typed_authority.get("policyDigestRef"),
        "evidenceStage2Authority.typedAuthority.policyDigestRef",
    )
    if typed_normalizer_id_ref != "normalizerId":
        raise ValueError(
            "evidenceStage2Authority.typedAuthority.normalizerIdRef must be `normalizerId`"
        )
    if typed_policy_digest_ref != "policyDigest":
        raise ValueError(
            "evidenceStage2Authority.typedAuthority.policyDigestRef must be `policyDigest`"
        )

    compatibility_alias = _require_object(
        stage2.get("compatibilityAlias"),
        "evidenceStage2Authority.compatibilityAlias",
    )
    alias_kind_ref = _require_non_empty_string(
        compatibility_alias.get("kindRef"),
        "evidenceStage2Authority.compatibilityAlias.kindRef",
    )
    alias_digest_ref = _require_non_empty_string(
        compatibility_alias.get("digestRef"),
        "evidenceStage2Authority.compatibilityAlias.digestRef",
    )
    alias_role = _require_non_empty_string(
        compatibility_alias.get("role"),
        "evidenceStage2Authority.compatibilityAlias.role",
    )
    if alias_role != _STAGE2_COMPATIBILITY_ALIAS_ROLE:
        raise ValueError(
            "evidenceStage2Authority.compatibilityAlias.role must be "
            f"`{_STAGE2_COMPATIBILITY_ALIAS_ROLE}`"
        )
    alias_support_until_epoch = _require_epoch(
        compatibility_alias.get("supportUntilEpoch"),
        "evidenceStage2Authority.compatibilityAlias.supportUntilEpoch",
    )
    if typed_digest_ref == alias_digest_ref:
        raise ValueError(
            "evidenceStage2Authority typed/alias digest refs must differ"
        )

    rollover_epoch = schema_epoch_discipline.get("rolloverEpoch")
    if not isinstance(rollover_epoch, str) or not rollover_epoch:
        raise ValueError(
            "evidenceStage2Authority requires schemaLifecycle.epochDiscipline.rolloverEpoch"
        )
    if alias_support_until_epoch != rollover_epoch:
        raise ValueError(
            "evidenceStage2Authority.compatibilityAlias.supportUntilEpoch must match "
            "schemaLifecycle.epochDiscipline.rolloverEpoch"
        )
    if active_epoch > alias_support_until_epoch:
        raise ValueError(
            "evidenceStage2Authority compatibility alias expired at "
            f"supportUntilEpoch={alias_support_until_epoch!r} (activeEpoch={active_epoch!r})"
        )

    bidir_evidence_route = _require_object(
        stage2.get("bidirEvidenceRoute"),
        "evidenceStage2Authority.bidirEvidenceRoute",
    )
    bidir_route_kind = _require_non_empty_string(
        bidir_evidence_route.get("routeKind"),
        "evidenceStage2Authority.bidirEvidenceRoute.routeKind",
    )
    if bidir_route_kind != _STAGE2_BIDIR_EVIDENCE_ROUTE_KIND:
        raise ValueError(
            "evidenceStage2Authority.bidirEvidenceRoute.routeKind must be "
            f"`{_STAGE2_BIDIR_EVIDENCE_ROUTE_KIND}`"
        )
    obligation_field_ref = _require_non_empty_string(
        bidir_evidence_route.get("obligationFieldRef"),
        "evidenceStage2Authority.bidirEvidenceRoute.obligationFieldRef",
    )
    if obligation_field_ref != _STAGE2_BIDIR_EVIDENCE_OBLIGATION_FIELD_REF:
        raise ValueError(
            "evidenceStage2Authority.bidirEvidenceRoute.obligationFieldRef must be "
            f"`{_STAGE2_BIDIR_EVIDENCE_OBLIGATION_FIELD_REF}`"
        )
    required_obligations = _require_string_list(
        bidir_evidence_route.get("requiredObligations"),
        "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations",
    )
    if set(required_obligations) != set(_STAGE2_REQUIRED_KERNEL_OBLIGATIONS):
        raise ValueError(
            "evidenceStage2Authority.bidirEvidenceRoute.requiredObligations must match canonical Stage 2 kernel obligations"
        )
    bidir_route_failure_classes = _require_object(
        bidir_evidence_route.get("failureClasses"),
        "evidenceStage2Authority.bidirEvidenceRoute.failureClasses",
    )
    parsed_bidir_route_failure_classes = (
        _require_non_empty_string(
            bidir_route_failure_classes.get("missing"),
            "evidenceStage2Authority.bidirEvidenceRoute.failureClasses.missing",
        ),
        _require_non_empty_string(
            bidir_route_failure_classes.get("drift"),
            "evidenceStage2Authority.bidirEvidenceRoute.failureClasses.drift",
        ),
    )
    if parsed_bidir_route_failure_classes != _STAGE2_KERNEL_COMPLIANCE_FAILURE_CLASSES:
        raise ValueError(
            "evidenceStage2Authority.bidirEvidenceRoute.failureClasses must map to canonical Stage 2 kernel-compliance classes"
        )
    fallback_raw = bidir_evidence_route.get("fallback")
    fallback_mode: Optional[str] = None
    fallback_profile_kinds: Tuple[str, ...] = tuple()
    if fallback_raw is not None:
        fallback = _require_object(
            fallback_raw, "evidenceStage2Authority.bidirEvidenceRoute.fallback"
        )
        fallback_mode = _require_non_empty_string(
            fallback.get("mode"),
            "evidenceStage2Authority.bidirEvidenceRoute.fallback.mode",
        )
        if fallback_mode != _STAGE2_BIDIR_EVIDENCE_FALLBACK_MODE:
            raise ValueError(
                "evidenceStage2Authority.bidirEvidenceRoute.fallback.mode must be "
                f"`{_STAGE2_BIDIR_EVIDENCE_FALLBACK_MODE}`"
            )
        profile_kinds_raw = fallback.get("profileKinds")
        if profile_kinds_raw is None:
            fallback_profile_kinds = tuple()
        elif isinstance(profile_kinds_raw, list):
            fallback_profile_kinds = tuple(
                _require_non_empty_string(
                    item,
                    f"evidenceStage2Authority.bidirEvidenceRoute.fallback.profileKinds[{idx}]",
                )
                for idx, item in enumerate(profile_kinds_raw)
            )
            if len(set(fallback_profile_kinds)) != len(fallback_profile_kinds):
                raise ValueError(
                    "evidenceStage2Authority.bidirEvidenceRoute.fallback.profileKinds must not contain duplicates"
                )
        else:
            raise ValueError(
                "evidenceStage2Authority.bidirEvidenceRoute.fallback.profileKinds must be a list"
            )

    kernel_compliance_sentinel_raw = stage2.get("kernelComplianceSentinel")
    parsed_kernel_sentinel: Optional[Dict[str, Any]] = None
    if kernel_compliance_sentinel_raw is not None:
        kernel_compliance_sentinel = _require_object(
            kernel_compliance_sentinel_raw,
            "evidenceStage2Authority.kernelComplianceSentinel",
        )
        sentinel_required_obligations = _require_string_list(
            kernel_compliance_sentinel.get("requiredObligations"),
            "evidenceStage2Authority.kernelComplianceSentinel.requiredObligations",
        )
        if set(sentinel_required_obligations) != set(required_obligations):
            raise ValueError(
                "evidenceStage2Authority.kernelComplianceSentinel.requiredObligations must match evidenceStage2Authority.bidirEvidenceRoute.requiredObligations"
            )
        sentinel_failure_classes = _require_object(
            kernel_compliance_sentinel.get("failureClasses"),
            "evidenceStage2Authority.kernelComplianceSentinel.failureClasses",
        )
        parsed_sentinel_failure_classes = (
            _require_non_empty_string(
                sentinel_failure_classes.get("missing"),
                "evidenceStage2Authority.kernelComplianceSentinel.failureClasses.missing",
            ),
            _require_non_empty_string(
                sentinel_failure_classes.get("drift"),
                "evidenceStage2Authority.kernelComplianceSentinel.failureClasses.drift",
            ),
        )
        if parsed_sentinel_failure_classes != parsed_bidir_route_failure_classes:
            raise ValueError(
                "evidenceStage2Authority.kernelComplianceSentinel.failureClasses must match evidenceStage2Authority.bidirEvidenceRoute.failureClasses"
            )
        if (
            fallback_mode != _STAGE2_BIDIR_EVIDENCE_FALLBACK_MODE
            or profile_kind not in fallback_profile_kinds
        ):
            raise ValueError(
                "evidenceStage2Authority.kernelComplianceSentinel requires bidirEvidenceRoute.fallback.mode=`profile_gated_sentinel` with current profileKind included in fallback.profileKinds"
            )
        parsed_kernel_sentinel = {
            "requiredObligations": sentinel_required_obligations,
            "failureClasses": {
                "missing": parsed_sentinel_failure_classes[0],
                "drift": parsed_sentinel_failure_classes[1],
            },
        }

    failure_classes = _require_object(
        stage2.get("failureClasses"),
        "evidenceStage2Authority.failureClasses",
    )
    parsed_failure_classes = (
        _require_non_empty_string(
            failure_classes.get("authorityAliasViolation"),
            "evidenceStage2Authority.failureClasses.authorityAliasViolation",
        ),
        _require_non_empty_string(
            failure_classes.get("aliasWindowViolation"),
            "evidenceStage2Authority.failureClasses.aliasWindowViolation",
        ),
        _require_non_empty_string(
            failure_classes.get("unbound"),
            "evidenceStage2Authority.failureClasses.unbound",
        ),
    )
    if parsed_failure_classes != _STAGE2_AUTHORITY_FAILURE_CLASSES:
        raise ValueError(
            "evidenceStage2Authority.failureClasses must map to canonical Stage 2 classes"
        )

    return {
        "profileKind": profile_kind,
        "activeStage": active_stage,
        "typedAuthority": {
            "kindRef": typed_kind_ref,
            "digestRef": typed_digest_ref,
            "normalizerIdRef": typed_normalizer_id_ref,
            "policyDigestRef": typed_policy_digest_ref,
        },
        "compatibilityAlias": {
            "kindRef": alias_kind_ref,
            "digestRef": alias_digest_ref,
            "role": alias_role,
            "supportUntilEpoch": alias_support_until_epoch,
        },
        "bidirEvidenceRoute": {
            "routeKind": bidir_route_kind,
            "obligationFieldRef": obligation_field_ref,
            "requiredObligations": required_obligations,
            "failureClasses": {
                "missing": parsed_bidir_route_failure_classes[0],
                "drift": parsed_bidir_route_failure_classes[1],
            },
            "fallback": {
                "mode": fallback_mode,
                "profileKinds": fallback_profile_kinds,
            }
            if fallback_mode is not None
            else None,
        },
        "failureClasses": {
            "authorityAliasViolation": parsed_failure_classes[0],
            "aliasWindowViolation": parsed_failure_classes[1],
            "unbound": parsed_failure_classes[2],
        },
        "kernelComplianceSentinel": parsed_kernel_sentinel,
    }


def _validate_worker_lane_authority_contract(
    payload: Any,
    *,
    active_epoch: str,
) -> Dict[str, Any]:
    worker_lane = _require_object(payload, "workerLaneAuthority")
    mutation_policy = _require_object(
        worker_lane.get("mutationPolicy"),
        "workerLaneAuthority.mutationPolicy",
    )
    default_mode = _require_non_empty_string(
        mutation_policy.get("defaultMode"),
        "workerLaneAuthority.mutationPolicy.defaultMode",
    )
    allowed_modes = _require_string_list(
        mutation_policy.get("allowedModes"),
        "workerLaneAuthority.mutationPolicy.allowedModes",
    )
    allowed_mode_set = set(allowed_modes)
    required_mode_set = set(_WORKER_ALLOWED_MUTATION_MODES)
    if default_mode != _WORKER_DEFAULT_MUTATION_MODE:
        raise ValueError(
            "workerLaneAuthority.mutationPolicy.defaultMode must be `instruction-linked`"
        )
    if default_mode not in allowed_mode_set:
        raise ValueError(
            "workerLaneAuthority.mutationPolicy.allowedModes must include defaultMode"
        )
    if allowed_mode_set != required_mode_set:
        raise ValueError(
            "workerLaneAuthority.mutationPolicy.allowedModes must match canonical modes: "
            + ", ".join(_WORKER_ALLOWED_MUTATION_MODES)
        )

    overrides_raw = mutation_policy.get("compatibilityOverrides", [])
    if not isinstance(overrides_raw, list):
        raise ValueError(
            "workerLaneAuthority.mutationPolicy.compatibilityOverrides must be a list"
        )
    override_rows: Dict[str, Dict[str, Any]] = {}
    for idx, override_raw in enumerate(overrides_raw):
        override = _require_object(
            override_raw,
            f"workerLaneAuthority.mutationPolicy.compatibilityOverrides[{idx}]",
        )
        mode = _require_non_empty_string(
            override.get("mode"),
            f"workerLaneAuthority.mutationPolicy.compatibilityOverrides[{idx}].mode",
        )
        support_until_epoch = _require_epoch(
            override.get("supportUntilEpoch"),
            f"workerLaneAuthority.mutationPolicy.compatibilityOverrides[{idx}].supportUntilEpoch",
        )
        requires_reason = override.get("requiresReason")
        if not isinstance(requires_reason, bool):
            raise ValueError(
                "workerLaneAuthority.mutationPolicy.compatibilityOverrides"
                f"[{idx}].requiresReason must be a boolean"
            )
        if mode == default_mode:
            raise ValueError(
                "workerLaneAuthority.mutationPolicy.compatibilityOverrides mode must differ from defaultMode"
            )
        if mode not in allowed_mode_set:
            raise ValueError(
                "workerLaneAuthority.mutationPolicy.compatibilityOverrides mode must be listed in allowedModes"
            )
        if active_epoch > support_until_epoch:
            raise ValueError(
                "workerLaneAuthority.mutationPolicy.compatibilityOverrides"
                f"[{idx}] expired at supportUntilEpoch={support_until_epoch!r}"
                f" (activeEpoch={active_epoch!r})"
            )
        if mode in override_rows:
            raise ValueError(
                "workerLaneAuthority.mutationPolicy.compatibilityOverrides mode values must be unique"
            )
        override_rows[mode] = {
            "supportUntilEpoch": support_until_epoch,
            "requiresReason": requires_reason,
        }

    expected_override_modes = required_mode_set - {default_mode}
    if set(override_rows) != expected_override_modes:
        raise ValueError(
            "workerLaneAuthority.mutationPolicy.compatibilityOverrides must define exactly one active override per non-default allowed mode"
        )

    mutation_routes = _require_object(
        worker_lane.get("mutationRoutes"),
        "workerLaneAuthority.mutationRoutes",
    )
    parsed_routes: Dict[str, str] = {}
    for key, expected in _WORKER_MUTATION_ROUTE_BINDINGS.items():
        value = _require_non_empty_string(
            mutation_routes.get(key),
            f"workerLaneAuthority.mutationRoutes.{key}",
        )
        if value != expected:
            raise ValueError(
                "workerLaneAuthority.mutationRoutes."
                f"{key} must resolve to canonical route {expected!r}"
            )
        parsed_routes[key] = value
    unknown_route_keys = sorted(set(mutation_routes) - set(_WORKER_MUTATION_ROUTE_BINDINGS))
    if unknown_route_keys:
        raise ValueError(
            "workerLaneAuthority.mutationRoutes includes unknown route keys: "
            + ", ".join(unknown_route_keys)
        )

    failure_classes = _require_object(
        worker_lane.get("failureClasses"),
        "workerLaneAuthority.failureClasses",
    )
    parsed_failure_classes = (
        _require_non_empty_string(
            failure_classes.get("policyDrift"),
            "workerLaneAuthority.failureClasses.policyDrift",
        ),
        _require_non_empty_string(
            failure_classes.get("mutationModeDrift"),
            "workerLaneAuthority.failureClasses.mutationModeDrift",
        ),
        _require_non_empty_string(
            failure_classes.get("routeUnbound"),
            "workerLaneAuthority.failureClasses.routeUnbound",
        ),
    )
    if parsed_failure_classes != _WORKER_FAILURE_CLASSES:
        raise ValueError(
            "workerLaneAuthority.failureClasses must map to canonical worker-lane classes"
        )

    return {
        "mutationPolicy": {
            "defaultMode": default_mode,
            "allowedModes": allowed_modes,
            "compatibilityOverrides": [
                {
                    "mode": mode,
                    "supportUntilEpoch": override_rows[mode]["supportUntilEpoch"],
                    "requiresReason": override_rows[mode]["requiresReason"],
                }
                for mode in sorted(override_rows)
            ],
        },
        "mutationRoutes": parsed_routes,
        "failureClasses": {
            "policyDrift": parsed_failure_classes[0],
            "mutationModeDrift": parsed_failure_classes[1],
            "routeUnbound": parsed_failure_classes[2],
        },
    }


def _validate_runtime_route_bindings(
    payload: Any, *, doctrine_operation_ids: frozenset[str]
) -> Dict[str, Any]:
    runtime_routes = _require_object(payload, "runtimeRouteBindings")
    required_routes = _require_object(
        runtime_routes.get("requiredOperationRoutes"),
        "runtimeRouteBindings.requiredOperationRoutes",
    )
    if not required_routes:
        raise ValueError(
            "runtimeRouteBindings.requiredOperationRoutes must be a non-empty object"
        )
    parsed_routes: Dict[str, Dict[str, Any]] = {}
    for key in sorted(required_routes):
        key_norm = _require_non_empty_string(
            key, "runtimeRouteBindings.requiredOperationRoutes.<routeId>"
        )
        route_obj = _require_object(
            required_routes.get(key),
            f"runtimeRouteBindings.requiredOperationRoutes.{key_norm}",
        )
        operation_id = _require_non_empty_string(
            route_obj.get("operationId"),
            f"runtimeRouteBindings.requiredOperationRoutes.{key_norm}.operationId",
        )
        if operation_id not in doctrine_operation_ids:
            raise ValueError(
                "runtimeRouteBindings.requiredOperationRoutes."
                f"{key_norm}.operationId must exist in doctrine op registry: {operation_id!r}"
            )
        required_morphisms = tuple(
            sorted(
                _require_string_list(
                    route_obj.get("requiredMorphisms"),
                    f"runtimeRouteBindings.requiredOperationRoutes.{key_norm}.requiredMorphisms",
                )
            )
        )
        parsed_routes[key_norm] = {
            "operationId": operation_id,
            "requiredMorphisms": required_morphisms,
        }

    failure_classes = _require_object(
        runtime_routes.get("failureClasses"),
        "runtimeRouteBindings.failureClasses",
    )
    missing_failure_class_keys = sorted(
        set(_REQUIRED_RUNTIME_ROUTE_FAILURE_CLASS_KEYS) - set(failure_classes)
    )
    if missing_failure_class_keys:
        raise ValueError(
            "runtimeRouteBindings.failureClasses missing required keys: "
            + ", ".join(missing_failure_class_keys)
        )
    unknown_failure_class_keys = sorted(
        set(failure_classes) - set(_REQUIRED_RUNTIME_ROUTE_FAILURE_CLASS_KEYS)
    )
    if unknown_failure_class_keys:
        raise ValueError(
            "runtimeRouteBindings.failureClasses includes unknown keys: "
            + ", ".join(unknown_failure_class_keys)
        )
    parsed_failure_classes = {
        key: _require_non_empty_string(
            failure_classes.get(key),
            f"runtimeRouteBindings.failureClasses.{key}",
        )
        for key in _REQUIRED_RUNTIME_ROUTE_FAILURE_CLASS_KEYS
    }

    return {
        "requiredOperationRoutes": parsed_routes,
        "failureClasses": parsed_failure_classes,
    }


def _validate_world_descent_contract(
    payload: Any, *, doctrine_operation_ids: frozenset[str]
) -> Dict[str, Any]:
    world_descent = _require_object(payload, "worldDescentContract")
    contract_id = _require_non_empty_string(
        world_descent.get("contractId"),
        "worldDescentContract.contractId",
    )
    if contract_id != _WORLD_DESCENT_CONTRACT_ID:
        raise ValueError(
            "worldDescentContract.contractId must resolve to canonical "
            f"{_WORLD_DESCENT_CONTRACT_ID!r}"
        )

    required_route_families = tuple(
        sorted(
            _require_string_list(
                world_descent.get("requiredRouteFamilies"),
                "worldDescentContract.requiredRouteFamilies",
            )
        )
    )
    if set(required_route_families) != set(_WORLD_DESCENT_DEFAULT_REQUIRED_ROUTE_FAMILIES):
        raise ValueError(
            "worldDescentContract.requiredRouteFamilies must match canonical set"
        )

    required_action_bindings = _require_object(
        world_descent.get("requiredActionRouteBindings"),
        "worldDescentContract.requiredActionRouteBindings",
    )
    parsed_action_bindings: Dict[str, Tuple[str, ...]] = {}
    for route_family in sorted(required_action_bindings):
        route_family_id = _require_non_empty_string(
            route_family, "worldDescentContract.requiredActionRouteBindings.<routeFamilyId>"
        )
        if route_family_id not in required_route_families:
            raise ValueError(
                "worldDescentContract.requiredActionRouteBindings."
                f"{route_family_id} must reference requiredRouteFamilies"
            )
        host_actions = tuple(
            sorted(
                _require_string_list(
                    required_action_bindings.get(route_family),
                    "worldDescentContract.requiredActionRouteBindings."
                    f"{route_family_id}",
                )
            )
        )
        for host_action_id in host_actions:
            if not _HOST_ACTION_ID_RE.match(host_action_id):
                raise ValueError(
                    "worldDescentContract.requiredActionRouteBindings."
                    f"{route_family_id} contains invalid host action id: {host_action_id!r}"
                )
        parsed_action_bindings[route_family_id] = host_actions

    expected_action_bindings = {
        route_family: tuple(sorted(host_actions))
        for route_family, host_actions in _WORLD_DESCENT_DEFAULT_REQUIRED_ACTION_ROUTE_BINDINGS.items()
    }
    if set(parsed_action_bindings) != set(expected_action_bindings):
        raise ValueError(
            "worldDescentContract.requiredActionRouteBindings must match canonical route-family keys"
        )
    for route_family_id, expected_actions in expected_action_bindings.items():
        if parsed_action_bindings.get(route_family_id, tuple()) != expected_actions:
            raise ValueError(
                "worldDescentContract.requiredActionRouteBindings."
                f"{route_family_id} must match canonical host-action bindings"
            )

    required_static_bindings = _require_object(
        world_descent.get("requiredStaticOperationBindings"),
        "worldDescentContract.requiredStaticOperationBindings",
    )
    parsed_static_bindings: Dict[str, Tuple[str, ...]] = {}
    for route_family in sorted(required_static_bindings):
        route_family_id = _require_non_empty_string(
            route_family, "worldDescentContract.requiredStaticOperationBindings.<routeFamilyId>"
        )
        if route_family_id not in required_route_families:
            raise ValueError(
                "worldDescentContract.requiredStaticOperationBindings."
                f"{route_family_id} must reference requiredRouteFamilies"
            )
        operation_ids = tuple(
            sorted(
                _require_string_list(
                    required_static_bindings.get(route_family),
                    "worldDescentContract.requiredStaticOperationBindings."
                    f"{route_family_id}",
                )
            )
        )
        for operation_id in operation_ids:
            if not _HOST_ACTION_OPERATION_ID_RE.match(operation_id):
                raise ValueError(
                    "worldDescentContract.requiredStaticOperationBindings."
                    f"{route_family_id} contains invalid operationId: {operation_id!r}"
                )
            if operation_id not in doctrine_operation_ids:
                raise ValueError(
                    "worldDescentContract.requiredStaticOperationBindings."
                    f"{route_family_id} operationId must exist in doctrine op registry: {operation_id!r}"
                )
        parsed_static_bindings[route_family_id] = operation_ids

    expected_static_bindings = {
        route_family: tuple(sorted(operation_ids))
        for route_family, operation_ids in _WORLD_DESCENT_DEFAULT_REQUIRED_STATIC_OPERATION_BINDINGS.items()
    }
    if set(parsed_static_bindings) != set(expected_static_bindings):
        raise ValueError(
            "worldDescentContract.requiredStaticOperationBindings must match canonical route-family keys"
        )
    for route_family_id, expected_operation_ids in expected_static_bindings.items():
        if parsed_static_bindings.get(route_family_id, tuple()) != expected_operation_ids:
            raise ValueError(
                "worldDescentContract.requiredStaticOperationBindings."
                f"{route_family_id} must match canonical operation bindings"
            )

    failure_classes = _require_object(
        world_descent.get("failureClasses"),
        "worldDescentContract.failureClasses",
    )
    missing_failure_class_keys = sorted(
        set(_WORLD_DESCENT_REQUIRED_FAILURE_CLASS_KEYS) - set(failure_classes)
    )
    if missing_failure_class_keys:
        raise ValueError(
            "worldDescentContract.failureClasses missing required keys: "
            + ", ".join(missing_failure_class_keys)
        )
    unknown_failure_class_keys = sorted(
        set(failure_classes) - set(_WORLD_DESCENT_REQUIRED_FAILURE_CLASS_KEYS)
    )
    if unknown_failure_class_keys:
        raise ValueError(
            "worldDescentContract.failureClasses includes unknown keys: "
            + ", ".join(unknown_failure_class_keys)
        )
    parsed_failure_classes = {
        key: _require_non_empty_string(
            failure_classes.get(key),
            f"worldDescentContract.failureClasses.{key}",
        )
        for key in _WORLD_DESCENT_REQUIRED_FAILURE_CLASS_KEYS
    }
    if parsed_failure_classes != _WORLD_DESCENT_DEFAULT_FAILURE_CLASSES:
        raise ValueError(
            "worldDescentContract.failureClasses must map to canonical world-descent classes"
        )

    return {
        "contractId": contract_id,
        "requiredRouteFamilies": list(required_route_families),
        "requiredActionRouteBindings": {
            route_family: list(parsed_action_bindings[route_family])
            for route_family in sorted(parsed_action_bindings)
        },
        "requiredStaticOperationBindings": {
            route_family: list(parsed_static_bindings[route_family])
            for route_family in sorted(parsed_static_bindings)
        },
        "failureClasses": parsed_failure_classes,
    }


def _validate_command_surface(payload: Any) -> Dict[str, Any]:
    command_surface = _require_object(payload, "commandSurface")
    missing_surface_ids = sorted(
        set(_REQUIRED_COMMAND_SURFACE_IDS) - set(command_surface)
    )
    if missing_surface_ids:
        raise ValueError(
            "commandSurface missing required surfaces: "
            + ", ".join(missing_surface_ids)
        )
    unknown_keys = sorted(
        set(command_surface) - (set(_REQUIRED_COMMAND_SURFACE_IDS) | {"failureClasses"})
    )
    if unknown_keys:
        raise ValueError(
            "commandSurface includes unknown keys: "
            + ", ".join(unknown_keys)
        )

    parsed_surface: Dict[str, Any] = {}
    for surface_id in _REQUIRED_COMMAND_SURFACE_IDS:
        row = _require_object(
            command_surface.get(surface_id),
            f"commandSurface.{surface_id}",
        )
        canonical_entrypoint = _require_command_tokens(
            row.get("canonicalEntrypoint"),
            f"commandSurface.{surface_id}.canonicalEntrypoint",
        )
        compatibility_aliases = _require_command_aliases(
            row.get("compatibilityAliases"),
            f"commandSurface.{surface_id}.compatibilityAliases",
        )
        if canonical_entrypoint in set(compatibility_aliases):
            raise ValueError(
                "commandSurface."
                f"{surface_id}.compatibilityAliases must not include canonicalEntrypoint"
            )
        parsed_surface[surface_id] = {
            "canonicalEntrypoint": list(canonical_entrypoint),
            "compatibilityAliases": [
                list(alias)
                for alias in sorted(compatibility_aliases)
            ],
        }

    failure_classes = _require_object(
        command_surface.get("failureClasses"),
        "commandSurface.failureClasses",
    )
    missing_failure_class_keys = sorted(
        set(_REQUIRED_COMMAND_SURFACE_FAILURE_CLASS_KEYS) - set(failure_classes)
    )
    if missing_failure_class_keys:
        raise ValueError(
            "commandSurface.failureClasses missing required keys: "
            + ", ".join(missing_failure_class_keys)
        )
    unknown_failure_class_keys = sorted(
        set(failure_classes) - set(_REQUIRED_COMMAND_SURFACE_FAILURE_CLASS_KEYS)
    )
    if unknown_failure_class_keys:
        raise ValueError(
            "commandSurface.failureClasses includes unknown keys: "
            + ", ".join(unknown_failure_class_keys)
        )
    unbound = _require_non_empty_string(
        failure_classes.get("unbound"),
        "commandSurface.failureClasses.unbound",
    )
    parsed_surface["failureClasses"] = {"unbound": unbound}
    return parsed_surface


def _validate_pipeline_wrapper_surface(payload: Any) -> Dict[str, Any]:
    wrapper_surface = _require_object(payload, "pipelineWrapperSurface")
    required_pipeline_entrypoint = _require_command_tokens(
        wrapper_surface.get("requiredPipelineEntrypoint"),
        "pipelineWrapperSurface.requiredPipelineEntrypoint",
    )
    instruction_pipeline_entrypoint = _require_command_tokens(
        wrapper_surface.get("instructionPipelineEntrypoint"),
        "pipelineWrapperSurface.instructionPipelineEntrypoint",
    )

    required_gate_hooks = _require_object(
        wrapper_surface.get("requiredGateHooks"),
        "pipelineWrapperSurface.requiredGateHooks",
    )
    missing_required_gate_hook_keys = sorted(
        set(_REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS) - set(required_gate_hooks)
    )
    if missing_required_gate_hook_keys:
        raise ValueError(
            "pipelineWrapperSurface.requiredGateHooks missing required keys: "
            + ", ".join(missing_required_gate_hook_keys)
        )
    unknown_required_gate_hook_keys = sorted(
        set(required_gate_hooks) - set(_REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS)
    )
    if unknown_required_gate_hook_keys:
        raise ValueError(
            "pipelineWrapperSurface.requiredGateHooks includes unknown keys: "
            + ", ".join(unknown_required_gate_hook_keys)
        )
    parsed_required_gate_hooks = {
        key: _require_non_empty_string(
            required_gate_hooks.get(key),
            f"pipelineWrapperSurface.requiredGateHooks.{key}",
        )
        for key in _REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS
    }

    instruction_gate_hooks = _require_object(
        wrapper_surface.get("instructionGateHooks"),
        "pipelineWrapperSurface.instructionGateHooks",
    )
    missing_instruction_gate_hook_keys = sorted(
        set(_REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS) - set(instruction_gate_hooks)
    )
    if missing_instruction_gate_hook_keys:
        raise ValueError(
            "pipelineWrapperSurface.instructionGateHooks missing required keys: "
            + ", ".join(missing_instruction_gate_hook_keys)
        )
    unknown_instruction_gate_hook_keys = sorted(
        set(instruction_gate_hooks) - set(_REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS)
    )
    if unknown_instruction_gate_hook_keys:
        raise ValueError(
            "pipelineWrapperSurface.instructionGateHooks includes unknown keys: "
            + ", ".join(unknown_instruction_gate_hook_keys)
        )
    parsed_instruction_gate_hooks = {
        key: _require_non_empty_string(
            instruction_gate_hooks.get(key),
            f"pipelineWrapperSurface.instructionGateHooks.{key}",
        )
        for key in _REQUIRED_PIPELINE_WRAPPER_HOOK_KEYS
    }

    failure_classes = _require_object(
        wrapper_surface.get("failureClasses"),
        "pipelineWrapperSurface.failureClasses",
    )
    missing_failure_class_keys = sorted(
        set(_REQUIRED_PIPELINE_WRAPPER_FAILURE_CLASS_KEYS) - set(failure_classes)
    )
    if missing_failure_class_keys:
        raise ValueError(
            "pipelineWrapperSurface.failureClasses missing required keys: "
            + ", ".join(missing_failure_class_keys)
        )
    unknown_failure_class_keys = sorted(
        set(failure_classes) - set(_REQUIRED_PIPELINE_WRAPPER_FAILURE_CLASS_KEYS)
    )
    if unknown_failure_class_keys:
        raise ValueError(
            "pipelineWrapperSurface.failureClasses includes unknown keys: "
            + ", ".join(unknown_failure_class_keys)
        )
    parsed_failure_classes = {
        key: _require_non_empty_string(
            failure_classes.get(key),
            f"pipelineWrapperSurface.failureClasses.{key}",
        )
        for key in _REQUIRED_PIPELINE_WRAPPER_FAILURE_CLASS_KEYS
    }
    if len(set(parsed_failure_classes.values())) != len(parsed_failure_classes):
        raise ValueError(
            "pipelineWrapperSurface.failureClasses must not contain duplicate values"
        )

    return {
        "requiredPipelineEntrypoint": list(required_pipeline_entrypoint),
        "instructionPipelineEntrypoint": list(instruction_pipeline_entrypoint),
        "requiredGateHooks": parsed_required_gate_hooks,
        "instructionGateHooks": parsed_instruction_gate_hooks,
        "failureClasses": parsed_failure_classes,
    }


def _validate_host_action_surface(
    payload: Any, *, doctrine_operation_ids: frozenset[str]
) -> Dict[str, Any]:
    host_action_surface = _require_object(payload, "hostActionSurface")
    unknown_surface_keys = sorted(
        set(host_action_surface) - {"requiredActions", "mcpOnlyHostActions", "failureClasses"}
    )
    if unknown_surface_keys:
        raise ValueError(
            "hostActionSurface includes unknown keys: " + ", ".join(unknown_surface_keys)
        )
    required_actions = _require_object(
        host_action_surface.get("requiredActions"),
        "hostActionSurface.requiredActions",
    )
    if not required_actions:
        raise ValueError("hostActionSurface.requiredActions must be a non-empty object")
    mcp_only_host_actions = _require_string_list(
        host_action_surface.get("mcpOnlyHostActions"),
        "hostActionSurface.mcpOnlyHostActions",
    )

    parsed_actions: Dict[str, Dict[str, Optional[str]]] = {}
    canonical_cli_bindings: Dict[str, str] = {}
    mcp_tool_bindings: Dict[str, str] = {}
    for host_action_id in sorted(required_actions):
        host_action_id_norm = _require_non_empty_string(
            host_action_id,
            "hostActionSurface.requiredActions.<hostActionId>",
        )
        if _HOST_ACTION_ID_RE.fullmatch(host_action_id_norm) is None:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm} must use `<family>.<action>` id shape"
            )
        row = _require_object(
            required_actions.get(host_action_id),
            f"hostActionSurface.requiredActions.{host_action_id_norm}",
        )
        unknown_row_keys = sorted(set(row) - {"canonicalCli", "mcpTool", "operationId"})
        if unknown_row_keys:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm} includes unknown keys: "
                + ", ".join(unknown_row_keys)
            )
        canonical_cli_raw = row.get("canonicalCli")
        canonical_cli = (
            _require_non_empty_string(
                canonical_cli_raw,
                f"hostActionSurface.requiredActions.{host_action_id_norm}.canonicalCli",
            )
            if canonical_cli_raw is not None
            else None
        )
        mcp_tool_raw = row.get("mcpTool")
        mcp_tool = (
            _require_non_empty_string(
                mcp_tool_raw,
                f"hostActionSurface.requiredActions.{host_action_id_norm}.mcpTool",
            )
            if mcp_tool_raw is not None
            else None
        )
        operation_id_raw = row.get("operationId")
        operation_id = (
            _require_non_empty_string(
                operation_id_raw,
                f"hostActionSurface.requiredActions.{host_action_id_norm}.operationId",
            )
            if operation_id_raw is not None
            else None
        )
        if mcp_tool is not None and _HOST_ACTION_MCP_TOOL_RE.fullmatch(mcp_tool) is None:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm}.mcpTool must be a snake_case tool id"
            )
        if operation_id is not None and _HOST_ACTION_OPERATION_ID_RE.fullmatch(operation_id) is None:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm}.operationId must use `op/<family>.<action>` shape"
            )
        if operation_id is not None and operation_id not in doctrine_operation_ids:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm}.operationId must exist in doctrine op registry: {operation_id!r}"
            )
        if canonical_cli is None and mcp_tool is None:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm} must bind at least one of canonicalCli or mcpTool"
            )
        if mcp_tool is not None:
            expected_mcp_operation_id = f"op/mcp.{mcp_tool}"
            if operation_id != expected_mcp_operation_id:
                raise ValueError(
                    "hostActionSurface.requiredActions."
                    f"{host_action_id_norm}.operationId must match mcpTool binding "
                    f"({expected_mcp_operation_id!r})"
                )
        expected_harness_operation_id = _HARNESS_SESSION_OPERATION_IDS.get(host_action_id_norm)
        if expected_harness_operation_id is not None and operation_id != expected_harness_operation_id:
            raise ValueError(
                "hostActionSurface.requiredActions."
                f"{host_action_id_norm}.operationId must match harness session route "
                f"({expected_harness_operation_id!r})"
            )
        if canonical_cli is not None:
            bound_host = canonical_cli_bindings.get(canonical_cli)
            if bound_host is not None and bound_host != host_action_id_norm:
                raise ValueError(
                    "hostActionSurface.requiredActions canonicalCli binding is ambiguous: "
                    f"{canonical_cli!r} used by {bound_host!r} and {host_action_id_norm!r}"
                )
            canonical_cli_bindings[canonical_cli] = host_action_id_norm
        if mcp_tool is not None:
            bound_host = mcp_tool_bindings.get(mcp_tool)
            if bound_host is not None and bound_host != host_action_id_norm:
                raise ValueError(
                    "hostActionSurface.requiredActions mcpTool binding is ambiguous: "
                    f"{mcp_tool!r} used by {bound_host!r} and {host_action_id_norm!r}"
                )
            mcp_tool_bindings[mcp_tool] = host_action_id_norm

        parsed_actions[host_action_id_norm] = {
            "canonicalCli": canonical_cli,
            "mcpTool": mcp_tool,
            "operationId": operation_id,
        }

    for host_action_id in mcp_only_host_actions:
        row = parsed_actions.get(host_action_id)
        if row is None:
            raise ValueError(
                "hostActionSurface.mcpOnlyHostActions references unknown host action: "
                f"{host_action_id!r}"
            )
        if row.get("canonicalCli") is not None:
            raise ValueError(
                "hostActionSurface.mcpOnlyHostActions requires null canonicalCli for "
                f"{host_action_id!r}"
            )
        if row.get("mcpTool") is None:
            raise ValueError(
                "hostActionSurface.mcpOnlyHostActions requires mcpTool binding for "
                f"{host_action_id!r}"
            )

    failure_classes = _require_object(
        host_action_surface.get("failureClasses"),
        "hostActionSurface.failureClasses",
    )
    missing_failure_class_keys = sorted(
        set(_REQUIRED_HOST_ACTION_SURFACE_FAILURE_CLASS_KEYS) - set(failure_classes)
    )
    if missing_failure_class_keys:
        raise ValueError(
            "hostActionSurface.failureClasses missing required keys: "
            + ", ".join(missing_failure_class_keys)
        )
    unknown_failure_class_keys = sorted(
        set(failure_classes) - set(_REQUIRED_HOST_ACTION_SURFACE_FAILURE_CLASS_KEYS)
    )
    if unknown_failure_class_keys:
        raise ValueError(
            "hostActionSurface.failureClasses includes unknown keys: "
            + ", ".join(unknown_failure_class_keys)
        )
    parsed_failure_classes = {
        key: _require_non_empty_string(
            failure_classes.get(key),
            f"hostActionSurface.failureClasses.{key}",
        )
        for key in _REQUIRED_HOST_ACTION_SURFACE_FAILURE_CLASS_KEYS
    }
    if len(set(parsed_failure_classes.values())) != len(parsed_failure_classes):
        raise ValueError("hostActionSurface.failureClasses must not contain duplicate values")

    return {
        "requiredActions": parsed_actions,
        "mcpOnlyHostActions": list(mcp_only_host_actions),
        "failureClasses": parsed_failure_classes,
    }


def _validate_control_plane_bundle_profile(payload: Any) -> Dict[str, Any]:
    profile = _require_object(payload, "controlPlaneBundleProfile")
    profile_id = _require_non_empty_string(
        profile.get("profileId"),
        "controlPlaneBundleProfile.profileId",
    )
    if profile_id != _CONTROL_PLANE_BUNDLE_PROFILE_ID:
        raise ValueError(
            "controlPlaneBundleProfile.profileId must equal "
            f"{_CONTROL_PLANE_BUNDLE_PROFILE_ID!r}"
        )

    context_family = _require_object(
        profile.get("contextFamily"),
        "controlPlaneBundleProfile.contextFamily",
    )
    context_family_id = _require_non_empty_string(
        context_family.get("id"),
        "controlPlaneBundleProfile.contextFamily.id",
    )
    if context_family_id != _CONTROL_PLANE_BUNDLE_CONTEXT_FAMILY_ID:
        raise ValueError(
            "controlPlaneBundleProfile.contextFamily.id must equal "
            f"{_CONTROL_PLANE_BUNDLE_CONTEXT_FAMILY_ID!r}"
        )
    context_kinds = _require_exact_members(
        _require_string_list(
            context_family.get("contextKinds"),
            "controlPlaneBundleProfile.contextFamily.contextKinds",
        ),
        _CONTROL_PLANE_BUNDLE_CONTEXT_KINDS,
        "controlPlaneBundleProfile.contextFamily.contextKinds",
    )
    morphism_kinds = _require_exact_members(
        _require_string_list(
            context_family.get("morphismKinds"),
            "controlPlaneBundleProfile.contextFamily.morphismKinds",
        ),
        _CONTROL_PLANE_BUNDLE_MORPHISM_KINDS,
        "controlPlaneBundleProfile.contextFamily.morphismKinds",
    )

    artifact_family = _require_object(
        profile.get("artifactFamily"),
        "controlPlaneBundleProfile.artifactFamily",
    )
    artifact_family_id = _require_non_empty_string(
        artifact_family.get("id"),
        "controlPlaneBundleProfile.artifactFamily.id",
    )
    if artifact_family_id != _CONTROL_PLANE_BUNDLE_ARTIFACT_FAMILY_ID:
        raise ValueError(
            "controlPlaneBundleProfile.artifactFamily.id must equal "
            f"{_CONTROL_PLANE_BUNDLE_ARTIFACT_FAMILY_ID!r}"
        )
    artifact_refs_obj = _require_object(
        artifact_family.get("artifactRefs"),
        "controlPlaneBundleProfile.artifactFamily.artifactRefs",
    )
    unknown_artifact_refs = sorted(
        set(artifact_refs_obj) - set(_CONTROL_PLANE_BUNDLE_ARTIFACT_REFS)
    )
    if unknown_artifact_refs:
        raise ValueError(
            "controlPlaneBundleProfile.artifactFamily.artifactRefs includes unknown keys: "
            + ", ".join(unknown_artifact_refs)
        )
    missing_artifact_refs = sorted(
        set(_CONTROL_PLANE_BUNDLE_ARTIFACT_REFS) - set(artifact_refs_obj)
    )
    if missing_artifact_refs:
        raise ValueError(
            "controlPlaneBundleProfile.artifactFamily.artifactRefs missing required keys: "
            + ", ".join(missing_artifact_refs)
        )
    artifact_refs: Dict[str, str] = {}
    for key, expected_path in _CONTROL_PLANE_BUNDLE_ARTIFACT_REFS.items():
        parsed_path = _require_non_empty_string(
            artifact_refs_obj.get(key),
            f"controlPlaneBundleProfile.artifactFamily.artifactRefs.{key}",
        )
        if parsed_path != expected_path:
            raise ValueError(
                "controlPlaneBundleProfile.artifactFamily.artifactRefs."
                f"{key} must equal {expected_path!r}"
            )
        artifact_refs[key] = parsed_path

    reindexing = _require_object(
        profile.get("reindexingCoherence"),
        "controlPlaneBundleProfile.reindexingCoherence",
    )
    reindexing_obligations = _require_exact_members(
        _require_string_list(
            reindexing.get("requiredObligations"),
            "controlPlaneBundleProfile.reindexingCoherence.requiredObligations",
        ),
        _CONTROL_PLANE_BUNDLE_REINDEXING_OBLIGATIONS,
        "controlPlaneBundleProfile.reindexingCoherence.requiredObligations",
    )
    commutation_witness = _require_non_empty_string(
        reindexing.get("commutationWitness"),
        "controlPlaneBundleProfile.reindexingCoherence.commutationWitness",
    )
    if commutation_witness != _CONTROL_PLANE_BUNDLE_COMMUTATION_WITNESS:
        raise ValueError(
            "controlPlaneBundleProfile.reindexingCoherence.commutationWitness must equal "
            f"{_CONTROL_PLANE_BUNDLE_COMMUTATION_WITNESS!r}"
        )

    cover_glue = _require_object(
        profile.get("coverGlue"),
        "controlPlaneBundleProfile.coverGlue",
    )
    worker_cover_kind = _require_non_empty_string(
        cover_glue.get("workerCoverKind"),
        "controlPlaneBundleProfile.coverGlue.workerCoverKind",
    )
    if worker_cover_kind != _CONTROL_PLANE_BUNDLE_WORKER_COVER_KIND:
        raise ValueError(
            "controlPlaneBundleProfile.coverGlue.workerCoverKind must equal "
            f"{_CONTROL_PLANE_BUNDLE_WORKER_COVER_KIND!r}"
        )
    merge_compatibility_witness = _require_non_empty_string(
        cover_glue.get("mergeCompatibilityWitness"),
        "controlPlaneBundleProfile.coverGlue.mergeCompatibilityWitness",
    )
    if merge_compatibility_witness != _CONTROL_PLANE_BUNDLE_COMMUTATION_WITNESS:
        raise ValueError(
            "controlPlaneBundleProfile.coverGlue.mergeCompatibilityWitness must equal "
            f"{_CONTROL_PLANE_BUNDLE_COMMUTATION_WITNESS!r}"
        )
    required_merge_artifacts = _require_exact_members(
        _require_string_list(
            cover_glue.get("requiredMergeArtifacts"),
            "controlPlaneBundleProfile.coverGlue.requiredMergeArtifacts",
        ),
        _CONTROL_PLANE_BUNDLE_REQUIRED_MERGE_ARTIFACTS,
        "controlPlaneBundleProfile.coverGlue.requiredMergeArtifacts",
    )

    authority_split = _require_object(
        profile.get("authoritySplit"),
        "controlPlaneBundleProfile.authoritySplit",
    )
    semantic_authority = _require_exact_members(
        _require_string_list(
            authority_split.get("semanticAuthority"),
            "controlPlaneBundleProfile.authoritySplit.semanticAuthority",
        ),
        _CONTROL_PLANE_BUNDLE_SEMANTIC_AUTHORITY,
        "controlPlaneBundleProfile.authoritySplit.semanticAuthority",
    )
    control_plane_role = _require_non_empty_string(
        authority_split.get("controlPlaneRole"),
        "controlPlaneBundleProfile.authoritySplit.controlPlaneRole",
    )
    if control_plane_role != _CONTROL_PLANE_BUNDLE_CONTROL_PLANE_ROLE:
        raise ValueError(
            "controlPlaneBundleProfile.authoritySplit.controlPlaneRole must equal "
            f"{_CONTROL_PLANE_BUNDLE_CONTROL_PLANE_ROLE!r}"
        )
    forbidden_roles = _require_exact_members(
        _require_string_list(
            authority_split.get("forbiddenControlPlaneRoles"),
            "controlPlaneBundleProfile.authoritySplit.forbiddenControlPlaneRoles",
        ),
        _CONTROL_PLANE_BUNDLE_FORBIDDEN_ROLES,
        "controlPlaneBundleProfile.authoritySplit.forbiddenControlPlaneRoles",
    )

    return {
        "profileId": profile_id,
        "contextFamily": {
            "id": context_family_id,
            "contextKinds": context_kinds,
            "morphismKinds": morphism_kinds,
        },
        "artifactFamily": {
            "id": artifact_family_id,
            "artifactRefs": artifact_refs,
        },
        "reindexingCoherence": {
            "requiredObligations": reindexing_obligations,
            "commutationWitness": commutation_witness,
        },
        "coverGlue": {
            "workerCoverKind": worker_cover_kind,
            "mergeCompatibilityWitness": merge_compatibility_witness,
            "requiredMergeArtifacts": required_merge_artifacts,
        },
        "authoritySplit": {
            "semanticAuthority": semantic_authority,
            "controlPlaneRole": control_plane_role,
            "forbiddenControlPlaneRoles": forbidden_roles,
        },
    }


def _validate_control_plane_kcir_mappings(
    payload: Any,
    *,
    active_epoch: str,
    schema_epoch_discipline: Dict[str, Any],
) -> Dict[str, Any]:
    mappings = _require_object(payload, "controlPlaneKcirMappings")
    profile_id = _require_non_empty_string(
        mappings.get("profileId"),
        "controlPlaneKcirMappings.profileId",
    )

    mapping_table_raw = _require_object(
        mappings.get("mappingTable"),
        "controlPlaneKcirMappings.mappingTable",
    )
    if not mapping_table_raw:
        raise ValueError("controlPlaneKcirMappings.mappingTable must be non-empty")

    parsed_mapping_table: Dict[str, Dict[str, Any]] = {}
    for row_id in sorted(mapping_table_raw):
        row_id_norm = _require_non_empty_string(
            row_id, "controlPlaneKcirMappings.mappingTable.<rowId>"
        )
        row = _require_object(
            mapping_table_raw.get(row_id),
            f"controlPlaneKcirMappings.mappingTable.{row_id_norm}",
        )
        source_kind = _require_non_empty_string(
            row.get("sourceKind"),
            f"controlPlaneKcirMappings.mappingTable.{row_id_norm}.sourceKind",
        )
        target_domain = _require_non_empty_string(
            row.get("targetDomain"),
            f"controlPlaneKcirMappings.mappingTable.{row_id_norm}.targetDomain",
        )
        target_kind = _require_non_empty_string(
            row.get("targetKind"),
            f"controlPlaneKcirMappings.mappingTable.{row_id_norm}.targetKind",
        )
        identity_fields = _require_string_list(
            row.get("identityFields"),
            f"controlPlaneKcirMappings.mappingTable.{row_id_norm}.identityFields",
        )
        if len(set(identity_fields)) != len(identity_fields):
            raise ValueError(
                f"controlPlaneKcirMappings.mappingTable.{row_id_norm}.identityFields must not contain duplicates"
            )
        parsed_mapping_table[row_id_norm] = {
            "sourceKind": source_kind,
            "targetDomain": target_domain,
            "targetKind": target_kind,
            "identityFields": identity_fields,
        }

    lineage = _require_object(
        mappings.get("identityDigestLineage"),
        "controlPlaneKcirMappings.identityDigestLineage",
    )
    digest_algorithm = _require_non_empty_string(
        lineage.get("digestAlgorithm"),
        "controlPlaneKcirMappings.identityDigestLineage.digestAlgorithm",
    )
    ref_profile_path = _require_non_empty_string(
        lineage.get("refProfilePath"),
        "controlPlaneKcirMappings.identityDigestLineage.refProfilePath",
    )
    normalizer_field = _require_non_empty_string(
        lineage.get("normalizerField"),
        "controlPlaneKcirMappings.identityDigestLineage.normalizerField",
    )
    policy_digest_field = _require_non_empty_string(
        lineage.get("policyDigestField"),
        "controlPlaneKcirMappings.identityDigestLineage.policyDigestField",
    )

    compatibility_policy = _require_object(
        mappings.get("compatibilityPolicy"),
        "controlPlaneKcirMappings.compatibilityPolicy",
    )
    legacy_policy = _require_object(
        compatibility_policy.get("legacyNonKcirEncodings"),
        "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings",
    )
    legacy_mode = _require_non_empty_string(
        legacy_policy.get("mode"),
        "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings.mode",
    )
    authority_mode = _require_non_empty_string(
        legacy_policy.get("authorityMode"),
        "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings.authorityMode",
    )
    support_until_epoch = _require_epoch(
        legacy_policy.get("supportUntilEpoch"),
        "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings.supportUntilEpoch",
    )
    if active_epoch > support_until_epoch:
        raise ValueError(
            "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings "
            f"expired at supportUntilEpoch={support_until_epoch!r} (activeEpoch={active_epoch!r})"
        )
    rollover_epoch = schema_epoch_discipline.get("rolloverEpoch")
    if isinstance(rollover_epoch, str) and rollover_epoch:
        if support_until_epoch != rollover_epoch:
            raise ValueError(
                "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings."
                "supportUntilEpoch must match schemaLifecycle.epochDiscipline.rolloverEpoch"
            )
    failure_class = _require_non_empty_string(
        legacy_policy.get("failureClass"),
        "controlPlaneKcirMappings.compatibilityPolicy.legacyNonKcirEncodings.failureClass",
    )

    return {
        "profileId": profile_id,
        "mappingTable": parsed_mapping_table,
        "identityDigestLineage": {
            "digestAlgorithm": digest_algorithm,
            "refProfilePath": ref_profile_path,
            "normalizerField": normalizer_field,
            "policyDigestField": policy_digest_field,
        },
        "compatibilityPolicy": {
            "legacyNonKcirEncodings": {
                "mode": legacy_mode,
                "authorityMode": authority_mode,
                "supportUntilEpoch": support_until_epoch,
                "failureClass": failure_class,
            }
        },
    }


def load_control_plane_contract(path: Path = CONTROL_PLANE_CONTRACT_PATH) -> Dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"failed to read control-plane contract {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json in control-plane contract {path}: {exc}") from exc
    root = _require_object(payload, "control-plane contract root")

    schema = root.get("schema")
    if schema != 1:
        raise ValueError("control-plane contract schema must be 1")

    schema_lifecycle = _require_object(root.get("schemaLifecycle"), "schemaLifecycle")
    active_epoch = _require_epoch(
        schema_lifecycle.get("activeEpoch"), "schemaLifecycle.activeEpoch"
    )
    schema_lifecycle_governance_obj = _require_object(
        schema_lifecycle.get("governance"),
        "schemaLifecycle.governance",
    )
    kind_families_raw = _require_object(
        schema_lifecycle.get("kindFamilies"), "schemaLifecycle.kindFamilies"
    )
    unknown_kind_families = sorted(
        set(kind_families_raw) - set(_REQUIRED_SCHEMA_KIND_FAMILIES)
    )
    if unknown_kind_families:
        raise ValueError(
            "schemaLifecycle.kindFamilies includes unknown families: "
            + ", ".join(unknown_kind_families)
        )
    kind_families: Dict[str, Dict[str, Any]] = {}
    for family_id in _REQUIRED_SCHEMA_KIND_FAMILIES:
        kind_families[family_id] = _require_schema_kind_family(
            kind_families_raw.get(family_id),
            f"schemaLifecycle.kindFamilies.{family_id}",
        )
    schema_epoch_discipline = _validate_schema_lifecycle_epoch_discipline(
        active_epoch, kind_families
    )
    schema_lifecycle_governance = _validate_schema_lifecycle_governance(
        schema_lifecycle_governance_obj,
        schema_epoch_discipline=schema_epoch_discipline,
    )

    contract_kind_declared = _require_non_empty_string(root.get("contractKind"), "contractKind")
    contract_kind = _resolve_kind_in_family(
        "controlPlaneContractKind",
        family=kind_families["controlPlaneContractKind"],
        kind=contract_kind_declared,
        active_epoch=active_epoch,
        label="contractKind",
    )
    if contract_kind != CONTROL_PLANE_CONTRACT_KIND:
        raise ValueError(
            f"control-plane contract kind must resolve to {CONTROL_PLANE_CONTRACT_KIND!r}"
        )
    control_plane_bundle_profile = _validate_control_plane_bundle_profile(
        root.get("controlPlaneBundleProfile")
    )
    control_plane_kcir_mappings = _validate_control_plane_kcir_mappings(
        root.get("controlPlaneKcirMappings"),
        active_epoch=active_epoch,
        schema_epoch_discipline=schema_epoch_discipline,
    )

    evidence_lanes: Dict[str, str] = {}
    evidence_lanes_raw = root.get("evidenceLanes")
    if evidence_lanes_raw is not None:
        evidence_lanes_obj = _require_object(evidence_lanes_raw, "evidenceLanes")
        required_lane_keys = (
            "semanticDoctrine",
            "strictChecker",
            "witnessCommutation",
            "runtimeTransport",
        )
        for key in required_lane_keys:
            evidence_lanes[key] = _require_non_empty_string(
                evidence_lanes_obj.get(key), f"evidenceLanes.{key}"
            )
        if len(set(evidence_lanes.values())) != len(evidence_lanes):
            raise ValueError("evidenceLanes values must not contain duplicates")

    lane_artifact_kinds: Dict[str, Tuple[str, ...]] = {}
    lane_artifact_kinds_raw = root.get("laneArtifactKinds")
    if lane_artifact_kinds_raw is not None:
        lane_artifact_kinds_obj = _require_object(
            lane_artifact_kinds_raw, "laneArtifactKinds"
        )
        for lane_id, kinds_raw in lane_artifact_kinds_obj.items():
            lane_id_norm = _require_non_empty_string(
                lane_id, "laneArtifactKinds.<laneId>"
            )
            lane_artifact_kinds[lane_id_norm] = _require_string_list(
                kinds_raw, f"laneArtifactKinds.{lane_id_norm}"
            )
        if evidence_lanes and not set(lane_artifact_kinds).issubset(
            set(evidence_lanes.values())
        ):
            raise ValueError(
                "laneArtifactKinds keys must be subset of evidenceLanes values"
            )

    checker_core_only_obligations: Tuple[str, ...] = tuple()
    required_cross_lane_witness_route: Optional[str] = None
    lane_ownership_raw = root.get("laneOwnership")
    if lane_ownership_raw is not None:
        lane_ownership = _require_object(lane_ownership_raw, "laneOwnership")
        checker_core_only_obligations = _require_optional_string_list(
            lane_ownership.get("checkerCoreOnlyObligations"),
            "laneOwnership.checkerCoreOnlyObligations",
        )
        required_route_obj = lane_ownership.get("requiredCrossLaneWitnessRoute")
        if required_route_obj is not None:
            required_route = _require_object(
                required_route_obj, "laneOwnership.requiredCrossLaneWitnessRoute"
            )
            required_cross_lane_witness_route = _require_non_empty_string(
                required_route.get("pullbackBaseChange"),
                "laneOwnership.requiredCrossLaneWitnessRoute.pullbackBaseChange",
            )

    lane_failure_classes = _require_optional_string_list(
        root.get("laneFailureClasses"), "laneFailureClasses"
    )
    doctrine_operation_ids = _load_doctrine_operation_ids(DOCTRINE_OP_REGISTRY_PATH)
    worker_lane_authority = _validate_worker_lane_authority_contract(
        root.get("workerLaneAuthority"),
        active_epoch=active_epoch,
    )
    world_descent_contract = _validate_world_descent_contract(
        root.get("worldDescentContract"),
        doctrine_operation_ids=doctrine_operation_ids,
    )
    runtime_route_bindings = _validate_runtime_route_bindings(
        root.get("runtimeRouteBindings"),
        doctrine_operation_ids=doctrine_operation_ids,
    )
    command_surface = _validate_command_surface(root.get("commandSurface"))
    pipeline_wrapper_surface = _validate_pipeline_wrapper_surface(
        root.get("pipelineWrapperSurface")
    )
    host_action_surface = _validate_host_action_surface(
        root.get("hostActionSurface"),
        doctrine_operation_ids=doctrine_operation_ids,
    )

    harness_retry_obj = _require_object(
        root.get("harnessRetry"),
        "harnessRetry",
    )
    harness_retry_policy_kind = _require_non_empty_string(
        harness_retry_obj.get("policyKind"),
        "harnessRetry.policyKind",
    )
    harness_retry_policy_path = _require_non_empty_string(
        harness_retry_obj.get("policyPath"),
        "harnessRetry.policyPath",
    )
    harness_retry_escalation_actions = _require_string_list(
        harness_retry_obj.get("escalationActions"),
        "harnessRetry.escalationActions",
    )
    harness_retry_active_issue_env_keys = _require_string_list(
        harness_retry_obj.get("activeIssueEnvKeys"),
        "harnessRetry.activeIssueEnvKeys",
    )
    harness_retry_issues_path_env_key = _require_non_empty_string(
        harness_retry_obj.get("issuesPathEnvKey"),
        "harnessRetry.issuesPathEnvKey",
    )
    harness_retry_session_path_env_key = _require_non_empty_string(
        harness_retry_obj.get("sessionPathEnvKey"),
        "harnessRetry.sessionPathEnvKey",
    )
    harness_retry_session_path_default = _require_non_empty_string(
        harness_retry_obj.get("sessionPathDefault"),
        "harnessRetry.sessionPathDefault",
    )
    harness_retry_session_issue_field = _require_non_empty_string(
        harness_retry_obj.get("sessionIssueField"),
        "harnessRetry.sessionIssueField",
    )

    required_gate_projection = _require_object(
        root.get("requiredGateProjection"), "requiredGateProjection"
    )
    projection_policy = _resolve_kind_in_family(
        "requiredProjectionPolicy",
        family=kind_families["requiredProjectionPolicy"],
        kind=_require_non_empty_string(
            required_gate_projection.get("projectionPolicy"),
            "requiredGateProjection.projectionPolicy",
        ),
        active_epoch=active_epoch,
        label="requiredGateProjection.projectionPolicy",
    )
    check_ids_raw = _require_object(
        required_gate_projection.get("checkIds"),
        "requiredGateProjection.checkIds",
    )
    required_check_id_keys = (
        "baseline",
        "build",
        "test",
        "testToy",
        "testKcirToy",
        "conformanceCheck",
        "conformanceRun",
        "doctrineCheck",
    )
    check_ids: Dict[str, str] = {}
    for key in required_check_id_keys:
        check_ids[key] = _require_non_empty_string(
            check_ids_raw.get(key), f"requiredGateProjection.checkIds.{key}"
        )
    if len(set(check_ids.values())) != len(check_ids):
        raise ValueError("requiredGateProjection.checkIds must not contain duplicate values")
    check_order = _require_string_list(
        required_gate_projection.get("checkOrder"),
        "requiredGateProjection.checkOrder",
    )
    if set(check_order) != set(check_ids.values()):
        raise ValueError(
            "requiredGateProjection.checkOrder must cover exactly requiredGateProjection.checkIds values"
        )

    required_witness = _require_object(root.get("requiredWitness"), "requiredWitness")
    required_witness_kind = _resolve_kind_in_family(
        "requiredWitnessKind",
        family=kind_families["requiredWitnessKind"],
        kind=_require_non_empty_string(
            required_witness.get("witnessKind"),
            "requiredWitness.witnessKind",
        ),
        active_epoch=active_epoch,
        label="requiredWitness.witnessKind",
    )
    required_decision_kind = _resolve_kind_in_family(
        "requiredDecisionKind",
        family=kind_families["requiredDecisionKind"],
        kind=_require_non_empty_string(
            required_witness.get("decisionKind"),
            "requiredWitness.decisionKind",
        ),
        active_epoch=active_epoch,
        label="requiredWitness.decisionKind",
    )

    instruction_witness = _require_object(
        root.get("instructionWitness"),
        "instructionWitness",
    )
    instruction_witness_kind = _resolve_kind_in_family(
        "instructionWitnessKind",
        family=kind_families["instructionWitnessKind"],
        kind=_require_non_empty_string(
            instruction_witness.get("witnessKind"),
            "instructionWitness.witnessKind",
        ),
        active_epoch=active_epoch,
        label="instructionWitness.witnessKind",
    )
    instruction_policy_kind = _resolve_kind_in_family(
        "instructionPolicyKind",
        family=kind_families["instructionPolicyKind"],
        kind=_require_non_empty_string(
            instruction_witness.get("policyKind"),
            "instructionWitness.policyKind",
        ),
        active_epoch=active_epoch,
        label="instructionWitness.policyKind",
    )
    instruction_policy_digest_prefix = _require_non_empty_string(
        instruction_witness.get("policyDigestPrefix"),
        "instructionWitness.policyDigestPrefix",
    )
    stage1_parity = _validate_stage1_parity_contract(root.get("evidenceStage1Parity"))
    stage1_rollback = _validate_stage1_rollback_contract(root.get("evidenceStage1Rollback"))
    stage2_authority_raw = root.get("evidenceStage2Authority")
    stage2_authority = (
        _validate_stage2_authority_contract(
            stage2_authority_raw,
            active_epoch=active_epoch,
            schema_epoch_discipline=schema_epoch_discipline,
        )
        if stage2_authority_raw is not None
        else None
    )

    return {
        "schema": schema,
        "contractKind": contract_kind,
        "schemaLifecycle": {
            "activeEpoch": active_epoch,
            "governance": schema_lifecycle_governance,
            "kindFamilies": kind_families,
            "epochDiscipline": schema_epoch_discipline,
        },
        "controlPlaneBundleProfile": control_plane_bundle_profile,
        "controlPlaneKcirMappings": control_plane_kcir_mappings,
        "evidenceLanes": evidence_lanes,
        "laneArtifactKinds": lane_artifact_kinds,
        "laneOwnership": {
            "checkerCoreOnlyObligations": checker_core_only_obligations,
            "requiredCrossLaneWitnessRoute": required_cross_lane_witness_route,
        },
        "laneFailureClasses": lane_failure_classes,
        "workerLaneAuthority": worker_lane_authority,
        "worldDescentContract": world_descent_contract,
        "runtimeRouteBindings": runtime_route_bindings,
        "commandSurface": command_surface,
        "pipelineWrapperSurface": pipeline_wrapper_surface,
        "hostActionSurface": host_action_surface,
        "harnessRetry": {
            "policyKind": harness_retry_policy_kind,
            "policyPath": harness_retry_policy_path,
            "escalationActions": harness_retry_escalation_actions,
            "activeIssueEnvKeys": harness_retry_active_issue_env_keys,
            "issuesPathEnvKey": harness_retry_issues_path_env_key,
            "sessionPathEnvKey": harness_retry_session_path_env_key,
            "sessionPathDefault": harness_retry_session_path_default,
            "sessionIssueField": harness_retry_session_issue_field,
        },
        "requiredGateProjection": {
            "projectionPolicy": projection_policy,
            "checkIds": check_ids,
            "checkOrder": check_order,
        },
        "requiredWitness": {
            "witnessKind": required_witness_kind,
            "decisionKind": required_decision_kind,
        },
        "instructionWitness": {
            "witnessKind": instruction_witness_kind,
            "policyKind": instruction_policy_kind,
            "policyDigestPrefix": instruction_policy_digest_prefix,
        },
        "evidenceStage1Parity": stage1_parity,
        "evidenceStage1Rollback": stage1_rollback,
        "evidenceStage2Authority": stage2_authority,
    }


_CONTRACT = load_control_plane_contract()
SCHEMA_LIFECYCLE_ACTIVE_EPOCH: str = _CONTRACT["schemaLifecycle"]["activeEpoch"]
SCHEMA_LIFECYCLE_GOVERNANCE_MODE: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("mode", "")
SCHEMA_LIFECYCLE_GOVERNANCE_DECISION_REF: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("decisionRef", "")
SCHEMA_LIFECYCLE_GOVERNANCE_OWNER: str = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("owner", "")
SCHEMA_LIFECYCLE_ROLLOVER_CADENCE_MONTHS: int | None = _CONTRACT[
    "schemaLifecycle"
].get("governance", {}).get("rolloverCadenceMonths")
SCHEMA_LIFECYCLE_FREEZE_REASON: str | None = _CONTRACT["schemaLifecycle"].get(
    "governance", {}
).get("freezeReason")
SCHEMA_LIFECYCLE_EPOCH_DISCIPLINE: Dict[str, Any] = dict(
    _CONTRACT["schemaLifecycle"].get("epochDiscipline", {})
)
SCHEMA_LIFECYCLE_ROLLOVER_EPOCH: str | None = SCHEMA_LIFECYCLE_EPOCH_DISCIPLINE.get(
    "rolloverEpoch"
)
SCHEMA_KIND_FAMILIES: Dict[str, Dict[str, Any]] = dict(
    _CONTRACT["schemaLifecycle"]["kindFamilies"]
)


def canonical_schema_kind(family_id: str) -> str:
    family = SCHEMA_KIND_FAMILIES.get(family_id)
    if not isinstance(family, dict):
        raise ValueError(
            f"unknown schemaLifecycle kind family: {family_id!r}"
        )
    canonical_kind = family.get("canonicalKind")
    if not isinstance(canonical_kind, str) or not canonical_kind:
        raise ValueError(
            f"schemaLifecycle.kindFamilies.{family_id} missing canonicalKind"
        )
    return canonical_kind


def resolve_schema_kind(
    family_id: str,
    kind: Any,
    *,
    active_epoch: str | None = None,
    label: str | None = None,
) -> str:
    family = SCHEMA_KIND_FAMILIES.get(family_id)
    if not isinstance(family, dict):
        raise ValueError(
            f"unknown schemaLifecycle kind family: {family_id!r}"
        )
    effective_epoch = _require_epoch(
        active_epoch if active_epoch is not None else SCHEMA_LIFECYCLE_ACTIVE_EPOCH,
        "schemaLifecycle.activeEpoch",
    )
    kind_label = label or f"schemaLifecycle.kindFamilies.{family_id}"
    kind_value = _require_non_empty_string(kind, kind_label)
    return _resolve_kind_in_family(
        family_id,
        family=family,
        kind=kind_value,
        active_epoch=effective_epoch,
        label=kind_label,
    )


REQUIRED_PROJECTION_POLICY: str = canonical_schema_kind("requiredProjectionPolicy")
REQUIRED_CHECK_IDS: Dict[str, str] = dict(_CONTRACT["requiredGateProjection"]["checkIds"])
REQUIRED_CHECK_ORDER: Tuple[str, ...] = tuple(
    _CONTRACT["requiredGateProjection"]["checkOrder"]
)

REQUIRED_WITNESS_KIND: str = canonical_schema_kind("requiredWitnessKind")
REQUIRED_DECISION_KIND: str = canonical_schema_kind("requiredDecisionKind")
REQUIRED_DELTA_KIND: str = canonical_schema_kind("requiredDeltaKind")

INSTRUCTION_WITNESS_KIND: str = canonical_schema_kind("instructionWitnessKind")
INSTRUCTION_POLICY_KIND: str = canonical_schema_kind("instructionPolicyKind")
INSTRUCTION_POLICY_DIGEST_PREFIX: str = _CONTRACT["instructionWitness"][
    "policyDigestPrefix"
]

EVIDENCE_LANES: Dict[str, str] = dict(_CONTRACT.get("evidenceLanes", {}))
LANE_ARTIFACT_KINDS: Dict[str, Tuple[str, ...]] = dict(
    _CONTRACT.get("laneArtifactKinds", {})
)
CHECKER_CORE_ONLY_OBLIGATIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("laneOwnership", {}).get("checkerCoreOnlyObligations", ())
)
REQUIRED_CROSS_LANE_WITNESS_ROUTE: Optional[str] = _CONTRACT.get(
    "laneOwnership", {}
).get("requiredCrossLaneWitnessRoute")
LANE_FAILURE_CLASSES: Tuple[str, ...] = tuple(_CONTRACT.get("laneFailureClasses", ()))
WORKER_LANE_DEFAULT_MUTATION_MODE: str = (
    _CONTRACT.get("workerLaneAuthority", {})
    .get("mutationPolicy", {})
    .get("defaultMode", "")
)
WORKER_LANE_ALLOWED_MUTATION_MODES: Tuple[str, ...] = tuple(
    _CONTRACT.get("workerLaneAuthority", {})
    .get("mutationPolicy", {})
    .get("allowedModes", ())
)
WORKER_LANE_MUTATION_ROUTES: Dict[str, str] = dict(
    _CONTRACT.get("workerLaneAuthority", {}).get("mutationRoutes", {})
)
WORKER_LANE_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("workerLaneAuthority", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("policyDrift", "mutationModeDrift", "routeUnbound")
)
WORLD_DESCENT_CONTRACT: Dict[str, Any] = dict(
    _CONTRACT.get("worldDescentContract", {})
)
WORLD_DESCENT_CONTRACT_ID: str = WORLD_DESCENT_CONTRACT.get("contractId", "")
WORLD_DESCENT_REQUIRED_ROUTE_FAMILIES: Tuple[str, ...] = tuple(
    WORLD_DESCENT_CONTRACT.get("requiredRouteFamilies", ())
)
WORLD_DESCENT_REQUIRED_ACTION_ROUTE_BINDINGS: Dict[str, Tuple[str, ...]] = {
    route_family: tuple(host_actions)
    for route_family, host_actions in WORLD_DESCENT_CONTRACT.get(
        "requiredActionRouteBindings", {}
    ).items()
    if isinstance(host_actions, list)
}
WORLD_DESCENT_REQUIRED_STATIC_OPERATION_BINDINGS: Dict[str, Tuple[str, ...]] = {
    route_family: tuple(operation_ids)
    for route_family, operation_ids in WORLD_DESCENT_CONTRACT.get(
        "requiredStaticOperationBindings", {}
    ).items()
    if isinstance(operation_ids, list)
}
WORLD_DESCENT_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    WORLD_DESCENT_CONTRACT.get("failureClasses", {}).get(key, "")
    for key in _WORLD_DESCENT_REQUIRED_FAILURE_CLASS_KEYS
)
RUNTIME_ROUTE_BINDINGS: Dict[str, Dict[str, Any]] = {
    route_id: {
        "operationId": str(route.get("operationId", "")),
        "requiredMorphisms": tuple(route.get("requiredMorphisms", ())),
    }
    for route_id, route in _CONTRACT.get("runtimeRouteBindings", {})
    .get("requiredOperationRoutes", {})
    .items()
    if isinstance(route, dict)
}
RUNTIME_ROUTE_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("runtimeRouteBindings", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("missingRoute", "morphismDrift", "contractUnbound")
)
CONTROL_PLANE_COMMAND_SURFACE: Dict[str, Any] = dict(
    _CONTRACT.get("commandSurface", {})
)
REQUIRED_DECISION_CANONICAL_ENTRYPOINT: Tuple[str, ...] = tuple(
    CONTROL_PLANE_COMMAND_SURFACE.get("requiredDecision", {}).get(
        "canonicalEntrypoint", ()
    )
)
REQUIRED_DECISION_COMPATIBILITY_ALIASES: Tuple[Tuple[str, ...], ...] = tuple(
    tuple(alias)
    for alias in CONTROL_PLANE_COMMAND_SURFACE.get("requiredDecision", {}).get(
        "compatibilityAliases", ()
    )
)
INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT: Tuple[str, ...] = tuple(
    CONTROL_PLANE_COMMAND_SURFACE.get("instructionEnvelopeCheck", {}).get(
        "canonicalEntrypoint", ()
    )
)
INSTRUCTION_ENVELOPE_CHECK_COMPATIBILITY_ALIASES: Tuple[Tuple[str, ...], ...] = tuple(
    tuple(alias)
    for alias in CONTROL_PLANE_COMMAND_SURFACE.get(
        "instructionEnvelopeCheck", {}
    ).get("compatibilityAliases", ())
)
INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT: Tuple[str, ...] = tuple(
    CONTROL_PLANE_COMMAND_SURFACE.get("instructionDecision", {}).get(
        "canonicalEntrypoint", ()
    )
)
INSTRUCTION_DECISION_COMPATIBILITY_ALIASES: Tuple[Tuple[str, ...], ...] = tuple(
    tuple(alias)
    for alias in CONTROL_PLANE_COMMAND_SURFACE.get("instructionDecision", {}).get(
        "compatibilityAliases", ()
    )
)
GOVERNANCE_PROMOTION_CHECK_CANONICAL_ENTRYPOINT: Tuple[str, ...] = tuple(
    CONTROL_PLANE_COMMAND_SURFACE.get("governancePromotionCheck", {}).get(
        "canonicalEntrypoint", ()
    )
)
GOVERNANCE_PROMOTION_CHECK_COMPATIBILITY_ALIASES: Tuple[Tuple[str, ...], ...] = tuple(
    tuple(alias)
    for alias in CONTROL_PLANE_COMMAND_SURFACE.get(
        "governancePromotionCheck", {}
    ).get("compatibilityAliases", ())
)
KCIR_MAPPING_CHECK_CANONICAL_ENTRYPOINT: Tuple[str, ...] = tuple(
    CONTROL_PLANE_COMMAND_SURFACE.get("kcirMappingCheck", {}).get(
        "canonicalEntrypoint", ()
    )
)
KCIR_MAPPING_CHECK_COMPATIBILITY_ALIASES: Tuple[Tuple[str, ...], ...] = tuple(
    tuple(alias)
    for alias in CONTROL_PLANE_COMMAND_SURFACE.get("kcirMappingCheck", {}).get(
        "compatibilityAliases", ()
    )
)
CONTROL_PLANE_COMMAND_SURFACE_FAILURE_CLASS_UNBOUND: str = (
    CONTROL_PLANE_COMMAND_SURFACE.get("failureClasses", {}).get("unbound", "")
)
PIPELINE_WRAPPER_SURFACE: Dict[str, Any] = dict(
    _CONTRACT.get("pipelineWrapperSurface", {})
)
PIPELINE_WRAPPER_REQUIRED_ENTRYPOINT: Tuple[str, ...] = tuple(
    PIPELINE_WRAPPER_SURFACE.get("requiredPipelineEntrypoint", ())
)
PIPELINE_WRAPPER_INSTRUCTION_ENTRYPOINT: Tuple[str, ...] = tuple(
    PIPELINE_WRAPPER_SURFACE.get("instructionPipelineEntrypoint", ())
)
PIPELINE_WRAPPER_REQUIRED_GOVERNANCE_HOOK: str = (
    PIPELINE_WRAPPER_SURFACE.get("requiredGateHooks", {}).get("governance", "")
)
PIPELINE_WRAPPER_REQUIRED_KCIR_MAPPING_HOOK: str = (
    PIPELINE_WRAPPER_SURFACE.get("requiredGateHooks", {}).get("kcirMapping", "")
)
PIPELINE_WRAPPER_INSTRUCTION_GOVERNANCE_HOOK: str = (
    PIPELINE_WRAPPER_SURFACE.get("instructionGateHooks", {}).get("governance", "")
)
PIPELINE_WRAPPER_INSTRUCTION_KCIR_MAPPING_HOOK: str = (
    PIPELINE_WRAPPER_SURFACE.get("instructionGateHooks", {}).get("kcirMapping", "")
)
PIPELINE_WRAPPER_FAILURE_CLASSES: Dict[str, str] = dict(
    PIPELINE_WRAPPER_SURFACE.get("failureClasses", {})
)
HOST_ACTION_SURFACE_REQUIRED_ACTIONS: Dict[str, Dict[str, Optional[str]]] = {
    host_action_id: {
        "canonicalCli": row.get("canonicalCli"),
        "mcpTool": row.get("mcpTool"),
        "operationId": row.get("operationId"),
    }
    for host_action_id, row in _CONTRACT.get("hostActionSurface", {})
    .get("requiredActions", {})
    .items()
    if isinstance(row, dict)
}
HOST_ACTION_SURFACE_MCP_ONLY_ACTIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("hostActionSurface", {}).get("mcpOnlyHostActions", ())
)
HOST_ACTION_SURFACE_FAILURE_CLASSES: Dict[str, str] = dict(
    _CONTRACT.get("hostActionSurface", {}).get("failureClasses", {})
)
CONTROL_PLANE_BUNDLE_PROFILE: Dict[str, Any] = dict(
    _CONTRACT.get("controlPlaneBundleProfile", {})
)
CONTROL_PLANE_BUNDLE_PROFILE_ID: str = CONTROL_PLANE_BUNDLE_PROFILE.get(
    "profileId", ""
)
CONTROL_PLANE_BUNDLE_CONTEXT_FAMILY_ID: str = (
    CONTROL_PLANE_BUNDLE_PROFILE.get("contextFamily", {}).get("id", "")
)
CONTROL_PLANE_BUNDLE_ARTIFACT_FAMILY_ID: str = (
    CONTROL_PLANE_BUNDLE_PROFILE.get("artifactFamily", {}).get("id", "")
)
CONTROL_PLANE_KCIR_MAPPINGS: Dict[str, Any] = dict(
    _CONTRACT.get("controlPlaneKcirMappings", {})
)
CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID: str = CONTROL_PLANE_KCIR_MAPPINGS.get(
    "profileId", ""
)
CONTROL_PLANE_KCIR_MAPPING_TABLE: Dict[str, Dict[str, Any]] = dict(
    CONTROL_PLANE_KCIR_MAPPINGS.get("mappingTable", {})
)
CONTROL_PLANE_KCIR_LEGACY_POLICY: Dict[str, Any] = dict(
    CONTROL_PLANE_KCIR_MAPPINGS.get("compatibilityPolicy", {}).get(
        "legacyNonKcirEncodings", {}
    )
)

HARNESS_RETRY_POLICY_KIND: str = _CONTRACT.get("harnessRetry", {}).get(
    "policyKind",
    "",
)
HARNESS_RETRY_POLICY_PATH: str = _CONTRACT.get("harnessRetry", {}).get(
    "policyPath",
    "",
)
HARNESS_ESCALATION_ACTIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("harnessRetry", {}).get("escalationActions", ())
)
HARNESS_ACTIVE_ISSUE_ENV_KEYS: Tuple[str, ...] = tuple(
    _CONTRACT.get("harnessRetry", {}).get("activeIssueEnvKeys", ())
)
HARNESS_ISSUES_PATH_ENV_KEY: str = _CONTRACT.get("harnessRetry", {}).get(
    "issuesPathEnvKey",
    "",
)
HARNESS_SESSION_PATH_ENV_KEY: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionPathEnvKey",
    "",
)
HARNESS_SESSION_PATH_DEFAULT: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionPathDefault",
    "",
)
HARNESS_SESSION_ISSUE_FIELD: str = _CONTRACT.get("harnessRetry", {}).get(
    "sessionIssueField",
    "",
)
EVIDENCE_STAGE1_PARITY_PROFILE_KIND: str = _CONTRACT.get(
    "evidenceStage1Parity", {}
).get("profileKind", "")
EVIDENCE_STAGE1_PARITY_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage1Parity", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("missing", "mismatch", "unbound")
)
EVIDENCE_STAGE1_ROLLBACK_PROFILE_KIND: str = _CONTRACT.get(
    "evidenceStage1Rollback", {}
).get("profileKind", "")
EVIDENCE_STAGE1_ROLLBACK_WITNESS_KIND: str = _CONTRACT.get(
    "evidenceStage1Rollback", {}
).get("witnessKind", "")
EVIDENCE_STAGE1_ROLLBACK_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage1Rollback", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("precondition", "identityDrift", "unbound")
)
EVIDENCE_STAGE2_AUTHORITY_PROFILE_KIND: str = _CONTRACT.get(
    "evidenceStage2Authority", {}
).get("profileKind", "")
EVIDENCE_STAGE2_AUTHORITY_ACTIVE_STAGE: str = _CONTRACT.get(
    "evidenceStage2Authority", {}
).get("activeStage", "")
EVIDENCE_STAGE2_AUTHORITY_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("authorityAliasViolation", "aliasWindowViolation", "unbound")
)
EVIDENCE_STAGE2_ALIAS_ROLE: str = (
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("compatibilityAlias", {})
    .get("role", "")
)
EVIDENCE_STAGE2_ALIAS_SUPPORT_UNTIL_EPOCH: str = (
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("compatibilityAlias", {})
    .get("supportUntilEpoch", "")
)
EVIDENCE_STAGE2_BIDIR_ROUTE_KIND: str = (
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("routeKind", "")
)
EVIDENCE_STAGE2_BIDIR_OBLIGATION_FIELD_REF: str = (
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("obligationFieldRef", "")
)
EVIDENCE_STAGE2_BIDIR_REQUIRED_OBLIGATIONS: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("requiredObligations", ())
)
EVIDENCE_STAGE2_BIDIR_FAILURE_CLASSES: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("failureClasses", {})
    .get(key, "")
    for key in ("missing", "drift")
)
EVIDENCE_STAGE2_BIDIR_FALLBACK_MODE: str = (
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("fallback", {})
    .get("mode", "")
)
EVIDENCE_STAGE2_BIDIR_FALLBACK_PROFILE_KINDS: Tuple[str, ...] = tuple(
    _CONTRACT.get("evidenceStage2Authority", {})
    .get("bidirEvidenceRoute", {})
    .get("fallback", {})
    .get("profileKinds", ())
)
# Compatibility aliases for transitional readers.
EVIDENCE_STAGE2_KERNEL_REQUIRED_OBLIGATIONS: Tuple[str, ...] = (
    EVIDENCE_STAGE2_BIDIR_REQUIRED_OBLIGATIONS
)
EVIDENCE_STAGE2_KERNEL_FAILURE_CLASSES: Tuple[str, ...] = (
    EVIDENCE_STAGE2_BIDIR_FAILURE_CLASSES
)
