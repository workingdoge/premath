#!/usr/bin/env python3
"""Validation helpers for ci.required witness artifacts."""

from __future__ import annotations

from typing import Any, Dict, List, Sequence, Tuple

from change_projection import PROJECTION_POLICY, normalize_paths, project_required_checks


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


def verify_required_witness_payload(
    witness: Dict[str, Any],
    changed_paths: Sequence[str],
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
    _check_str_field(witness, "witnessKind", "ci.required.v1", errors)
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
        result_check_ids.append(check_id)

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
        "docsOnly": projection.docs_only,
        "reasons": expected_reasons,
        "expectedVerdict": expected_verdict,
    }

    return errors, derived
