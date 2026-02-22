#!/usr/bin/env python3
"""Deterministic delta snapshot helpers for strict CI compare phases."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Dict, List

DELTA_SCHEMA = 1
DELTA_KIND = "ci.delta.v1"


def _non_empty_string(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    trimmed = value.strip()
    return trimmed or None


def compute_typed_core_projection_digest(
    authority_payload_digest: str,
    normalizer_id: str,
    policy_digest: str,
) -> str:
    h = hashlib.sha256()
    for part in (authority_payload_digest, normalizer_id, policy_digest):
        h.update(part.encode("utf-8"))
        h.update(b"\x00")
    return "ev1_" + h.hexdigest()


def default_delta_snapshot_path(out_dir: Path) -> Path:
    return out_dir / "latest-delta.json"


def make_delta_snapshot_payload(plan: Dict[str, Any]) -> Dict[str, Any]:
    projection_digest = _non_empty_string(plan.get("projectionDigest"))
    projection_policy = _non_empty_string(plan.get("projectionPolicy"))
    normalizer_id = _non_empty_string(plan.get("normalizerId")) or "normalizer.ci.required.v1"
    authority_payload_digest = projection_digest
    typed_core_projection_digest = (
        compute_typed_core_projection_digest(
            authority_payload_digest,
            normalizer_id,
            projection_policy,
        )
        if authority_payload_digest and projection_policy
        else None
    )
    return {
        "schema": DELTA_SCHEMA,
        "deltaKind": DELTA_KIND,
        "projectionPolicy": plan.get("projectionPolicy"),
        "projectionDigest": plan.get("projectionDigest"),
        "typedCoreProjectionDigest": typed_core_projection_digest,
        "authorityPayloadDigest": authority_payload_digest,
        "normalizerId": normalizer_id,
        "policyDigest": projection_policy,
        "requiredChecks": plan.get("requiredChecks"),
        "changedPaths": plan.get("changedPaths"),
        "deltaSource": plan.get("deltaSource"),
        "fromRef": plan.get("fromRef"),
        "toRef": plan.get("toRef"),
    }


def write_delta_snapshot(path: Path, payload: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)
        f.write("\n")


def load_delta_snapshot(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        payload = json.load(f)
    if not isinstance(payload, dict):
        raise ValueError(f"delta snapshot root must be object: {path}")
    return payload


def read_changed_paths(payload: Dict[str, Any], *, label: str = "changedPaths") -> List[str]:
    raw = payload.get(label)
    if not isinstance(raw, list):
        raise ValueError(f"delta snapshot {label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(raw):
        if not isinstance(item, str):
            raise ValueError(f"delta snapshot {label}[{idx}] must be a string")
        out.append(item)
    return out
