#!/usr/bin/env python3
"""Deterministic retry-policy loader/classifier for CI pipeline wrappers."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Sequence, Tuple


DEFAULT_RETRY_POLICY_REL_PATH = Path("policies/control/harness-retry-policy-v1.json")
RETRY_POLICY_KIND = "ci.harness.retry.policy.v1"


class RetryPolicyError(ValueError):
    """Retry-policy validation error with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


@dataclass(frozen=True)
class RetryDecision:
    """Deterministic per-attempt retry/escalation decision."""

    attempt: int
    retry: bool
    max_attempts: int
    backoff_class: str
    escalation_action: str
    rule_id: str
    matched_failure_class: str
    failure_classes: Tuple[str, ...]


def _canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _stable_hash(value: Any) -> str:
    return hashlib.sha256(_canonical_json(value).encode("utf-8")).hexdigest()


def normalize_failure_classes(values: Sequence[str] | None) -> Tuple[str, ...]:
    if values is None:
        return tuple()
    normalized = sorted(
        {
            value.strip()
            for value in values
            if isinstance(value, str) and value.strip()
        }
    )
    return tuple(normalized)


def _require_non_empty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise RetryPolicyError("retry_policy_invalid_shape", f"{label} must be a non-empty string")
    return value.strip()


def _require_positive_int(value: Any, label: str) -> int:
    if not isinstance(value, int) or value < 1:
        raise RetryPolicyError("retry_policy_invalid_shape", f"{label} must be an integer >= 1")
    return value


def _parse_rule(rule_raw: Any, *, label: str, require_failure_classes: bool) -> Dict[str, Any]:
    if not isinstance(rule_raw, dict):
        raise RetryPolicyError("retry_policy_invalid_shape", f"{label} must be an object")
    rule_id = _require_non_empty_string(rule_raw.get("ruleId"), f"{label}.ruleId")
    max_attempts = _require_positive_int(rule_raw.get("maxAttempts"), f"{label}.maxAttempts")
    backoff_class = _require_non_empty_string(rule_raw.get("backoffClass"), f"{label}.backoffClass")
    escalation_action = _require_non_empty_string(
        rule_raw.get("escalationAction"), f"{label}.escalationAction"
    )

    failure_classes: Tuple[str, ...] = tuple()
    if require_failure_classes:
        classes_raw = rule_raw.get("failureClasses")
        if not isinstance(classes_raw, list) or not classes_raw:
            raise RetryPolicyError(
                "retry_policy_invalid_shape", f"{label}.failureClasses must be a non-empty list"
            )
        failure_classes = normalize_failure_classes(classes_raw)
        if not failure_classes:
            raise RetryPolicyError(
                "retry_policy_invalid_shape",
                f"{label}.failureClasses must contain non-empty strings",
            )

    return {
        "ruleId": rule_id,
        "maxAttempts": max_attempts,
        "backoffClass": backoff_class,
        "escalationAction": escalation_action,
        "failureClasses": failure_classes,
    }


