#!/usr/bin/env python3
"""Provider-neutral CI environment mapping and reference resolution helpers."""

from __future__ import annotations

from typing import Dict, Mapping, Optional, Tuple


def _clean(value: Optional[str]) -> Optional[str]:
    if value is None:
        return None
    trimmed = value.strip()
    return trimmed or None


def map_github_to_premath_env(env: Mapping[str, str]) -> Dict[str, str]:
    """Map GitHub CI variables to provider-neutral Premath CI refs."""
    out: Dict[str, str] = {}

    base_ref = _clean(env.get("PREMATH_CI_BASE_REF"))
    if base_ref is None:
        github_base = _clean(env.get("GITHUB_BASE_REF"))
        if github_base is not None:
            base_ref = f"origin/{github_base}"
    if base_ref is not None:
        out["PREMATH_CI_BASE_REF"] = base_ref

    head_ref = _clean(env.get("PREMATH_CI_HEAD_REF"))
    if head_ref is None:
        head_ref = _clean(env.get("GITHUB_SHA"))
    if head_ref is not None:
        out["PREMATH_CI_HEAD_REF"] = head_ref

    return out


def resolve_premath_ci_refs(env: Mapping[str, str]) -> Tuple[Optional[str], str]:
    """Resolve canonical Premath CI refs from env with deterministic fallbacks."""
    base_ref = _clean(env.get("PREMATH_CI_BASE_REF"))
    head_ref = _clean(env.get("PREMATH_CI_HEAD_REF")) or "HEAD"
    return (base_ref, head_ref)
