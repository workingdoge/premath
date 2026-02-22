#!/usr/bin/env python3
"""
Execute doctrine-inf semantic boundary vectors.

These vectors validate law-level preserved/not-preserved boundary behavior.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Set

DEFAULT_FIXTURES = (
    Path(__file__).resolve().parents[2]
    / "tests"
    / "conformance"
    / "fixtures"
    / "doctrine-inf"
)


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


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item:
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item)
    return out


def canonical_set(values: List[str]) -> List[str]:
    return sorted(set(values))


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "doctrine-inf":
        raise ValueError("manifest.suiteId must be 'doctrine-inf'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def evaluate_boundary_case(case: Dict[str, Any]) -> Dict[str, Any]:
    artifacts = case.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ValueError("artifacts must be an object")

    registry = set(ensure_string_list(artifacts.get("doctrineRegistry", []), "artifacts.doctrineRegistry"))
    if not registry:
        raise ValueError("artifacts.doctrineRegistry must be non-empty")

    declares = artifacts.get("destinationDeclares")
    if not isinstance(declares, dict):
        raise ValueError("artifacts.destinationDeclares must be an object")

    preserved = set(ensure_string_list(declares.get("preserved", []), "artifacts.destinationDeclares.preserved"))
    not_preserved = set(
        ensure_string_list(declares.get("notPreserved", []), "artifacts.destinationDeclares.notPreserved")
    )
    edge_morphisms = ensure_string_list(artifacts.get("edgeMorphisms", []), "artifacts.edgeMorphisms")

    failure_classes: Set[str] = set()

    if preserved & not_preserved:
        failure_classes.add("doctrine_declaration_overlap")

    unknown_declaration = sorted((preserved | not_preserved).difference(registry))
    if unknown_declaration:
        failure_classes.add("doctrine_unknown_morphism")

    unknown_edge = sorted(set(edge_morphisms).difference(registry))
    if unknown_edge:
        failure_classes.add("doctrine_unknown_morphism")

    for morphism in edge_morphisms:
        if morphism in not_preserved:
            failure_classes.add("doctrine_boundary_not_preserved")
            continue
        if morphism not in preserved:
            failure_classes.add("doctrine_boundary_not_declared_preserved")

    if failure_classes:
        return {
            "result": "rejected",
            "failureClasses": sorted(failure_classes),
        }
    return {
        "result": "accepted",
        "failureClasses": [],
    }


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
            if case.get("suiteId") != "doctrine-inf":
                raise ValueError(f"{case_path}: suiteId must be 'doctrine-inf'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            got = evaluate_boundary_case(case)
            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(expect.get("expectedFailureClasses", []), f"{expect_path}: expectedFailureClasses")
            )

            got_failure_classes = canonical_set(got.get("failureClasses", []))
            if got.get("result") != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={got!r}"
                )

            print(f"[ok] doctrine-inf/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[doctrine-inf-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[doctrine-inf-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run doctrine-inf semantic boundary vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Doctrine-inf fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[doctrine-inf-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
