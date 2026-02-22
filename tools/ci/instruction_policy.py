#!/usr/bin/env python3
"""Shared instruction policy bindings and check-surface validation."""

from __future__ import annotations

import hashlib
import json
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping

from control_plane_contract import (
    INSTRUCTION_POLICY_DIGEST_PREFIX,
    INSTRUCTION_POLICY_KIND,
)

ROOT = Path(__file__).resolve().parents[2]
POLICY_DIR = ROOT / "policies" / "instruction"
POLICY_KIND = INSTRUCTION_POLICY_KIND
POLICY_DIGEST_PREFIX = INSTRUCTION_POLICY_DIGEST_PREFIX


class PolicyValidationError(ValueError):
    """Validation error with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        super().__init__(message)


def _ensure_non_empty_string(value: Any, label: str, failure_class: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise PolicyValidationError(failure_class, f"{label} must be a non-empty string")
    return value.strip()


def _canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _stable_hash(value: Any) -> str:
    return hashlib.sha256(_canonical_json(value).encode("utf-8")).hexdigest()


def _ensure_unique_string_list(value: Any, label: str, failure_class: str) -> List[str]:
    if not isinstance(value, list) or not value:
        raise PolicyValidationError(failure_class, f"{label} must be a non-empty list")
    out: List[str] = []
    for idx, item in enumerate(value):
        out.append(_ensure_non_empty_string(item, f"{label}[{idx}]", failure_class))
    deduped = sorted(set(out))
    if len(deduped) != len(out):
        raise PolicyValidationError(failure_class, f"{label} must not contain duplicates")
    return deduped


def compute_policy_digest(canonical_policy: Mapping[str, Any]) -> str:
    return POLICY_DIGEST_PREFIX + _stable_hash(canonical_policy)


def _canonicalize_policy(payload: Any, path: Path) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise PolicyValidationError(
            "instruction_policy_invalid_shape",
            f"{path}: policy artifact root must be an object",
        )

    schema = payload.get("schema")
    if schema != 1:
        raise PolicyValidationError(
            "instruction_policy_invalid_shape",
            f"{path}: schema must be 1",
        )
    policy_kind = _ensure_non_empty_string(
        payload.get("policyKind"),
        f"{path}: policyKind",
        "instruction_policy_invalid_shape",
    )
    if policy_kind != POLICY_KIND:
        raise PolicyValidationError(
            "instruction_policy_invalid_shape",
            f"{path}: policyKind must be {POLICY_KIND!r}",
        )
    policy_id = _ensure_non_empty_string(
        payload.get("policyId"),
        f"{path}: policyId",
        "instruction_policy_invalid_shape",
    )
    allowed_checks = _ensure_unique_string_list(
        payload.get("allowedChecks"),
        f"{path}: allowedChecks",
        "instruction_policy_invalid_shape",
    )
    allowed_normalizers = _ensure_unique_string_list(
        payload.get("allowedNormalizers"),
        f"{path}: allowedNormalizers",
        "instruction_policy_invalid_shape",
    )

    return {
        "schema": 1,
        "policyKind": POLICY_KIND,
        "policyId": policy_id,
        "allowedChecks": allowed_checks,
        "allowedNormalizers": allowed_normalizers,
    }


@lru_cache(maxsize=1)
def _load_policy_registry() -> Dict[str, Dict[str, Any]]:
    registry: Dict[str, Dict[str, Any]] = {}
    if not POLICY_DIR.exists() or not POLICY_DIR.is_dir():
        raise PolicyValidationError(
            "instruction_unknown_policy",
            f"policy registry not found at {POLICY_DIR}",
        )

    for path in sorted(POLICY_DIR.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        canonical = _canonicalize_policy(payload, path)
        declared_digest = _ensure_non_empty_string(
            payload.get("policyDigest"),
            f"{path}: policyDigest",
            "instruction_policy_invalid_shape",
        )
        if not declared_digest.startswith(POLICY_DIGEST_PREFIX):
            raise PolicyValidationError(
                "instruction_policy_digest_mismatch",
                f"{path}: policyDigest must start with {POLICY_DIGEST_PREFIX!r}",
            )
        computed_digest = compute_policy_digest(canonical)
        if declared_digest != computed_digest:
            raise PolicyValidationError(
                "instruction_policy_digest_mismatch",
                (
                    f"{path}: policyDigest mismatch "
                    f"(declared={declared_digest}, computed={computed_digest})"
                ),
            )
        if declared_digest in registry:
            raise PolicyValidationError(
                "instruction_policy_invalid_shape",
                f"{path}: duplicate policyDigest {declared_digest!r}",
            )
        canonical["policyDigest"] = declared_digest
        canonical["policyPath"] = str(path.relative_to(ROOT))
        registry[declared_digest] = canonical

    if not registry:
        raise PolicyValidationError(
            "instruction_unknown_policy",
            f"no policy artifacts found under {POLICY_DIR}",
        )
    return registry


def resolve_instruction_policy(policy_digest: str) -> Dict[str, Any]:
    digest = _ensure_non_empty_string(
        policy_digest,
        "policyDigest",
        "instruction_unknown_policy",
    )
    if not digest.startswith(POLICY_DIGEST_PREFIX):
        raise PolicyValidationError(
            "instruction_unknown_policy",
            (
                "policyDigest must be a canonical digest (pol1_...) referencing "
                "a registry artifact"
            ),
        )

    registry = _load_policy_registry()
    policy = registry.get(digest)
    if policy is None:
        raise PolicyValidationError(
            "instruction_unknown_policy",
            f"policyDigest {digest!r} is not registered",
        )
    return policy


def validate_requested_checks(
    policy_digest: str,
    requested_checks: Iterable[str],
    normalizer_id: str | None = None,
) -> Dict[str, Any]:
    policy = resolve_instruction_policy(policy_digest)
    allowed_checks = set(policy["allowedChecks"])
    requested = sorted(set(str(item).strip() for item in requested_checks))
    disallowed = sorted(check_id for check_id in requested if check_id not in allowed_checks)
    if disallowed:
        raise PolicyValidationError(
            "instruction_check_not_allowed",
            (
                "requestedChecks include check IDs not allowed by policyDigest "
                f"{policy_digest!r}: {disallowed}"
            ),
        )
    if normalizer_id is not None:
        normalizer = _ensure_non_empty_string(
            normalizer_id,
            "normalizerId",
            "instruction_normalizer_not_allowed",
        )
        if normalizer not in set(policy["allowedNormalizers"]):
            raise PolicyValidationError(
                "instruction_normalizer_not_allowed",
                (
                    "normalizerId is not allowed by policyDigest "
                    f"{policy_digest!r}: {normalizer!r}"
                ),
            )
    return policy


def validate_proposal_binding_matches_envelope(
    envelope_normalizer_id: str,
    envelope_policy_digest: str,
    proposal: Mapping[str, Any] | None,
) -> None:
    if proposal is None:
        return
    canonical = proposal.get("canonical")
    if not isinstance(canonical, dict):
        raise PolicyValidationError(
            "proposal_invalid_shape",
            "proposal canonical payload must be an object",
        )
    binding = canonical.get("binding")
    if not isinstance(binding, dict):
        raise PolicyValidationError(
            "proposal_unbound_policy",
            "proposal canonical binding must be an object",
        )

    proposal_normalizer_id = _ensure_non_empty_string(
        binding.get("normalizerId"),
        "proposal.binding.normalizerId",
        "proposal_unbound_policy",
    )
    proposal_policy_digest = _ensure_non_empty_string(
        binding.get("policyDigest"),
        "proposal.binding.policyDigest",
        "proposal_unbound_policy",
    )

    if proposal_normalizer_id != envelope_normalizer_id:
        raise PolicyValidationError(
            "proposal_binding_mismatch",
            (
                "proposal.binding.normalizerId must match instruction normalizerId "
                f"({proposal_normalizer_id!r} != {envelope_normalizer_id!r})"
            ),
        )
    if proposal_policy_digest != envelope_policy_digest:
        raise PolicyValidationError(
            "proposal_binding_mismatch",
            (
                "proposal.binding.policyDigest must match instruction policyDigest "
                f"({proposal_policy_digest!r} != {envelope_policy_digest!r})"
            ),
        )
