#!/usr/bin/env python3
"""Run statement-binding conformance vectors."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence

import check_statement_bindings

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "statement-bindings"


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run statement-binding fixture vectors.")
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


def run(fixtures_root: Path) -> int:
    manifest = load_json(fixtures_root / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "statement-bindings":
        raise ValueError("manifest.suiteId must be 'statement-bindings'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")

    errors: List[str] = []
    executed = 0
    for vector_id in vectors:
        try:
            case = load_json(fixtures_root / vector_id / "case.json")
            expect = load_json(fixtures_root / vector_id / "expect.json")
            if case.get("schema") != 1:
                raise ValueError(f"{vector_id}: case schema must be 1")
            if case.get("suiteId") != "statement-bindings":
                raise ValueError(f"{vector_id}: case suiteId must be statement-bindings")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{vector_id}: case vectorId mismatch")
            if expect.get("schema") != 1:
                raise ValueError(f"{vector_id}: expect schema must be 1")

            statement_index = case.get("statementIndex")
            binding_contract = case.get("bindingContract")
            if not isinstance(statement_index, dict):
                raise ValueError(f"{vector_id}: statementIndex must be an object")
            if not isinstance(binding_contract, dict):
                raise ValueError(f"{vector_id}: bindingContract must be an object")

            result = check_statement_bindings.evaluate_statement_bindings(
                statement_index=statement_index,
                binding_contract=binding_contract,
                repo_root=ROOT,
            )
            got_result = ensure_string(result.get("result"), f"{vector_id}: result.result")
            got_failures = canonical(
                ensure_string_list(result.get("failureClasses", []), f"{vector_id}: result.failureClasses")
            )

            expected_result = ensure_string(expect.get("result"), f"{vector_id}: expect.result")
            expected_failures = canonical(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{vector_id}: expect.expectedFailureClasses",
                )
            )
            if got_result != expected_result or got_failures != expected_failures:
                raise ValueError(
                    f"expect/got mismatch: expect=({expected_result}, {expected_failures}) "
                    f"got=({got_result}, {got_failures})"
                )

            print(f"[ok] statement-bindings/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[statement-binding-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1
    print(f"[statement-binding-run] OK (vectors={executed})")
    return 0


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[statement-binding-run] ERROR: fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[statement-binding-run] ERROR: fixtures path is not a directory: {fixtures}")
        return 2
    try:
        return run(fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[statement-binding-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
