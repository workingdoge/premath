#!/usr/bin/env python3
"""
Execute deterministic harness typestate closure vectors.

This suite runs `premath harness-join-check` over fixture inputs and validates
stable accept/reject + failure-class projections for protocol/handoff/tool-use
closure and mutation-admissibility preconditions.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Sequence

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "harness-typestate"
PREMATH_BIN = ROOT / "target" / "debug" / "premath"
HARNESS_JOIN_CHECK_SOURCE = (
    ROOT / "crates" / "premath-cli" / "src" / "commands" / "harness_join_check.rs"
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


def parse_emitted_failure_classes(source_path: Path) -> List[str]:
    try:
        source = source_path.read_text(encoding="utf-8")
    except FileNotFoundError as exc:
        raise ValueError(f"missing harness join-check source: {source_path}") from exc

    classes = sorted(
        {
            match
            for match in re.findall(
                r'insert\("([a-z0-9_.-]+)"\.to_string\(\)\);',
                source,
            )
        }
    )
    if not classes:
        raise ValueError(
            f"no emitted failure classes parsed from join-check source: {source_path}"
        )
    return classes


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "harness-typestate":
        raise ValueError("manifest.suiteId must be 'harness-typestate'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def run_harness_join_check(payload: Dict[str, Any]) -> Dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="premath-harness-typestate-") as tmp:
        input_path = Path(tmp) / "input.json"
        input_path.write_text(
            json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        if PREMATH_BIN.is_file():
            command = [
                str(PREMATH_BIN),
                "harness-join-check",
                "--input",
                str(input_path),
                "--json",
            ]
        else:
            command = [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "harness-join-check",
                "--input",
                str(input_path),
                "--json",
            ]

        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            raise ValueError(
                f"harness-join-check command failed (exit={completed.returncode})\n"
                f"stdout:\n{completed.stdout}\n"
                f"stderr:\n{completed.stderr}"
            )

        try:
            output = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise ValueError(f"harness-join-check stdout was not valid json: {exc}") from exc
        if not isinstance(output, dict):
            raise ValueError("harness-join-check stdout root must be object")
        return output


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    errors: List[str] = []
    executed = 0
    expected_failure_classes_all: set[str] = set()

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "harness-typestate":
                raise ValueError(f"{case_path}: suiteId must be 'harness-typestate'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            input_payload = case.get("input")
            if not isinstance(input_payload, dict):
                raise ValueError(f"{case_path}: input must be an object")

            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(expect.get("expectedFailureClasses", []), f"{expect_path}: expectedFailureClasses")
            )
            expected_failure_classes_all.update(expected_failure_classes)
            expected_join_closed = expect.get("expectedJoinClosed")
            if expected_join_closed is not None and not isinstance(expected_join_closed, bool):
                raise ValueError(f"{expect_path}: expectedJoinClosed must be a boolean when present")

            output = run_harness_join_check(input_payload)
            got_result = ensure_string(output.get("result"), f"{vector_id}: output.result")
            got_failure_classes = canonical_set(
                ensure_string_list(output.get("failureClasses", []), f"{vector_id}: output.failureClasses")
            )

            if got_result != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={{'result': {got_result!r}, 'failureClasses': {got_failure_classes!r}}}"
                )

            if expected_join_closed is not None:
                got_join_closed = output.get("joinClosed")
                if got_join_closed is not expected_join_closed:
                    raise ValueError(
                        f"{vector_id}: expected joinClosed={expected_join_closed}, got={got_join_closed}"
                    )

            print(f"[ok] harness-typestate/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    try:
        emitted_failure_classes = set(parse_emitted_failure_classes(HARNESS_JOIN_CHECK_SOURCE))
        expected_failure_classes = set(expected_failure_classes_all)
        missing_coverage = sorted(emitted_failure_classes - expected_failure_classes)
        unreferenced_vectors = sorted(expected_failure_classes - emitted_failure_classes)

        if missing_coverage:
            errors.append(
                "failure-class coverage missing vectors for emitted classes: "
                + ", ".join(missing_coverage)
            )
        if unreferenced_vectors:
            errors.append(
                "failure-class coverage has unreferenced expected classes: "
                + ", ".join(unreferenced_vectors)
            )
    except Exception as exc:  # noqa: BLE001
        errors.append(f"failure-class coverage check: {exc}")

    if errors:
        print(f"[harness-typestate-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[harness-typestate-run] OK "
        f"(vectors={executed}, coveredClasses={len(expected_failure_classes_all)})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run harness typestate conformance vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Harness typestate fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[harness-typestate-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
