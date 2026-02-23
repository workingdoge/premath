#!/usr/bin/env python3
"""Control-plane KCIR mapping gate helpers for CI pipeline wrappers."""

from __future__ import annotations

import functools
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Tuple

from control_plane_contract import (
    CONTROL_PLANE_KCIR_LEGACY_POLICY,
    CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID,
    CONTROL_PLANE_KCIR_MAPPING_TABLE,
    RUNTIME_ROUTE_BINDINGS,
)


KCIR_MAPPING_CONTRACT_VIOLATION = "kcir_mapping_contract_violation"
KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION = (
    "kcir_mapping_legacy_encoding_authority_violation"
)


@dataclass(frozen=True)
class MappingGateReport:
    """Deterministic mapping-gate evaluation report."""

    profile_id: str
    declared_rows: Tuple[str, ...]
    checked_rows: Tuple[str, ...]
    failure_classes: Tuple[str, ...]


def _required_instruction_rows(include_proposal: bool) -> Tuple[str, ...]:
    rows = ["instructionEnvelope", "coherenceObligations", "doctrineRouteBinding"]
    if include_proposal:
        rows.append("proposalPayload")
    return tuple(rows)


def _required_required_rows() -> Tuple[str, ...]:
    return (
        "coherenceCheckPayload",
        "requiredDecisionInput",
        "coherenceObligations",
        "doctrineRouteBinding",
    )


def _expected_declared_rows() -> Tuple[str, ...]:
    return tuple(
        sorted(
            set(_required_instruction_rows(include_proposal=True))
            | set(_required_required_rows())
        )
    )


def _ordered_failure_classes(values: Tuple[str, ...]) -> Tuple[str, ...]:
    seen: set[str] = set()
    ordered: list[str] = []
    for value in values:
        if not isinstance(value, str):
            continue
        trimmed = value.strip()
        if not trimmed or trimmed in seen:
            continue
        seen.add(trimmed)
        ordered.append(trimmed)
    return tuple(ordered)


def _non_empty_string(value: Any) -> str | None:
    if isinstance(value, str):
        trimmed = value.strip()
        if trimmed:
            return trimmed
    return None


def _load_json_object(path: Path) -> Dict[str, Any] | None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(payload, dict):
        return None
    return payload


def _sha256_file(path: Path) -> str | None:
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return None


def _stable_sha256_json(value: Any) -> str:
    encoded = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


@functools.lru_cache(maxsize=1)
def _global_mapping_context() -> Dict[str, str | None]:
    root = _repo_root()
    coherence_contract = _load_json_object(
        root / "specs/premath/draft/COHERENCE-CONTRACT.json"
    )
    doctrine_site_digest = _sha256_file(root / "specs/premath/draft/DOCTRINE-SITE.json")

    obligation_digest: str | None = None
    coherence_normalizer: str | None = None
    coherence_policy: str | None = None
    if isinstance(coherence_contract, dict):
        binding = coherence_contract.get("binding")
        if isinstance(binding, dict):
            coherence_normalizer = _non_empty_string(binding.get("normalizerId"))
            coherence_policy = _non_empty_string(binding.get("policyDigest"))
        obligations = coherence_contract.get("obligations")
        if isinstance(obligations, list):
            obligation_ids: list[str] = []
            for row in obligations:
                if isinstance(row, dict):
                    obligation_id = _non_empty_string(row.get("id"))
                    if obligation_id is not None:
                        obligation_ids.append(obligation_id)
            if obligation_ids:
                obligation_digest = _stable_sha256_json(sorted(set(obligation_ids)))

    operation_ids = sorted(
        {
            str(route.get("operationId", "")).strip()
            for route in RUNTIME_ROUTE_BINDINGS.values()
            if isinstance(route, dict) and str(route.get("operationId", "")).strip()
        }
    )
    operation_id_joined = ",".join(operation_ids) if operation_ids else None

    return {
        "obligationDigest": obligation_digest,
        "coherenceNormalizerId": coherence_normalizer,
        "coherencePolicyDigest": coherence_policy,
        "siteDigest": doctrine_site_digest,
        "operationId": operation_id_joined,
    }