def load_retry_policy(repo_root: Path, policy_path: Path | None = None) -> Dict[str, Any]:
    path = policy_path or DEFAULT_RETRY_POLICY_REL_PATH
    if not path.is_absolute():
        path = (repo_root / path).resolve()

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise RetryPolicyError("retry_policy_missing", f"retry policy not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise RetryPolicyError(
            "retry_policy_invalid_json", f"invalid JSON in retry policy {path}: {exc}"
        ) from exc
    except OSError as exc:
        raise RetryPolicyError("retry_policy_io_error", f"failed reading retry policy {path}: {exc}") from exc

    if not isinstance(payload, dict):
        raise RetryPolicyError("retry_policy_invalid_shape", "retry policy root must be an object")
    if payload.get("schema") != 1:
        raise RetryPolicyError("retry_policy_invalid_shape", "retry policy schema must be 1")
    policy_kind = _require_non_empty_string(payload.get("policyKind"), "policyKind")
    if policy_kind != RETRY_POLICY_KIND:
        raise RetryPolicyError(
            "retry_policy_invalid_shape",
            f"policyKind must be {RETRY_POLICY_KIND!r} (got {policy_kind!r})",
        )
    policy_id = _require_non_empty_string(payload.get("policyId"), "policyId")
    policy_digest = _require_non_empty_string(payload.get("policyDigest"), "policyDigest")
    if not policy_digest.startswith("pol1_"):
        raise RetryPolicyError(
            "retry_policy_invalid_digest",
            "policyDigest must use pol1_ prefix",
        )

    default_rule_raw = payload.get("defaultRule")
    default_rule = _parse_rule(
        {
            "ruleId": "default",
            **(default_rule_raw if isinstance(default_rule_raw, dict) else {}),
        },
        label="defaultRule",
        require_failure_classes=False,
    )

    rules_raw = payload.get("rules")
    if not isinstance(rules_raw, list) or not rules_raw:
        raise RetryPolicyError("retry_policy_invalid_shape", "rules must be a non-empty list")
    rules_by_failure_class: Dict[str, Dict[str, Any]] = {}
    seen_rule_ids = {default_rule["ruleId"]}
    rules: list[Dict[str, Any]] = []
    for idx, rule_raw in enumerate(rules_raw):
        rule = _parse_rule(
            rule_raw,
            label=f"rules[{idx}]",
            require_failure_classes=True,
        )
        rule_id = str(rule["ruleId"])
        if rule_id in seen_rule_ids:
            raise RetryPolicyError(
                "retry_policy_invalid_shape", f"duplicate ruleId in rules: {rule_id!r}"
            )
        seen_rule_ids.add(rule_id)
        for failure_class in rule["failureClasses"]:
            if failure_class in rules_by_failure_class:
                previous = rules_by_failure_class[failure_class]["ruleId"]
                raise RetryPolicyError(
                    "retry_policy_invalid_shape",
                    (
                        "failureClasses must be disjoint across rules "
                        f"(duplicate {failure_class!r} in {previous!r} and {rule_id!r})"
                    ),
                )
            rules_by_failure_class[failure_class] = rule
        rules.append(rule)

    digest_payload = dict(payload)
    digest_payload.pop("policyDigest", None)
    expected_digest = "pol1_" + _stable_hash(digest_payload)
    if policy_digest != expected_digest:
        raise RetryPolicyError(
            "retry_policy_invalid_digest",
            f"policyDigest mismatch (expected {expected_digest}, got {policy_digest})",
        )

    return {
        "policyPath": str(path),
        "policyKind": policy_kind,
        "policyId": policy_id,
        "policyDigest": policy_digest,
        "defaultRule": default_rule,
        "rules": tuple(rules),
        "rulesByFailureClass": rules_by_failure_class,
    }


def failure_classes_from_witness_payload(payload: Any) -> Tuple[str, ...]:
    if not isinstance(payload, dict):
        return ("pipeline_invalid_witness_shape",)

    classes: list[str] = []
    for key in ("failureClasses", "operationalFailureClasses", "semanticFailureClasses"):
        values = payload.get(key)
        if isinstance(values, list):
            classes.extend(str(value) for value in values if isinstance(value, str))
    normalized = normalize_failure_classes(classes)
    if normalized:
        return normalized

    verdict = payload.get("verdictClass")
    if verdict == "accepted":
        return tuple()
    return ("pipeline_missing_failure_class",)


def failure_classes_from_witness_path(path: Path) -> Tuple[str, ...]:
    if not path.exists():
        return ("pipeline_missing_witness",)
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return ("pipeline_invalid_witness_json",)
    except OSError:
        return ("pipeline_witness_io_error",)
    return failure_classes_from_witness_payload(payload)


def resolve_retry_decision(
    policy: Dict[str, Any],
    failure_classes: Sequence[str] | None,
    *,
    attempt: int,
) -> RetryDecision:
    if attempt < 1:
        raise RetryPolicyError("retry_policy_invalid_shape", "attempt must be >= 1")
    normalized = normalize_failure_classes(failure_classes)
    matched_failure_class = normalized[0] if normalized else "pipeline_missing_failure_class"
    rule = policy["defaultRule"]
    for failure_class in normalized:
        candidate = policy["rulesByFailureClass"].get(failure_class)
        if candidate is not None:
            rule = candidate
            matched_failure_class = failure_class
            break

    max_attempts = int(rule["maxAttempts"])
    retry = attempt < max_attempts
    return RetryDecision(
        attempt=attempt,
        retry=retry,
        max_attempts=max_attempts,
        backoff_class=str(rule["backoffClass"]),
        escalation_action=str(rule["escalationAction"]),
        rule_id=str(rule["ruleId"]),
        matched_failure_class=matched_failure_class,
        failure_classes=normalized,
    )
