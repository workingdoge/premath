#!/usr/bin/env python3
"""Deterministic change projection for CI closure checks."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Sequence, Set, Tuple

from provider_env import resolve_premath_ci_refs

PROJECTION_SCHEMA = 1
CONTROL_PLANE_CONTRACT_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "CONTROL-PLANE-CONTRACT.json"
)
CONTROL_PLANE_CONTRACT_KIND = "premath.control_plane.contract.v1"


def _load_control_plane_projection_contract() -> Dict[str, Any]:
    try:
        payload = json.loads(CONTROL_PLANE_CONTRACT_PATH.read_text(encoding="utf-8"))
    except OSError as exc:
        raise RuntimeError(
            f"failed to read control-plane contract: {CONTROL_PLANE_CONTRACT_PATH}: {exc}"
        ) from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"invalid json in control-plane contract: {CONTROL_PLANE_CONTRACT_PATH}: {exc}"
        ) from exc

    if not isinstance(payload, dict):
        raise RuntimeError("control-plane contract root must be an object")
    if payload.get("schema") != 1:
        raise RuntimeError("control-plane contract schema must be 1")
    if payload.get("contractKind") != CONTROL_PLANE_CONTRACT_KIND:
        raise RuntimeError(
            f"control-plane contract kind must be {CONTROL_PLANE_CONTRACT_KIND!r}"
        )

    required = payload.get("requiredGateProjection")
    if not isinstance(required, dict):
        raise RuntimeError("control-plane contract requiredGateProjection must be an object")

    projection_policy = required.get("projectionPolicy")
    if not isinstance(projection_policy, str) or not projection_policy.strip():
        raise RuntimeError("control-plane contract projectionPolicy must be a non-empty string")

    check_ids = required.get("checkIds")
    if not isinstance(check_ids, dict):
        raise RuntimeError("control-plane contract checkIds must be an object")

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
    parsed_ids: Dict[str, str] = {}
    for key in required_check_id_keys:
        value = check_ids.get(key)
        if not isinstance(value, str) or not value.strip():
            raise RuntimeError(f"control-plane contract checkIds.{key} must be a non-empty string")
        parsed_ids[key] = value.strip()
    if len(set(parsed_ids.values())) != len(parsed_ids):
        raise RuntimeError("control-plane contract checkIds must not contain duplicate values")

    check_order = required.get("checkOrder")
    if not isinstance(check_order, list) or not check_order:
        raise RuntimeError("control-plane contract checkOrder must be a non-empty list")
    parsed_order: List[str] = []
    for idx, item in enumerate(check_order):
        if not isinstance(item, str) or not item.strip():
            raise RuntimeError(f"control-plane contract checkOrder[{idx}] must be a non-empty string")
        parsed_order.append(item.strip())

    known_ids = set(parsed_ids.values())
    order_set = set(parsed_order)
    if len(order_set) != len(parsed_order):
        raise RuntimeError("control-plane contract checkOrder must not contain duplicates")
    if order_set != known_ids:
        raise RuntimeError("control-plane contract checkOrder must cover exactly checkIds values")

    return {
        "projectionPolicy": projection_policy.strip(),
        "checkIds": parsed_ids,
        "checkOrder": parsed_order,
    }


_CONTROL_PLANE_PROJECTION = _load_control_plane_projection_contract()
PROJECTION_POLICY = _CONTROL_PLANE_PROJECTION["projectionPolicy"]

CHECK_BASELINE = _CONTROL_PLANE_PROJECTION["checkIds"]["baseline"]
CHECK_BUILD = _CONTROL_PLANE_PROJECTION["checkIds"]["build"]
CHECK_TEST = _CONTROL_PLANE_PROJECTION["checkIds"]["test"]
CHECK_TEST_TOY = _CONTROL_PLANE_PROJECTION["checkIds"]["testToy"]
CHECK_TEST_KCIR_TOY = _CONTROL_PLANE_PROJECTION["checkIds"]["testKcirToy"]
CHECK_CONFORMANCE = _CONTROL_PLANE_PROJECTION["checkIds"]["conformanceCheck"]
CHECK_CONFORMANCE_RUN = _CONTROL_PLANE_PROJECTION["checkIds"]["conformanceRun"]
CHECK_DOCTRINE = _CONTROL_PLANE_PROJECTION["checkIds"]["doctrineCheck"]

CHECK_ORDER: Sequence[str] = tuple(_CONTROL_PLANE_PROJECTION["checkOrder"])

DOC_FILE_NAMES: Set[str] = {
    "AGENTS.md",
    "COMMITMENT.md",
    "README.md",
    "RELEASE_NOTES.md",
    "LICENSE",
}

DOC_EXTENSIONS: Sequence[str] = (
    ".md",
    ".mdx",
    ".rst",
    ".txt",
    ".adoc",
)

SEMANTIC_BASELINE_PREFIXES: Sequence[str] = (
    ".github/workflows/",
    "tools/ci/",
    "tools/infra/terraform/",
    "infra/terraform/",
)

SEMANTIC_BASELINE_EXACT: Set[str] = {
    ".mise.toml",
    "hk.pkl",
    "pitchfork.toml",
}

RUST_PREFIXES: Sequence[str] = ("crates/",)
RUST_EXACT: Set[str] = {"Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml"}
KERNEL_PREFIX = "crates/premath-kernel/"

CONFORMANCE_PREFIXES: Sequence[str] = (
    "tests/conformance/",
    "tests/toy/fixtures/",
    "tests/kcir_toy/fixtures/",
    "tools/conformance/",
    "tools/toy/",
    "tools/kcir_toy/",
)

RAW_DOC_TRIGGER_PREFIXES: Sequence[str] = (
    "specs/premath/raw/",
    "tests/conformance/",
)

DOCTRINE_DOC_PREFIXES: Sequence[str] = (
    "specs/premath/draft/",
    "specs/premath/raw/",
    "specs/process/",
)


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


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_hash(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


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


def _starts_with_any(path: str, prefixes: Sequence[str]) -> bool:
    return any(path.startswith(prefix) for prefix in prefixes)


def is_doc_like_path(path: str) -> bool:
    if path in DOC_FILE_NAMES:
        return True
    if path.startswith("docs/") or path.startswith("specs/"):
        return True
    return any(path.endswith(ext) for ext in DOC_EXTENSIONS)


def is_semantic_baseline_path(path: str) -> bool:
    return path in SEMANTIC_BASELINE_EXACT or _starts_with_any(path, SEMANTIC_BASELINE_PREFIXES)


def is_rust_path(path: str) -> bool:
    return path in RUST_EXACT or _starts_with_any(path, RUST_PREFIXES)


def is_conformance_path(path: str) -> bool:
    return _starts_with_any(path, CONFORMANCE_PREFIXES)


def is_known_projection_surface(path: str) -> bool:
    return (
        is_doc_like_path(path)
        or is_semantic_baseline_path(path)
        or is_rust_path(path)
        or is_conformance_path(path)
    )


def project_required_checks(changed_paths: Sequence[str]) -> ProjectionResult:
    paths = normalize_paths(changed_paths)

    reasons: Set[str] = set()
    checks: Set[str] = set()

    if not paths:
        reasons.add("empty_delta_fallback_baseline")
        checks.add(CHECK_BASELINE)
        ordered = [CHECK_BASELINE]
        projection_digest = "proj1_" + stable_hash(
            {
                "projectionPolicy": PROJECTION_POLICY,
                "changedPaths": paths,
                "requiredChecks": ordered,
            }
        )
        return ProjectionResult(
            changed_paths=paths,
            required_checks=ordered,
            docs_only=True,
            reasons=sorted(reasons),
            projection_digest=projection_digest,
        )

    docs_only = all(is_doc_like_path(path) for path in paths)

    if any(is_semantic_baseline_path(path) for path in paths):
        reasons.add("semantic_surface_changed")
        checks.add(CHECK_BASELINE)

    if checks and CHECK_BASELINE in checks:
        ordered = [CHECK_BASELINE]
        projection_digest = "proj1_" + stable_hash(
            {
                "projectionPolicy": PROJECTION_POLICY,
                "changedPaths": paths,
                "requiredChecks": ordered,
            }
        )
        return ProjectionResult(
            changed_paths=paths,
            required_checks=ordered,
            docs_only=docs_only,
            reasons=sorted(reasons),
            projection_digest=projection_digest,
        )

    rust_touched = any(is_rust_path(path) for path in paths)
    if rust_touched:
        reasons.add("rust_surface_changed")
        checks.add(CHECK_BUILD)
        checks.add(CHECK_TEST)

    kernel_touched = any(path.startswith(KERNEL_PREFIX) for path in paths)
    if kernel_touched:
        reasons.add("kernel_surface_changed")
        checks.add(CHECK_TEST_TOY)
        checks.add(CHECK_TEST_KCIR_TOY)

    conformance_touched = any(is_conformance_path(path) for path in paths)
    if conformance_touched:
        reasons.add("conformance_surface_changed")
        checks.add(CHECK_CONFORMANCE)
        checks.add(CHECK_CONFORMANCE_RUN)
        checks.add(CHECK_TEST_TOY)
        checks.add(CHECK_TEST_KCIR_TOY)

    unknown_non_doc_paths = [
        path for path in paths if (not is_doc_like_path(path) and not is_known_projection_surface(path))
    ]
    if unknown_non_doc_paths:
        reasons.add("non_doc_unknown_surface_fallback_baseline")
        checks.add(CHECK_BASELINE)

    if docs_only:
        raw_docs_touched = any(_starts_with_any(path, RAW_DOC_TRIGGER_PREFIXES) for path in paths)
        doctrine_docs_touched = any(_starts_with_any(path, DOCTRINE_DOC_PREFIXES) for path in paths)
        if raw_docs_touched:
            reasons.add("docs_only_raw_or_conformance_touched")
            checks.add(CHECK_CONFORMANCE)
        if doctrine_docs_touched:
            reasons.add("docs_only_doctrine_surface_touched")
            checks.add(CHECK_DOCTRINE)

    if not checks and not docs_only:
        reasons.add("non_doc_unknown_surface_fallback_baseline")
        checks.add(CHECK_BASELINE)

    if CHECK_BASELINE in checks:
        ordered = [CHECK_BASELINE]
    else:
        ordered = [check_id for check_id in CHECK_ORDER if check_id in checks]

    projection_digest = "proj1_" + stable_hash(
        {
            "projectionPolicy": PROJECTION_POLICY,
            "changedPaths": paths,
            "requiredChecks": ordered,
        }
    )

    return ProjectionResult(
        changed_paths=paths,
        required_checks=ordered,
        docs_only=docs_only,
        reasons=sorted(reasons),
        projection_digest=projection_digest,
    )


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
