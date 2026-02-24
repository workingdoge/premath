#!/usr/bin/env python3
"""Run statement-index conformance vectors."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence

import check_statement_index

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "statement-index"


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run statement-index fixture vectors.")
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
    if suite_id != "statement-index":
        raise ValueError("manifest.suiteId must be 'statement-index'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")

    errors: List[str] = []
    executed = 0
    for vector_id in vectors:
        try:
            case = load_json(fixtures_root / vector_id / "case.json")
            expect = load_json(fixtures_root / vector_id / "expect.json")
            if case.get("schema") != 1:
                raise ValueError(f"{vector_id}: case schema must be 1")
            if case.get("suiteId") != "statement-index":
                raise ValueError(f"{vector_id}: case suiteId must be statement-index")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{vector_id}: case vectorId mismatch")
            if expect.get("schema") != 1:
                raise ValueError(f"{vector_id}: expect schema must be 1")

            doc_path = ensure_string(case.get("docPath"), f"{vector_id}: docPath")
            markdown_text = ensure_string(case.get("markdown"), f"{vector_id}: markdown")
            source_digest = hashlib.sha256(markdown_text.encode("utf-8")).hexdigest()
            extractor_digest = "fixture-extractor-v1"

            result = check_statement_index.evaluate_statement_index(
                markdown_text=markdown_text,
                doc_path=doc_path,
                source_digest=source_digest,
                extractor_digest=extractor_digest,
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
                    f"expect/got mismatch: "
                    f"expect=({expected_result}, {expected_failures}) "
                    f"got=({got_result}, {got_failures})"
                )

            expected_ids = canonical(
                ensure_string_list(
                    expect.get("expectedStatementIds", []),
                    f"{vector_id}: expect.expectedStatementIds",
                )
            )
            got_ids = canonical(
                [str(row.get("statementId", "")).strip() for row in result.get("rows", []) if isinstance(row, dict)]
            )
            if expected_ids != got_ids:
                raise ValueError(f"statement id mismatch: expect={expected_ids} got={got_ids}")

            print(f"[ok] statement-index/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[statement-index-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1
    print(f"[statement-index-run] OK (vectors={executed})")
    return 0


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    fixtures = args.fixtures
    if not fixtures.exists():
        print(f"[statement-index-run] ERROR: fixtures path does not exist: {fixtures}")
        return 2
    if not fixtures.is_dir():
        print(f"[statement-index-run] ERROR: fixtures path is not a directory: {fixtures}")
        return 2
    try:
        return run(fixtures)
    except Exception as exc:  # noqa: BLE001
        print(f"[statement-index-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
