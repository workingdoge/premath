#!/usr/bin/env python3
"""Validate instruction envelope schema/shape before execution."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, List

from instruction_policy import (
    PolicyValidationError,
    validate_proposal_binding_matches_envelope,
    validate_requested_checks,
)
from instruction_proposal import ProposalValidationError, validate_instruction_proposal


DEFAULT_GLOBS = (
    "instructions/*.json",
    "tests/ci/fixtures/instructions/*.json",
)


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_nonempty_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label} must be a non-empty list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    if len(set(out)) != len(out):
        raise ValueError(f"{label} must not contain duplicates")
    return out


def validate_envelope(path: Path, payload: Dict[str, Any]) -> None:
    stem = path.stem
    if path.suffix != ".json":
        raise ValueError("filename must end with .json")
    if "-" not in stem:
        raise ValueError("filename stem must follow <ts>-<id> format")

    schema = payload.get("schema")
    if not isinstance(schema, int) or schema <= 0:
        raise ValueError("schema must be a positive integer")

    ensure_string(payload.get("intent"), "intent")
    normalizer_id = ensure_string(payload.get("normalizerId"), "normalizerId")
    policy_digest = ensure_string(payload.get("policyDigest"), "policyDigest")
    if "scope" not in payload:
        raise ValueError("scope is required")
    scope = payload.get("scope")
    if scope in (None, ""):
        raise ValueError("scope must be non-empty")
    requested_checks = ensure_nonempty_string_list(payload.get("requestedChecks"), "requestedChecks")
    try:
        validate_requested_checks(policy_digest, requested_checks, normalizer_id=normalizer_id)
    except PolicyValidationError as exc:
        raise ValueError(f"{exc.failure_class}: {exc}") from exc

    instruction_type = payload.get("instructionType")
    if instruction_type is not None:
        ensure_string(instruction_type, "instructionType")

    typing_policy = payload.get("typingPolicy")
    if typing_policy is not None:
        if not isinstance(typing_policy, dict):
            raise ValueError("typingPolicy must be an object when provided")
        allow_unknown = typing_policy.get("allowUnknown")
        if allow_unknown is not None and not isinstance(allow_unknown, bool):
            raise ValueError("typingPolicy.allowUnknown must be a boolean when provided")

    capability_claims = payload.get("capabilityClaims")
    if capability_claims is not None:
        if not isinstance(capability_claims, list):
            raise ValueError("capabilityClaims must be a list when provided")
        normalized_claims: List[str] = []
        for idx, claim in enumerate(capability_claims):
            if not isinstance(claim, str) or not claim.strip():
                raise ValueError(f"capabilityClaims[{idx}] must be a non-empty string")
            normalized_claims.append(claim.strip())
        if len(set(normalized_claims)) != len(normalized_claims):
            raise ValueError("capabilityClaims must not contain duplicates")

    try:
        proposal = validate_instruction_proposal(payload)
        validate_proposal_binding_matches_envelope(normalizer_id, policy_digest, proposal)
    except ProposalValidationError as exc:
        raise ValueError(f"{exc.failure_class}: {exc}") from exc
    except PolicyValidationError as exc:
        raise ValueError(f"{exc.failure_class}: {exc}") from exc


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check instruction envelope schema/shape.")
    parser.add_argument(
        "paths",
        nargs="*",
        help="Instruction files to validate. If omitted, validates default instruction globs.",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    return parser.parse_args()


def resolve_paths(repo_root: Path, args_paths: List[str]) -> List[Path]:
    files: List[Path] = []
    if args_paths:
        for raw in args_paths:
            path = Path(raw)
            if not path.is_absolute():
                path = (repo_root / path).resolve()
            files.append(path)
    else:
        for pattern in DEFAULT_GLOBS:
            files.extend(sorted((repo_root).glob(pattern)))
    return files


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()
    files = resolve_paths(root, args.paths)

    if not files:
        print("[instruction-check] FAIL (no instruction envelopes found)")
        return 1

    errors: List[str] = []
    checked = 0
    for path in files:
        checked += 1
        if not path.exists() or not path.is_file():
            errors.append(f"{path}: file not found")
            continue
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(payload, dict):
                raise ValueError("root must be a JSON object")
            validate_envelope(path, payload)
        except Exception as exc:
            errors.append(f"{path}: {exc}")

    if errors:
        print(f"[instruction-check] FAIL (checked={checked}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[instruction-check] OK (checked={checked})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
