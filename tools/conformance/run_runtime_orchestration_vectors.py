#!/usr/bin/env python3
"""Run runtime orchestration conformance vectors."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

import check_runtime_orchestration

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "runtime-orchestration"


def load_json(path: Path) -> Dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ValueError(f"missing file: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid json: {path} ({exc})") from exc
    if not isinstance(payload, dict):
        raise ValueError(f"json root must be object: {path}")
    return payload


def ensure_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value.strip()


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    return out


def ensure_optional_string(value: Any, label: str) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string when provided")
    return value.strip()


def canonical_set(values: Sequence[str]) -> List[str]:
    return sorted(set(values))


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "runtime-orchestration":
        raise ValueError("manifest.suiteId must be 'runtime-orchestration'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    errors: List[str] = []
    executed = 0
    invariance_rows: Dict[str, List[Tuple[str, str, str, Tuple[str, ...]]]] = {}

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "runtime-orchestration":
                raise ValueError(f"{case_path}: suiteId must be 'runtime-orchestration'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")
            semantic_scenario_id = ensure_optional_string(
                case.get("semanticScenarioId"),
                f"{case_path}: semanticScenarioId",
            )
            profile = ensure_optional_string(case.get("profile"), f"{case_path}: profile")
            if vector_id.startswith("invariance/"):
                if semantic_scenario_id is None:
                    raise ValueError(
                        f"{case_path}: invariance vectors require semanticScenarioId"
                    )
                if profile is None:
                    raise ValueError(f"{case_path}: invariance vectors require profile")

            control_plane_contract = case.get("controlPlaneContract")
            if not isinstance(control_plane_contract, dict):
                raise ValueError(f"{case_path}: controlPlaneContract must be an object")
            doctrine_op_registry = case.get("doctrineOpRegistry")
            if not isinstance(doctrine_op_registry, dict):
                raise ValueError(f"{case_path}: doctrineOpRegistry must be an object")
            doctrine_site_input = case.get("doctrineSiteInput")
            if doctrine_site_input is not None and not isinstance(doctrine_site_input, dict):
                raise ValueError(f"{case_path}: doctrineSiteInput must be an object when provided")
            harness_runtime_text = ensure_string(
                case.get("harnessRuntimeText"),
                f"{case_path}: harnessRuntimeText",
            )

            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{expect_path}: expectedFailureClasses",
                )
            )

            output = check_runtime_orchestration.evaluate_runtime_orchestration(
                control_plane_contract=control_plane_contract,
                operation_registry=doctrine_op_registry,
                harness_runtime_text=harness_runtime_text,
                doctrine_site_input=doctrine_site_input,
            )
            got_result = ensure_string(output.get("result"), f"{vector_id}: output.result")
            got_failure_classes = canonical_set(
                ensure_string_list(
                    output.get("failureClasses", []),
                    f"{vector_id}: output.failureClasses",
                )
            )
            if got_result != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={{'result': {got_result!r}, 'failureClasses': {got_failure_classes!r}}}"
                )
            if semantic_scenario_id is not None:
                invariance_rows.setdefault(semantic_scenario_id, []).append(
                    (
                        vector_id,
                        profile or "default",
                        got_result,
                        tuple(got_failure_classes),
                    )
                )

            print(f"[ok] runtime-orchestration/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    for scenario_id in sorted(invariance_rows):
        rows = invariance_rows[scenario_id]
        if len(rows) < 2:
            errors.append(
                f"invariance scenario {scenario_id!r} has fewer than 2 vectors"
            )
            continue
        baseline_result = rows[0][2]
        baseline_failures = rows[0][3]
        for vector_id, profile, result, failure_classes in rows[1:]:
            if result != baseline_result or failure_classes != baseline_failures:
                errors.append(
                    "invariance mismatch for "
                    f"{scenario_id!r}: baseline=({baseline_result}, {list(baseline_failures)}) "
                    f"vs {vector_id}@{profile}=({result}, {list(failure_classes)})"
                )

    if errors:
        print(f"[runtime-orchestration-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[runtime-orchestration-run] OK "
        f"(vectors={executed}, invarianceScenarios={len(invariance_rows)})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run runtime orchestration conformance vectors."
    )
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Runtime orchestration fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[runtime-orchestration-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
