#!/usr/bin/env python3
"""Deterministic helpers for CI-emitted GateWitnessEnvelope artifacts."""

from __future__ import annotations

import hashlib
import json
import re
from typing import Any, Dict, Iterable, Optional


_SAFE_CHECK_ID_RE = re.compile(r"[^A-Za-z0-9._-]+")


def _canonical_json_bytes(value: Any) -> bytes:
    if value is None:
        return b"null"
    if isinstance(value, bool):
        return b"true" if value else b"false"
    if isinstance(value, int):
        return str(value).encode("utf-8")
    if isinstance(value, float):
        return format(value).encode("utf-8")
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    if isinstance(value, list):
        out = bytearray(b"[")
        for idx, item in enumerate(value):
            if idx:
                out.extend(b",")
            out.extend(_canonical_json_bytes(item))
        out.extend(b"]")
        return bytes(out)
    if isinstance(value, dict):
        out = bytearray(b"{")
        for idx, key in enumerate(sorted(value.keys())):
            if idx:
                out.extend(b",")
            out.extend(
                json.dumps(str(key), ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            )
            out.extend(b":")
            out.extend(_canonical_json_bytes(value[key]))
        out.extend(b"}")
        return bytes(out)
    raise TypeError(f"unsupported value type for canonical json: {type(value).__name__}")


def stable_sha256(value: Any) -> str:
    return hashlib.sha256(_canonical_json_bytes(value)).hexdigest()


def _base32hex_lower_no_pad(data: bytes) -> str:
    alphabet = "0123456789abcdefghijklmnopqrstuv"
    bits = 0
    num_bits = 0
    out: list[str] = []

    for byte in data:
        bits = (bits << 8) | byte
        num_bits += 8
        while num_bits >= 5:
            num_bits -= 5
            idx = (bits >> num_bits) & 0x1F
            out.append(alphabet[idx])

    if num_bits > 0:
        idx = (bits << (5 - num_bits)) & 0x1F
        out.append(alphabet[idx])

    return "".join(out)


def compute_witness_id(
    class_name: str,
    law_ref: str,
    token_path: Optional[str],
    context: Optional[Dict[str, Any]],
) -> str:
    key = {
        "schema": 1,
        "class": class_name,
        "context": context,
        "lawRef": law_ref,
        "tokenPath": token_path,
    }
    digest = hashlib.sha256(_canonical_json_bytes(key)).digest()
    return f"w1_{_base32hex_lower_no_pad(digest)}"


def compute_intent_id(
    intent_kind: str,
    target_scope: str,
    requested_outcomes: Iterable[str],
    constraints: Optional[Dict[str, Any]] = None,
) -> str:
    spec: Dict[str, Any] = {
        "intentKind": intent_kind,
        "targetScope": target_scope,
        "requestedOutcomes": sorted(set(requested_outcomes)),
    }
    if constraints is not None:
        spec["constraints"] = constraints
    return f"intent1_{stable_sha256(spec)}"


def compute_run_id(identity: Dict[str, Any], include_cover_strategy_digest: bool = False) -> str:
    material = dict(identity)
    if not include_cover_strategy_digest:
        material.pop("coverStrategyDigest", None)
    return f"run1_{stable_sha256(material)}"


def sanitize_check_id(check_id: str) -> str:
    sanitized = _SAFE_CHECK_ID_RE.sub("_", check_id.strip())
    sanitized = sanitized.strip("._")
    return sanitized or "check"


def make_gate_witness_envelope(
    check_id: str,
    exit_code: int,
    projection_digest: str,
    policy_digest: str,
    ctx_ref: str,
    data_head_ref: str,
) -> Dict[str, Any]:
    context_id = f"ctx.ci.required.{projection_digest}"
    intent_id = compute_intent_id(
        intent_kind="ci_required_check",
        target_scope=f"check:{check_id}",
        requested_outcomes=["gate_witness_envelope"],
    )
    identity = {
        "worldId": "world.ci.required",
        "unitId": f"unit.ci.check.{check_id}",
        "parentUnitId": f"unit.ci.projection.{projection_digest}",
        "contextId": context_id,
        "intentId": intent_id,
        "coverId": f"cover.ci.required.{projection_digest}",
        "ctxRef": ctx_ref,
        "dataHeadRef": data_head_ref,
        "adapterId": "adapter.ci.runner",
        "adapterVersion": "1",
        "normalizerId": "normalizer.ci.required.v1",
        "policyDigest": policy_digest,
    }
    run_id = compute_run_id(identity)

    failures = []
    result = "accepted"
    if exit_code != 0:
        result = "rejected"
        token_path = f"ci/check/{check_id}"
        context = {
            "checkId": check_id,
            "exitCode": exit_code,
            "projectionDigest": projection_digest,
        }
        failures.append(
            {
                "witnessId": compute_witness_id(
                    class_name="descent_failure",
                    law_ref="GATE-3.3",
                    token_path=token_path,
                    context=context,
                ),
                "class": "descent_failure",
                "lawRef": "GATE-3.3",
                "message": f"ci required check '{check_id}' failed (exitCode={exit_code})",
                "context": context,
                "tokenPath": token_path,
                "details": {
                    "phase": "run_gate",
                    "responsibleComponent": "gate_execution_plane",
                },
            }
        )

    return {
        "witnessSchema": 1,
        "witnessKind": "gate",
        "runId": run_id,
        "worldId": identity["worldId"],
        "contextId": identity["contextId"],
        "intentId": identity["intentId"],
        "adapterId": identity["adapterId"],
        "adapterVersion": identity["adapterVersion"],
        "ctxRef": identity["ctxRef"],
        "dataHeadRef": identity["dataHeadRef"],
        "normalizerId": identity["normalizerId"],
        "policyDigest": identity["policyDigest"],
        "result": result,
        "failures": failures,
    }