def _legacy_policy_failure_class() -> str:
    candidate = _non_empty_string(CONTROL_PLANE_KCIR_LEGACY_POLICY.get("failureClass"))
    return candidate or KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION


def _legacy_authority_forbidden() -> bool:
    return _non_empty_string(
        CONTROL_PLANE_KCIR_LEGACY_POLICY.get("authorityMode")
    ) == "forbidden"


def _mapping_row_identities(
    row_id: str,
    *,
    witness: Dict[str, Any] | None,
    proposal_ingest: Dict[str, Any] | None,
    decision: Dict[str, Any] | None,
    decision_digest: str | None,
) -> Dict[str, str | None]:
    row = CONTROL_PLANE_KCIR_MAPPING_TABLE.get(row_id, {})
    identity_fields = row.get("identityFields", ())
    if not isinstance(identity_fields, (list, tuple)):
        return {}
    global_ctx = _global_mapping_context()
    coherence_policy_digest = _non_empty_string(global_ctx.get("coherencePolicyDigest"))
    coherence_normalizer_id = _non_empty_string(global_ctx.get("coherenceNormalizerId"))
    obligation_digest = _non_empty_string(global_ctx.get("obligationDigest"))
    doctrine_site_digest = _non_empty_string(global_ctx.get("siteDigest"))
    doctrine_operation_id = _non_empty_string(global_ctx.get("operationId"))

    values: Dict[str, str | None] = {}
    for field in identity_fields:
        if not isinstance(field, str):
            continue
        value: str | None = None
        if row_id == "proposalPayload":
            if isinstance(proposal_ingest, dict):
                value = _non_empty_string(proposal_ingest.get(field))
            if value is None and isinstance(witness, dict):
                value = _non_empty_string(witness.get(field))
        elif row_id == "requiredDecisionInput":
            if field == "requiredDigest":
                value = (
                    _non_empty_string(decision.get("witnessSha256"))
                    if isinstance(decision, dict)
                else None
                )
            elif field == "decisionDigest":
                value = _non_empty_string(decision_digest)
            elif isinstance(decision, dict):
                value = _non_empty_string(decision.get(field))
        elif row_id == "coherenceObligations":
            if field == "obligationDigest":
                value = obligation_digest
            elif field == "normalizerId":
                value = coherence_normalizer_id or (
                    _non_empty_string(witness.get("normalizerId"))
                    if isinstance(witness, dict)
                    else None
                )
            elif field == "policyDigest":
                value = coherence_policy_digest or (
                    _non_empty_string(witness.get("policyDigest"))
                    if isinstance(witness, dict)
                    else None
                )
        elif row_id == "doctrineRouteBinding":
            if field == "operationId":
                value = doctrine_operation_id
            elif field == "siteDigest":
                value = doctrine_site_digest
            elif field == "policyDigest":
                value = coherence_policy_digest or (
                    _non_empty_string(witness.get("policyDigest"))
                    if isinstance(witness, dict)
                    else None
                )
        else:
            if isinstance(witness, dict):
                value = _non_empty_string(witness.get(field))
        values[field] = value
    return values


def _rows_for_instruction(
    envelope: Dict[str, Any] | None,
    witness: Dict[str, Any] | None,
) -> Tuple[str, ...]:
    include_proposal = False
    if isinstance(envelope, dict):
        proposal = envelope.get("proposal")
        llm_proposal = envelope.get("llmProposal")
        if isinstance(proposal, dict) or isinstance(llm_proposal, dict):
            include_proposal = True
    elif isinstance(witness, dict) and isinstance(witness.get("proposalIngest"), dict):
        include_proposal = True
    return _required_instruction_rows(include_proposal=include_proposal)


