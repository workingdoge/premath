#!/usr/bin/env python3
"""
Execute canonical cross-model kernel profile vectors.

This suite compares stable semantic Gate outcomes for the same scenario across:
- semantic toy fixtures (`tests/toy/fixtures`)
- KCIR toy fixtures (`tests/kcir_toy/fixtures`)
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "toy"))
sys.path.insert(0, str(ROOT / "tools" / "kcir_toy"))

from gate_from_fixture import run_gate_from_fixture  # type: ignore  # noqa: E402
from toy_gate_check import run_case as run_toy_case  # type: ignore  # noqa: E402

DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "kernel-profile"
DEFAULT_TOY_FIXTURES = ROOT / "tests" / "toy" / "fixtures"
DEFAULT_KCIR_FIXTURES = ROOT / "tests" / "kcir_toy" / "fixtures"


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


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "kernel-profile":
        raise ValueError("manifest.suiteId must be 'kernel-profile'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def stable_projection(payload: Any, label: str) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ValueError(f"{label} must be an object")
    result = ensure_string(payload.get("result"), f"{label}.result")
    if result not in {"accepted", "rejected"}:
        raise ValueError(f"{label}.result must be 'accepted' or 'rejected'")

    failures = payload.get("failures", [])
    if not isinstance(failures, list):
        raise ValueError(f"{label}.failures must be a list")
    failure_view: List[Dict[str, str]] = []
    for idx, failure in enumerate(failures):
        if not isinstance(failure, dict):
            raise ValueError(f"{label}.failures[{idx}] must be an object")
        failure_view.append(
            {
                "class": ensure_string(failure.get("class"), f"{label}.failures[{idx}].class"),
                "lawRef": ensure_string(failure.get("lawRef"), f"{label}.failures[{idx}].lawRef"),
                "witnessId": ensure_string(failure.get("witnessId"), f"{label}.failures[{idx}].witnessId"),
            }
        )
    failure_view.sort(key=lambda row: (row["class"], row["lawRef"], row["witnessId"]))
    return {"result": result, "failures": failure_view}


def run(fixtures: Path, toy_fixtures: Path, kcir_fixtures: Path) -> int:
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
            if case.get("suiteId") != "kernel-profile":
                raise ValueError(f"{case_path}: suiteId must be 'kernel-profile'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")
            if expect.get("result") != "accepted":
                raise ValueError(f"{expect_path}: result must be 'accepted'")

            artifacts = case.get("artifacts")
            if not isinstance(artifacts, dict):
                raise ValueError(f"{case_path}: artifacts must be an object")
            scenario_id = ensure_string(artifacts.get("scenarioId"), "artifacts.scenarioId")

            expected = stable_projection(expect.get("expectedOutcome"), f"{expect_path}: expectedOutcome")

            toy_case_path = toy_fixtures / scenario_id / "case.json"
            toy_case = load_json(toy_case_path)
            toy_got = stable_projection(run_toy_case(toy_case), f"{case_path}: toyResult")

            kcir_fixture_path = kcir_fixtures / scenario_id
            if not kcir_fixture_path.is_dir():
                raise ValueError(f"{case_path}: missing KCIR fixture directory: {kcir_fixture_path}")
            kcir_got = stable_projection(
                run_gate_from_fixture(str(kcir_fixture_path)),
                f"{case_path}: kcirResult",
            )

            if toy_got != kcir_got:
                raise ValueError(
                    f"cross-model mismatch for {scenario_id}\n"
                    f"toy={json.dumps(toy_got, sort_keys=True)}\n"
                    f"kcir={json.dumps(kcir_got, sort_keys=True)}"
                )
            if toy_got != expected:
                raise ValueError(
                    f"expect/toy mismatch for {scenario_id}\n"
                    f"expect={json.dumps(expected, sort_keys=True)}\n"
                    f"toy={json.dumps(toy_got, sort_keys=True)}"
                )

            print(f"[ok] kernel-profile/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[kernel-profile-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[kernel-profile-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run cross-model kernel profile vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Kernel profile fixture root (default: {DEFAULT_FIXTURES})",
    )
    parser.add_argument(
        "--toy-fixtures",
        type=Path,
        default=DEFAULT_TOY_FIXTURES,
        help=f"Semantic toy fixture root (default: {DEFAULT_TOY_FIXTURES})",
    )
    parser.add_argument(
        "--kcir-fixtures",
        type=Path,
        default=DEFAULT_KCIR_FIXTURES,
        help=f"KCIR toy fixture root (default: {DEFAULT_KCIR_FIXTURES})",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    toy_fixtures = args.toy_fixtures
    kcir_fixtures = args.kcir_fixtures
    if not fixtures.exists():
        print(f"[error] fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures}")
        return 2
    if not toy_fixtures.is_dir():
        print(f"[error] toy fixtures path is not a directory: {toy_fixtures}")
        return 2
    if not kcir_fixtures.is_dir():
        print(f"[error] kcir fixtures path is not a directory: {kcir_fixtures}")
        return 2
    try:
        return run(fixtures, toy_fixtures, kcir_fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[kernel-profile-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
