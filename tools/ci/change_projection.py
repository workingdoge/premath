#!/usr/bin/env python3
"""Deterministic change projection for CI closure checks."""

from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple

from control_plane_contract import REQUIRED_PROJECTION_POLICY
from provider_env import resolve_premath_ci_refs
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


def _run_git(repo_root: Path, args: Sequence[str]) -> Optional[str]:
    cmd = ["git", *args]
    completed = subprocess.run(
        cmd,
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return None
    return completed.stdout.strip()


def _ref_exists(repo_root: Path, ref: str) -> bool:
    return _run_git(repo_root, ["rev-parse", "--verify", "--quiet", ref]) is not None


def detect_default_base_ref(repo_root: Path) -> Optional[str]:
    env_base, _head_ref = resolve_premath_ci_refs(os.environ)
    candidates: List[str] = []
    if env_base:
        if env_base.startswith("origin/"):
            candidates.extend([env_base, env_base[len("origin/"):]])
        else:
            candidates.extend([env_base, f"origin/{env_base}"])

    candidates.extend([
        "origin/main",
        "main",
        "origin/master",
        "master",
        "HEAD~1",
    ])

    for candidate in candidates:
        if _ref_exists(repo_root, candidate):
            return candidate
    return None


def detect_default_head_ref() -> str:
    _base_ref, head_ref = resolve_premath_ci_refs(os.environ)
    return head_ref


def detect_changed_paths(
    repo_root: Path,
    from_ref: Optional[str] = None,
    to_ref: Optional[str] = None,
) -> ChangeDetectionResult:
    head_ref = to_ref or detect_default_head_ref()
    base_ref = from_ref or detect_default_base_ref(repo_root)
    paths: List[str] = []
    source = "none"

    if base_ref is not None:
        output = _run_git(
            repo_root,
            ["diff", "--name-only", "--diff-filter=ACMR", f"{base_ref}...{head_ref}"],
        )
        if output is not None:
            paths.extend(line for line in output.splitlines() if line.strip())
            source = "git_diff"
        else:
            source = "diff_failed"

    # Include local staged/worktree changes so local gating does not miss
    # uncommitted deltas.
    staged = _run_git(repo_root, ["diff", "--name-only", "--cached", "--diff-filter=ACMR"])
    if staged:
        paths.extend(line for line in staged.splitlines() if line.strip())
        source = "workspace" if source == "none" else f"{source}+workspace"

    worktree = _run_git(repo_root, ["diff", "--name-only", "--diff-filter=ACMR"])
    if worktree:
        paths.extend(line for line in worktree.splitlines() if line.strip())
        source = "workspace" if source == "none" else f"{source}+workspace"

    return ChangeDetectionResult(
        changed_paths=normalize_paths(paths),
        source=source,
        from_ref=base_ref,
        to_ref=head_ref,
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
