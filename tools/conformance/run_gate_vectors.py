#!/usr/bin/env python3
"""
Execute canonical Gate conformance vectors.

These vectors exercise deterministic Gate witness classing/order for:
- stability
- locality
- descent
- contractible glue uniqueness
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "toy"))

from toy_gate_check import run_case  # type: ignore  # noqa: E402

DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "gate"


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
    if suite_id != "gate":
        raise ValueError("manifest.suiteId must be 'gate'")

    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def _stable_failure_subset_equal(expect: Dict[str, Any], got: Dict[str, Any]) -> bool:
    if expect.get("result") != got.get("result"):
        return False
    ef = expect.get("failures", [])
    gf = got.get("failures", [])
    if not isinstance(ef, list) or not isinstance(gf, list):
        return False
    if len(ef) != len(gf):
        return False
    for e, g in zip(ef, gf):
        if not isinstance(e, dict) or not isinstance(g, dict):
            return False
        for key in ("class", "lawRef", "witnessId"):
            if e.get(key) != g.get(key):
                return False
    return True


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
            if case.get("suiteId") != "gate":
                raise ValueError(f"{case_path}: suiteId must be 'gate'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            got = run_case(case)
            if not _stable_failure_subset_equal(expect, got):
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={json.dumps(expect, sort_keys=True)}\n"
                    f"got={json.dumps(got, sort_keys=True)}"
                )
            print(f"[ok] gate/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[gate-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[gate-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Gate conformance vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Gate fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[gate-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