def evaluate_instruction_mapping(
    root: Path,
    *,
    instruction_path: Path,
    instruction_id: str,
    strict: bool,
) -> MappingGateReport:
    witness_path = root / "artifacts/ciwitness" / f"{instruction_id}.json"
    witness = _load_json_object(witness_path)
    envelope = _load_json_object(instruction_path)
    proposal_ingest = (
        witness.get("proposalIngest")
        if isinstance(witness, dict) and isinstance(witness.get("proposalIngest"), dict)
        else None
    )

    declared_rows = tuple(sorted(CONTROL_PLANE_KCIR_MAPPING_TABLE))
    checked_rows = _rows_for_instruction(envelope, witness)
    failures: list[str] = []

    if strict:
        if set(declared_rows) != set(_expected_declared_rows()):
            failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
        if witness is None:
            failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
        for row_id in checked_rows:
            if row_id not in CONTROL_PLANE_KCIR_MAPPING_TABLE:
                failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                continue
            identities = _mapping_row_identities(
                row_id,
                witness=witness,
                proposal_ingest=proposal_ingest,
                decision=None,
                decision_digest=None,
            )
            if not identities:
                failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                continue
            for value in identities.values():
                if value is None:
                    failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                    break

        if _legacy_authority_forbidden() and isinstance(envelope, dict):
            has_legacy_alias = (
                envelope.get("proposal") is None
                and isinstance(envelope.get("llmProposal"), dict)
            )
            if has_legacy_alias:
                failures.append(_legacy_policy_failure_class())

    return MappingGateReport(
        profile_id=CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID,
        declared_rows=declared_rows,
        checked_rows=checked_rows,
        failure_classes=_ordered_failure_classes(tuple(failures)),
    )


def evaluate_required_mapping(
    root: Path,
    *,
    strict: bool,
) -> MappingGateReport:
    witness_path = root / "artifacts/ciwitness/latest-required.json"
    decision_path = root / "artifacts/ciwitness/latest-decision.json"

    witness = _load_json_object(witness_path)
    decision = _load_json_object(decision_path)
    decision_digest = _sha256_file(decision_path)
    declared_rows = tuple(sorted(CONTROL_PLANE_KCIR_MAPPING_TABLE))
    checked_rows = _required_required_rows()
    failures: list[str] = []

    if strict:
        if set(declared_rows) != set(_expected_declared_rows()):
            failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
        if witness is None:
            failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
        if decision is None:
            failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)

        for row_id in checked_rows:
            if row_id not in CONTROL_PLANE_KCIR_MAPPING_TABLE:
                failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                continue
            identities = _mapping_row_identities(
                row_id,
                witness=witness,
                proposal_ingest=None,
                decision=decision,
                decision_digest=decision_digest,
            )
            if not identities:
                failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                continue
            for value in identities.values():
                if value is None:
                    failures.append(KCIR_MAPPING_CONTRACT_VIOLATION)
                    break

        if _legacy_authority_forbidden():
            typed_core_digest = None
            authority_alias_digest = None
            if isinstance(witness, dict):
                typed_core_digest = _non_empty_string(
                    witness.get("typedCoreProjectionDigest")
                )
                authority_alias_digest = _non_empty_string(
                    witness.get("authorityPayloadDigest")
                )
            if isinstance(decision, dict):
                typed_core_digest = typed_core_digest or _non_empty_string(
                    decision.get("typedCoreProjectionDigest")
                )
                authority_alias_digest = authority_alias_digest or _non_empty_string(
                    decision.get("authorityPayloadDigest")
                )
            if authority_alias_digest and not typed_core_digest:
                failures.append(_legacy_policy_failure_class())

    return MappingGateReport(
        profile_id=CONTROL_PLANE_KCIR_MAPPING_PROFILE_ID,
        declared_rows=declared_rows,
        checked_rows=checked_rows,
        failure_classes=_ordered_failure_classes(tuple(failures)),
    )


def render_mapping_summary_lines(report: MappingGateReport) -> list[str]:
    declared_count = len(set(report.declared_rows))
    checked_set = set(report.checked_rows)
    checked_count = len(checked_set)
    missing_rows = sorted(set(report.declared_rows) - checked_set)
    rows = ", ".join(report.checked_rows) if report.checked_rows else "(none)"
    missing = ", ".join(missing_rows) if missing_rows else "(none)"
    failures = ", ".join(report.failure_classes) if report.failure_classes else "(none)"
    return [
        f"- KCIR mapping profile: `{report.profile_id or '(missing)'}`",
        f"- KCIR mapping coverage: `{checked_count}/{declared_count}`",
        f"- KCIR mapping rows: `{rows}`",
        f"- KCIR mapping rows missing: `{missing}`",
        f"- KCIR mapping failures: `{failures}`",
    ]
