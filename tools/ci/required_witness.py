#!/usr/bin/env python3
"""Validation helpers for ci.required witness artifacts."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any, Dict, List, Mapping, Optional, Sequence, Tuple

from change_projection import PROJECTION_POLICY, normalize_paths, project_required_checks
from control_plane_contract import REQUIRED_WITNESS_KIND
from gate_witness_envelope import stable_sha256


_HEX_64_RE = re.compile(r"^[0-9a-f]{64}$")


def _string_list(value: Any, label: str, errors: List[str]) -> List[str]:
    if not isinstance(value, list):
        errors.append(f"{label} must be a list")
        return []

    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{label}[{idx}] must be a non-empty string")
            continue
        out.append(item.strip())
    return out


def _check_str_field(witness: Dict[str, Any], key: str, expected: str, errors: List[str]) -> None:
    value = witness.get(key)
    if value != expected:
        errors.append(f"{key} mismatch (expected={expected!r}, actual={value!r})")


def _normalize_rel_path(path: str) -> str:
    normalized = path.strip().replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def _load_gate_witness_payload(
    artifact_rel_path: str,
    errors: List[str],
    witness_root: Optional[Path],
    gate_witness_payloads: Optional[Mapping[str, Dict[str, Any]]],
) -> Optional[Dict[str, Any]]:
    if gate_witness_payloads is not None:
        payload = gate_witness_payloads.get(artifact_rel_path)
        if payload is None:
            errors.append(f"gateWitnessRefs missing inline payload: {artifact_rel_path}")
            return None
        if not isinstance(payload, dict):
            errors.append(f"gateWitness payload must be an object: {artifact_rel_path}")
            return None
        return payload

    if witness_root is None:
        return None

    root = witness_root.resolve()
    target = (root / artifact_rel_path).resolve()
    try:
        target.relative_to(root)
    except ValueError:
        errors.append(f"gateWitnessRefs path escapes witness root: {artifact_rel_path}")
        return None

    if not target.exists() or not target.is_file():
        errors.append(f"gateWitnessRefs artifact not found: {artifact_rel_path}")
        return None

    try:
        with target.open("r", encoding="utf-8") as f:
            payload = json.load(f)
    except json.JSONDecodeError as exc:
        errors.append(f"gateWitness artifact is not valid json ({artifact_rel_path}): {exc}")
        return None

    if not isinstance(payload, dict):
        errors.append(f"gateWitness artifact root must be object: {artifact_rel_path}")
        return None
    return payload


def _verify_gate_witness_refs(
    witness: Dict[str, Any],
    executed_checks: Sequence[str],
    results_by_check: Mapping[str, Dict[str, Any]],
    errors: List[str],
    witness_root: Optional[Path],
    gate_witness_payloads: Optional[Mapping[str, Dict[str, Any]]],
) -> Dict[str, str]:
    source_by_check: Dict[str, str] = {}
    refs_raw = witness.get("gateWitnessRefs")
    if refs_raw is None:
        return source_by_check
    if not isinstance(refs_raw, list):
        errors.append("gateWitnessRefs must be a list when present")
        return source_by_check

    if len(refs_raw) != len(executed_checks):
        errors.append(
            "gateWitnessRefs length mismatch "
            f"(expected={len(executed_checks)}, actual={len(refs_raw)})"
        )

    for idx, ref in enumerate(refs_raw):
        if not isinstance(ref, dict):
            errors.append(f"gateWitnessRefs[{idx}] must be an object")
            continue

        check_id = ref.get("checkId")
        if not isinstance(check_id, str) or not check_id.strip():
            errors.append(f"gateWitnessRefs[{idx}].checkId must be a non-empty string")
            continue
        check_id = check_id.strip()

        expected_check_id = executed_checks[idx] if idx < len(executed_checks) else None
        if expected_check_id is not None and check_id != expected_check_id:
            errors.append(
                f"gateWitnessRefs[{idx}].checkId mismatch "
                f"(expected={expected_check_id!r}, actual={check_id!r})"
            )

        result_row = results_by_check.get(check_id)
        if result_row is None:
            errors.append(f"gateWitnessRefs[{idx}] unknown checkId: {check_id!r}")
            continue
        expected_gate_result = "accepted" if int(result_row["exitCode"]) == 0 else "rejected"

        source = ref.get("source")
        if not isinstance(source, str) or source not in {"native", "fallback"}:
            errors.append(
                f"gateWitnessRefs[{idx}].source must be 'native' or 'fallback' "
                f"(actual={source!r})"
            )
        else:
            source_by_check[check_id] = source

        artifact_rel_path = ref.get("artifactRelPath")
        if not isinstance(artifact_rel_path, str) or not artifact_rel_path.strip():
            errors.append(f"gateWitnessRefs[{idx}].artifactRelPath must be a non-empty string")
            continue
        artifact_rel_path = _normalize_rel_path(artifact_rel_path)
        if artifact_rel_path.startswith("/") or artifact_rel_path.startswith("../"):
            errors.append(f"gateWitnessRefs[{idx}].artifactRelPath must be relative")
            continue
        if "/../" in artifact_rel_path or artifact_rel_path == "..":
            errors.append(f"gateWitnessRefs[{idx}].artifactRelPath must not contain '..'")
            continue

        sha256 = ref.get("sha256")
        if not isinstance(sha256, str) or not _HEX_64_RE.fullmatch(sha256):
            errors.append(f"gateWitnessRefs[{idx}].sha256 must be 64 lowercase hex chars")
            continue

        ref_witness_kind = ref.get("witnessKind")
        if ref_witness_kind is not None and ref_witness_kind != "gate":
            errors.append(
                f"gateWitnessRefs[{idx}].witnessKind mismatch "
                f"(expected='gate', actual={ref_witness_kind!r})"
            )

        ref_result = ref.get("result")
        if ref_result is not None and ref_result != expected_gate_result:
            errors.append(
                f"gateWitnessRefs[{idx}].result mismatch "
                f"(expected={expected_gate_result!r}, actual={ref_result!r})"
            )

        payload = _load_gate_witness_payload(
            artifact_rel_path=artifact_rel_path,
            errors=errors,
            witness_root=witness_root,
            gate_witness_payloads=gate_witness_payloads,
        )
        if payload is None:
            continue

        payload_digest = stable_sha256(payload)
        if payload_digest != sha256:
            errors.append(
                f"gateWitnessRefs[{idx}] digest mismatch "
                f"(expected={sha256}, actual={payload_digest})"
            )

        payload_kind = payload.get("witnessKind")
        if payload_kind != "gate":
            errors.append(
                f"gateWitnessRefs[{idx}] payload witnessKind mismatch "
                f"(expected='gate', actual={payload_kind!r})"
            )

        payload_result = payload.get("result")
        if payload_result != expected_gate_result:
            errors.append(
                f"gateWitnessRefs[{idx}] payload result mismatch "
                f"(expected={expected_gate_result!r}, actual={payload_result!r})"
            )

        payload_failures = payload.get("failures")
        if not isinstance(payload_failures, list):
            errors.append(f"gateWitnessRefs[{idx}] payload failures must be a list")
        else:
            if payload_result == "accepted" and payload_failures:
                errors.append(
                    f"gateWitnessRefs[{idx}] accepted payload must have empty failures list"
                )
            if payload_result == "rejected" and not payload_failures:
                errors.append(
                    f"gateWitnessRefs[{idx}] rejected payload must include failures"
                )

        ref_run_id = ref.get("runId")
        if ref_run_id is not None:
            if not isinstance(ref_run_id, str) or not ref_run_id.strip():
                errors.append(f"gateWitnessRefs[{idx}].runId must be a non-empty string")
            elif payload.get("runId") != ref_run_id:
                errors.append(
                    f"gateWitnessRefs[{idx}] runId mismatch "
                    f"(ref={ref_run_id!r}, payload={payload.get('runId')!r})"
                )
    return source_by_check


def verify_required_witness_payload(
    witness: Dict[str, Any],
    changed_paths: Sequence[str],
    witness_root: Optional[Path] = None,
    gate_witness_payloads: Optional[Mapping[str, Dict[str, Any]]] = None,
    native_required_checks: Optional[Sequence[str]] = None,
) -> Tuple[List[str], Dict[str, Any]]:
    """Verify a ci.required witness against the deterministic projection contract.

    Returns `(errors, derived)` where `derived` includes normalized projection material.
    """
    errors: List[str] = []

    normalized_paths = normalize_paths(changed_paths)
    projection = project_required_checks(normalized_paths)
    expected_required = list(projection.required_checks)

    if witness.get("ciSchema") != 1:
        errors.append(f"ciSchema must be 1 (actual={witness.get('ciSchema')!r})")
    _check_str_field(witness, "witnessKind", REQUIRED_WITNESS_KIND, errors)
    _check_str_field(witness, "projectionPolicy", PROJECTION_POLICY, errors)
    _check_str_field(witness, "policyDigest", PROJECTION_POLICY, errors)

    witness_changed_paths = normalize_paths(
        _string_list(witness.get("changedPaths"), "changedPaths", errors)
    )
    if witness_changed_paths != normalized_paths:
        errors.append(
            "changedPaths mismatch "
            f"(expected={normalized_paths}, actual={witness_changed_paths})"
        )

    projection_digest = witness.get("projectionDigest")
    expected_digest = projection.projection_digest
    if projection_digest != expected_digest:
        errors.append(
            "projectionDigest mismatch "
            f"(expected={expected_digest!r}, actual={projection_digest!r})"
        )

    required_checks = _string_list(witness.get("requiredChecks"), "requiredChecks", errors)
    if required_checks != expected_required:
        errors.append(
            "requiredChecks mismatch "
            f"(expected={expected_required}, actual={required_checks})"
        )

    executed_checks = _string_list(witness.get("executedChecks"), "executedChecks", errors)
    if executed_checks != required_checks:
        errors.append(
            "executedChecks mismatch "
            f"(expected={required_checks}, actual={executed_checks})"
        )

    results_raw = witness.get("results")
    if not isinstance(results_raw, list):
        errors.append("results must be a list")
        results_raw = []

    result_check_ids: List[str] = []
    results_by_check: Dict[str, Dict[str, Any]] = {}
    failed_count = 0
    for idx, row in enumerate(results_raw):
        if not isinstance(row, dict):
            errors.append(f"results[{idx}] must be an object")
            continue

        check_id = row.get("checkId")
        status = row.get("status")
        exit_code = row.get("exitCode")

        if not isinstance(check_id, str) or not check_id:
            errors.append(f"results[{idx}].checkId must be a non-empty string")
            continue
        if check_id in results_by_check:
            errors.append(f"results[{idx}].checkId must be unique (duplicate={check_id!r})")
            continue
        result_check_ids.append(check_id)
        results_by_check[check_id] = row

        if status not in {"passed", "failed"}:
            errors.append(f"results[{idx}].status must be 'passed' or 'failed'")
        if not isinstance(exit_code, int):
            errors.append(f"results[{idx}].exitCode must be an integer")
            continue

        expected_status = "passed" if exit_code == 0 else "failed"
        if status != expected_status:
            errors.append(
                f"results[{idx}] status/exitCode mismatch "
                f"(status={status!r}, exitCode={exit_code})"
            )

        if exit_code != 0:
            failed_count += 1

    if result_check_ids != executed_checks:
        errors.append(
            "results checkId sequence mismatch "
            f"(expected={executed_checks}, actual={result_check_ids})"
        )

    source_by_check = _verify_gate_witness_refs(
        witness=witness,
        executed_checks=executed_checks,
        results_by_check=results_by_check,
        errors=errors,
        witness_root=witness_root,
        gate_witness_payloads=gate_witness_payloads,
    )

    native_required = _string_list(
        list(native_required_checks or []),
        "nativeRequiredChecks",
        errors,
    )
    for idx, check_id in enumerate(native_required):
        if check_id not in executed_checks:
            errors.append(
                f"nativeRequiredChecks[{idx}] not executed "
                f"(checkId={check_id!r}, executed={executed_checks})"
            )
            continue
        source = source_by_check.get(check_id)
        if source != "native":
            errors.append(
                f"nativeRequiredChecks[{idx}] requires native source "
                f"(checkId={check_id!r}, source={source!r})"
            )

    docs_only = witness.get("docsOnly")
    if docs_only is not projection.docs_only:
        errors.append(
            f"docsOnly mismatch (expected={projection.docs_only!r}, actual={docs_only!r})"
        )

    reasons = _string_list(witness.get("reasons"), "reasons", errors)
    expected_reasons = list(projection.reasons)
    if reasons != expected_reasons:
        errors.append(
            f"reasons mismatch (expected={expected_reasons}, actual={reasons})"
        )

    expected_verdict = "accepted" if failed_count == 0 else "rejected"
    verdict_class = witness.get("verdictClass")
    if verdict_class != expected_verdict:
        errors.append(
            f"verdictClass mismatch (expected={expected_verdict!r}, actual={verdict_class!r})"
        )

    failure_classes = _string_list(witness.get("failureClasses"), "failureClasses", errors)
    expected_failure_classes = [] if failed_count == 0 else ["check_failed"]
    if sorted(failure_classes) != sorted(expected_failure_classes):
        errors.append(
            "failureClasses mismatch "
            f"(expected={expected_failure_classes}, actual={failure_classes})"
        )

    derived = {
        "changedPaths": normalized_paths,
        "projectionDigest": expected_digest,
        "requiredChecks": expected_required,
        "executedChecks": executed_checks,
        "gateWitnessSourceByCheck": source_by_check,
        "docsOnly": projection.docs_only,
        "reasons": expected_reasons,
        "expectedVerdict": expected_verdict,
    }

    return errors, derived
