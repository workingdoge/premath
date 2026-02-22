#!/usr/bin/env python3
"""
Execute deterministic Witness-ID conformance vectors.

This suite validates stability/sensitivity requirements from draft/WITNESS-ID:
- stable subset only (schema/class/lawRef/tokenPath/context),
- deterministic computation over canonical key material,
- sensitivity to key-field changes.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "toy"))

from witness_id import witness_id  # type: ignore  # noqa: E402

DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "witness-id"


def load_json(path: Path) -> Dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
    except FileNotFoundError as exc:
        raise ValueError(f"missing file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json: {path} ({exc})") from exc
    if not isinstance(data, dict):
        raise ValueError(f"json root must be object: {path}")
    return data


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be a non-empty string")
    return value


def ensure_optional_string(value: Any, label: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label} must be null or non-empty string")
    return value


def ensure_optional_dict(value: Any, label: str) -> Dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be null or object")
    return value


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item:
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item)
    return out


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "witness-id":
        raise ValueError("manifest.suiteId must be 'witness-id'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def witness_key_material(case_path: Path, payload: Any, label: str) -> Tuple[str, str, str | None, Dict[str, Any] | None]:
    if not isinstance(payload, dict):
        raise ValueError(f"{case_path}: {label} must be an object")
    cls = ensure_string(payload.get("class"), f"{label}.class")
    law_ref = ensure_string(payload.get("lawRef"), f"{label}.lawRef")
    token_path = ensure_optional_string(payload.get("tokenPath"), f"{label}.tokenPath")
    context = ensure_optional_dict(payload.get("context"), f"{label}.context")
    return cls, law_ref, token_path, context


def compute_witness_id(
    cls: str,
    law_ref: str,
    token_path: str | None,
    context: Dict[str, Any] | None,
) -> str:
    return witness_id(
        cls=cls,
        law_ref=law_ref,
        token_path=token_path,
        context=context,
    )


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    errors: List[str] = []
    executed = 0

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "witness-id":
                raise ValueError(f"{case_path}: suiteId must be 'witness-id'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")
            if expect.get("result") != "accepted":
                raise ValueError(f"{expect_path}: result must be 'accepted'")

            artifacts = case.get("artifacts")
            if not isinstance(artifacts, dict):
                raise ValueError(f"{case_path}: artifacts must be an object")

            left = witness_key_material(case_path, artifacts.get("leftWitness"), "artifacts.leftWitness")
            right = witness_key_material(case_path, artifacts.get("rightWitness"), "artifacts.rightWitness")

            relation = ensure_string(artifacts.get("expectedRelation"), "artifacts.expectedRelation")
            if relation not in {"equal", "not_equal"}:
                raise ValueError(f"{case_path}: artifacts.expectedRelation must be 'equal' or 'not_equal'")

            expected_left = ensure_string(
                artifacts.get("expectedLeftWitnessId"),
                "artifacts.expectedLeftWitnessId",
            )
            expected_right = ensure_string(
                artifacts.get("expectedRightWitnessId"),
                "artifacts.expectedRightWitnessId",
            )

            left_id_a = compute_witness_id(*left)
            left_id_b = compute_witness_id(*left)
            right_id_a = compute_witness_id(*right)
            right_id_b = compute_witness_id(*right)

            if left_id_a != left_id_b:
                raise ValueError(f"{case_path}: left witness-id computation is non-deterministic")
            if right_id_a != right_id_b:
                raise ValueError(f"{case_path}: right witness-id computation is non-deterministic")
            if left_id_a != expected_left:
                raise ValueError(
                    f"{case_path}: expectedLeftWitnessId mismatch "
                    f"(expect={expected_left}, got={left_id_a})"
                )
            if right_id_a != expected_right:
                raise ValueError(
                    f"{case_path}: expectedRightWitnessId mismatch "
                    f"(expect={expected_right}, got={right_id_a})"
                )

            if relation == "equal" and left_id_a != right_id_a:
                raise ValueError(
                    f"{case_path}: expected equal witness IDs, got left={left_id_a}, right={right_id_a}"
                )
            if relation == "not_equal" and left_id_a == right_id_a:
                raise ValueError(
                    f"{case_path}: expected distinct witness IDs, got both={left_id_a}"
                )

            print(f"[ok] witness-id/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[witness-id-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[witness-id-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Witness-ID conformance vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Witness-ID fixture root (default: {DEFAULT_FIXTURES})",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[error] fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures}")
        return 2
    try:
        return run(fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[witness-id-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
