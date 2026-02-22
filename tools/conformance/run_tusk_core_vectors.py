#!/usr/bin/env python3
"""
Execute deterministic tusk-core runtime contract vectors.

This suite runs `premath tusk-eval` over fixture descent packs and validates
stable result/failure-class/law-ref projections.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "tusk-core"
PREMATH_BIN = ROOT / "target" / "debug" / "premath"


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
    if suite_id != "tusk-core":
        raise ValueError("manifest.suiteId must be 'tusk-core'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def write_json(path: Path, payload: Dict[str, Any]) -> None:
    with path.open("w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, ensure_ascii=False)
        f.write("\n")


def run_tusk_eval(identity: Dict[str, Any], descent_pack: Dict[str, Any]) -> Dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="premath-tusk-core-") as tmp:
        identity_path = Path(tmp) / "identity.json"
        descent_pack_path = Path(tmp) / "descent-pack.json"
        write_json(identity_path, identity)
        write_json(descent_pack_path, descent_pack)

        if PREMATH_BIN.is_file():
            command = [
                str(PREMATH_BIN),
                "tusk-eval",
                "--identity",
                str(identity_path),
                "--descent-pack",
                str(descent_pack_path),
                "--json",
            ]
        else:
            command = [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "tusk-eval",
                "--identity",
                str(identity_path),
                "--descent-pack",
                str(descent_pack_path),
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
                f"tusk-eval command failed (exit={completed.returncode})\n"
                f"stdout:\n{completed.stdout}\n"
                f"stderr:\n{completed.stderr}"
            )
        try:
            payload = json.loads(completed.stdout)
        except json.JSONDecodeError as exc:
            raise ValueError(f"tusk-eval stdout was not valid json: {exc}") from exc
        if not isinstance(payload, dict):
            raise ValueError("tusk-eval stdout json root must be an object")
        return payload


def stable_projection(payload: Dict[str, Any], label: str) -> Dict[str, Any]:
    envelope = payload.get("envelope")
    if not isinstance(envelope, dict):
        raise ValueError(f"{label}.envelope must be an object")

    result = ensure_string(envelope.get("result"), f"{label}.envelope.result")
    if result not in {"accepted", "rejected"}:
        raise ValueError(f"{label}.envelope.result must be 'accepted' or 'rejected'")

    raw_failures = envelope.get("failures", [])
    if not isinstance(raw_failures, list):
        raise ValueError(f"{label}.envelope.failures must be a list")

    failure_pairs: List[Dict[str, str]] = []
    for idx, failure in enumerate(raw_failures):
        if not isinstance(failure, dict):
            raise ValueError(f"{label}.envelope.failures[{idx}] must be an object")
        failure_pairs.append(
            {
                "class": ensure_string(failure.get("class"), f"{label}.envelope.failures[{idx}].class"),
                "lawRef": ensure_string(failure.get("lawRef"), f"{label}.envelope.failures[{idx}].lawRef"),
            }
        )

    failure_pairs.sort(key=lambda row: (row["class"], row["lawRef"]))
    failure_classes = sorted({row["class"] for row in failure_pairs})
    failure_law_refs = sorted({row["lawRef"] for row in failure_pairs})

    glue_selected: Optional[str] = None
    glue_result = payload.get("glueResult")
    if glue_result is not None:
        if not isinstance(glue_result, dict):
            raise ValueError(f"{label}.glueResult must be object or null")
        glue_selected = ensure_string(glue_result.get("selected"), f"{label}.glueResult.selected")

    return {
        "result": result,
        "failureClasses": failure_classes,
        "failureLawRefs": failure_law_refs,
        "failurePairs": failure_pairs,
        "glueSelected": glue_selected,
    }


def canonical_expected(expect: Dict[str, Any], label: str) -> Dict[str, Any]:
    expected_outcome = expect.get("expectedOutcome")
    if not isinstance(expected_outcome, dict):
        raise ValueError(f"{label}.expectedOutcome must be an object")

    result = ensure_string(expected_outcome.get("result"), f"{label}.expectedOutcome.result")
    if result not in {"accepted", "rejected"}:
        raise ValueError(f"{label}.expectedOutcome.result must be 'accepted' or 'rejected'")

    failure_classes = sorted(
        set(ensure_string_list(expected_outcome.get("failureClasses", []), f"{label}.expectedOutcome.failureClasses"))
    )
    failure_law_refs = sorted(
        set(ensure_string_list(expected_outcome.get("failureLawRefs", []), f"{label}.expectedOutcome.failureLawRefs"))
    )

    raw_failure_pairs = expected_outcome.get("failurePairs", [])
    if not isinstance(raw_failure_pairs, list):
        raise ValueError(f"{label}.expectedOutcome.failurePairs must be a list")
    failure_pairs: List[Dict[str, str]] = []
    for idx, pair in enumerate(raw_failure_pairs):
        if not isinstance(pair, dict):
            raise ValueError(f"{label}.expectedOutcome.failurePairs[{idx}] must be an object")
        failure_pairs.append(
            {
                "class": ensure_string(pair.get("class"), f"{label}.expectedOutcome.failurePairs[{idx}].class"),
                "lawRef": ensure_string(pair.get("lawRef"), f"{label}.expectedOutcome.failurePairs[{idx}].lawRef"),
            }
        )
    failure_pairs.sort(key=lambda row: (row["class"], row["lawRef"]))

    glue_selected_value = expected_outcome.get("glueSelected")
    if glue_selected_value is None:
        glue_selected: Optional[str] = None
    else:
        glue_selected = ensure_string(glue_selected_value, f"{label}.expectedOutcome.glueSelected")

    return {
        "result": result,
        "failureClasses": failure_classes,
        "failureLawRefs": failure_law_refs,
        "failurePairs": failure_pairs,
        "glueSelected": glue_selected,
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
            if case.get("suiteId") != "tusk-core":
                raise ValueError(f"{case_path}: suiteId must be 'tusk-core'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")
            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")

            artifacts = case.get("artifacts")
            if not isinstance(artifacts, dict):
                raise ValueError(f"{case_path}: artifacts must be an object")
            identity = artifacts.get("identity")
            if not isinstance(identity, dict):
                raise ValueError(f"{case_path}: artifacts.identity must be an object")
            descent_pack = artifacts.get("descentPack")
            if not isinstance(descent_pack, dict):
                raise ValueError(f"{case_path}: artifacts.descentPack must be an object")

            payload = run_tusk_eval(identity, descent_pack)
            got = stable_projection(payload, f"{case_path}: payload")
            expected = canonical_expected(expect, f"{expect_path}: expectedOutcome")

            if got["result"] != expected_result:
                raise ValueError(
                    f"{case_path}: expected top-level result={expected_result}, got={got['result']}"
                )
            if got != expected:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={json.dumps(expected, sort_keys=True)}\n"
                    f"got={json.dumps(got, sort_keys=True)}"
                )

            print(f"[ok] tusk-core/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if errors:
        print(f"[tusk-core-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[tusk-core-run] OK (vectors={executed})")
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run tusk-core conformance vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Tusk-core fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[tusk-core-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
