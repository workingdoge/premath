#!/usr/bin/env python3
"""Run KCIR identity projection vectors for statement-index rows."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

import check_statement_index

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "statement-kcir"
FAILURE_CLASS_INPUT_INVALID = "statement_kcir_input_invalid"


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run statement KCIR reference vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Fixture root (default: {DEFAULT_FIXTURES})",
    )
    return parser.parse_args(argv)


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


def ensure_optional_string(value: Any, label: str) -> str | None:
    if value is None:
        return None
    return ensure_string(value, label)


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    return out


def canonical(values: Sequence[str]) -> List[str]:
    return sorted(set(values))


def evaluate_case(statement_core: Any) -> Dict[str, Any]:
    failures: List[str] = []
    errors: List[str] = []
    if not isinstance(statement_core, dict):
        failures.append(FAILURE_CLASS_INPUT_INVALID)
        errors.append("statementCore must be an object")
        return {
            "result": "rejected",
            "failureClasses": canonical(failures),
            "errors": errors,
            "kcirRef": None,
        }

    required_fields = ("statementId", "docPath", "anchor", "stmtType", "statementText")
    normalized: Dict[str, str] = {}
    for field in required_fields:
        raw = statement_core.get(field)
        if not isinstance(raw, str) or not raw.strip():
            failures.append(FAILURE_CLASS_INPUT_INVALID)
            errors.append(f"statementCore.{field} must be a non-empty string")
            continue
        normalized[field] = raw.strip()

    if failures:
        return {
            "result": "rejected",
            "failureClasses": canonical(failures),
            "errors": errors,
            "kcirRef": None,
        }

    ref = check_statement_index.compute_statement_kcir_ref(normalized)
    return {
        "result": "accepted",
        "failureClasses": [],
        "errors": [],
        "kcirRef": ref,
    }


def run(fixtures_root: Path) -> int:
    manifest = load_json(fixtures_root / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "statement-kcir":
        raise ValueError("manifest.suiteId must be 'statement-kcir'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")

    errors: List[str] = []
    executed = 0
    invariance_rows: Dict[str, List[Tuple[str, str, str | None, Tuple[str, ...]]]] = {}
    for vector_id in vectors:
        try:
            case = load_json(fixtures_root / vector_id / "case.json")
            expect = load_json(fixtures_root / vector_id / "expect.json")
            if case.get("schema") != 1:
                raise ValueError(f"{vector_id}: case schema must be 1")
            if case.get("suiteId") != "statement-kcir":
                raise ValueError(f"{vector_id}: case suiteId must be statement-kcir")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{vector_id}: case vectorId mismatch")
            if expect.get("schema") != 1:
                raise ValueError(f"{vector_id}: expect schema must be 1")

            semantic_scenario_id = ensure_optional_string(
                case.get("semanticScenarioId"),
                f"{vector_id}: case.semanticScenarioId",
            )
            profile = ensure_optional_string(case.get("profile"), f"{vector_id}: case.profile")

            result = evaluate_case(case.get("statementCore"))

            expected_result = ensure_string(expect.get("result"), f"{vector_id}: expect.result")
            expected_failures = canonical(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{vector_id}: expect.expectedFailureClasses",
                )
            )
            got_result = ensure_string(result.get("result"), f"{vector_id}: result.result")
            got_failures = canonical(
                ensure_string_list(
                    result.get("failureClasses", []),
                    f"{vector_id}: result.failureClasses",
                )
            )
            if got_result != expected_result or got_failures != expected_failures:
                raise ValueError(
                    f"expect/got mismatch: expect=({expected_result}, {expected_failures}) "
                    f"got=({got_result}, {got_failures})"
                )

            expected_ref = expect.get("expectedKcirRef")
            got_ref = result.get("kcirRef")
            if expected_ref is not None:
                if not isinstance(expected_ref, str):
                    raise ValueError(f"{vector_id}: expect.expectedKcirRef must be a string when provided")
                if got_ref != expected_ref:
                    raise ValueError(f"{vector_id}: expectedKcirRef mismatch")
            if expected_result == "accepted":
                if not isinstance(got_ref, str) or not got_ref.startswith("kcir1_"):
                    raise ValueError(f"{vector_id}: accepted vectors must emit kcir1_* ref")

            if semantic_scenario_id is not None:
                invariance_rows.setdefault(semantic_scenario_id, []).append(
                    (
                        vector_id,
                        profile or "default",
                        got_ref if isinstance(got_ref, str) else None,
                        tuple(got_failures),
                    )
                )

            print(f"[ok] statement-kcir/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    for scenario_id, rows in sorted(invariance_rows.items()):
        if len(rows) < 2:
            errors.append(f"invariance scenario {scenario_id!r} has fewer than 2 vectors")
            continue
        baseline = rows[0]
        for row in rows[1:]:
            if row[2] != baseline[2] or row[3] != baseline[3]:
                errors.append(
                    f"invariance mismatch for {scenario_id!r}: "
                    f"baseline={baseline[0]}@{baseline[1]} ref={baseline[2]} failures={list(baseline[3])} "
                    f"vs {row[0]}@{row[1]} ref={row[2]} failures={list(row[3])}"
                )

    if errors:
        print(f"[statement-kcir-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1
    print(
        "[statement-kcir-run] OK "
        f"(vectors={executed}, invarianceScenarios={len(invariance_rows)})"
    )
    return 0


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[statement-kcir-run] ERROR: fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[statement-kcir-run] ERROR: fixtures path is not a directory: {fixtures}")
        return 2
    try:
        return run(fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[statement-kcir-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
