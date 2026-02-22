#!/usr/bin/env python3
"""Deterministic change projection for CI closure checks."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple

from control_plane_contract import REQUIRED_PROJECTION_POLICY
from required_delta_client import RequiredDeltaError, run_required_delta
from required_projection_client import (
    RequiredProjectionError,
    run_required_projection,
)

PROJECTION_SCHEMA = 1
PROJECTION_POLICY = REQUIRED_PROJECTION_POLICY


@dataclass(frozen=True)
class ProjectionResult:
    changed_paths: List[str]
    required_checks: List[str]
    docs_only: bool
    reasons: List[str]
    projection_digest: str


@dataclass(frozen=True)
class ChangeDetectionResult:
    changed_paths: List[str]
    source: str
    from_ref: Optional[str]
    to_ref: str


def _normalize_path(path: str) -> str:
    normalized = path.strip().replace("\\", "/")
    while normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized


def normalize_paths(paths: Iterable[str]) -> List[str]:
    out: Set[str] = set()
    for raw in paths:
        normalized = _normalize_path(str(raw))
        if normalized:
            out.add(normalized)
    return sorted(out)


_PROJECTION_CACHE: Dict[Tuple[str, ...], ProjectionResult] = {}


def project_required_checks(changed_paths: Sequence[str]) -> ProjectionResult:
    normalized_paths = normalize_paths(changed_paths)
    cache_key = tuple(normalized_paths)
    cached = _PROJECTION_CACHE.get(cache_key)
    if cached is not None:
        return cached

    root = Path(__file__).resolve().parents[2]
    request = {"changedPaths": normalized_paths}
    try:
        payload = run_required_projection(root, request)
    except RequiredProjectionError as exc:
        raise ValueError(f"{exc.failure_class}: {exc.reason}") from exc

    projection_policy = payload.get("projectionPolicy")
    if projection_policy != PROJECTION_POLICY:
        raise ValueError(
            "projection policy mismatch "
            f"(expected={PROJECTION_POLICY!r}, actual={projection_policy!r})"
        )

    result = ProjectionResult(
        changed_paths=payload.get("changedPaths", []),
        required_checks=payload.get("requiredChecks", []),
        docs_only=bool(payload.get("docsOnly")),
        reasons=payload.get("reasons", []),
        projection_digest=str(payload.get("projectionDigest", "")),
    )
    _PROJECTION_CACHE[cache_key] = result
    return result


def detect_changed_paths(
    repo_root: Path,
    from_ref: Optional[str] = None,
    to_ref: Optional[str] = None,
) -> ChangeDetectionResult:
    request: Dict[str, Any] = {"repoRoot": str(repo_root)}
    if from_ref is not None:
        request["fromRef"] = from_ref
    if to_ref is not None:
        request["toRef"] = to_ref

    try:
        payload = run_required_delta(repo_root, request)
    except RequiredDeltaError as exc:
        raise ValueError(f"{exc.failure_class}: {exc.reason}") from exc

    return ChangeDetectionResult(
        changed_paths=normalize_paths(payload.get("changedPaths", [])),
        source=str(payload.get("source", "none")),
        from_ref=payload.get("fromRef"),
        to_ref=str(payload.get("toRef", "HEAD")),
    )


def projection_plan_payload(
    projection: ProjectionResult,
    source: str,
    from_ref: Optional[str],
    to_ref: str,
) -> Dict[str, Any]:
    return {
        "schema": PROJECTION_SCHEMA,
        "projectionPolicy": PROJECTION_POLICY,
        "projectionDigest": projection.projection_digest,
        "changedPaths": projection.changed_paths,
        "requiredChecks": projection.required_checks,
        "docsOnly": projection.docs_only,
        "reasons": projection.reasons,
        "deltaSource": source,
        "fromRef": from_ref,
        "toRef": to_ref,
    }
