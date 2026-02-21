#!/usr/bin/env python3
"""
Validate conformance capability fixture stubs.

This checker is intentionally lightweight. It ensures:
- manifest/vector shape integrity,
- case/expect JSON presence and consistency,
- invariance-pair completeness by semanticScenarioId.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Tuple


def load_json(path: Path, errors: List[str]) -> Optional[dict]:
    try:
        with path.open("r", encoding="utf-8") as f:
            data = json.load(f)
    except FileNotFoundError:
        errors.append(f"missing file: {path}")
        return None
    except json.JSONDecodeError as exc:
        errors.append(f"invalid json: {path} ({exc})")
        return None
    if not isinstance(data, dict):
        errors.append(f"json root must be object: {path}")
        return None
    return data


def discover_vector_dirs(capability_dir: Path) -> List[str]:
    out: List[str] = []
    for case_path in sorted(capability_dir.glob("**/case.json")):
        rel = case_path.relative_to(capability_dir).as_posix()
        # rel is "<vector>/case.json"
        out.append(rel[: -len("/case.json")])
    return out


def validate_capability_dir(capability_dir: Path, errors: List[str], warnings: List[str]) -> int:
    manifest_path = capability_dir / "manifest.json"
    manifest = load_json(manifest_path, errors)
    if manifest is None:
        return 0

    capability_id = manifest.get("capabilityId")
    if not isinstance(capability_id, str) or not capability_id:
        errors.append(f"{manifest_path}: capabilityId must be non-empty string")
        return 0

    if capability_id != capability_dir.name:
        errors.append(
            f"{manifest_path}: capabilityId '{capability_id}' must match directory name '{capability_dir.name}'"
        )

    vectors = manifest.get("vectors")
    if not isinstance(vectors, list) or not vectors:
        errors.append(f"{manifest_path}: vectors must be non-empty list")
        return 0

    manifest_vectors: List[str] = []
    for idx, v in enumerate(vectors):
        if not isinstance(v, str) or not v:
            errors.append(f"{manifest_path}: vectors[{idx}] must be non-empty string")
            continue
        manifest_vectors.append(v)

    if len(set(manifest_vectors)) != len(manifest_vectors):
        errors.append(f"{manifest_path}: duplicate entries in vectors")

    discovered_vectors = discover_vector_dirs(capability_dir)
    missing_in_manifest = sorted(set(discovered_vectors) - set(manifest_vectors))
    missing_on_disk = sorted(set(manifest_vectors) - set(discovered_vectors))
    for v in missing_in_manifest:
        errors.append(f"{manifest_path}: case exists on disk but not in vectors: {v}")
    for v in missing_on_disk:
        errors.append(f"{manifest_path}: vector declared but missing case.json: {v}")

    invariance_groups: Dict[str, List[Tuple[str, Optional[str]]]] = defaultdict(list)

    checked = 0
    for vector in manifest_vectors:
        case_path = capability_dir / vector / "case.json"
        expect_path = capability_dir / vector / "expect.json"
        case = load_json(case_path, errors)
        expect = load_json(expect_path, errors)
        if case is None or expect is None:
            continue
        checked += 1

        case_cap = case.get("capabilityId")
        if case_cap != capability_id:
            errors.append(f"{case_path}: capabilityId '{case_cap}' != manifest capabilityId '{capability_id}'")

        case_vec = case.get("vectorId")
        if case_vec != vector:
            errors.append(f"{case_path}: vectorId '{case_vec}' != manifest vector '{vector}'")

        if case.get("schema") != 1:
            warnings.append(f"{case_path}: schema is not 1")
        if expect.get("schema") != 1:
            warnings.append(f"{expect_path}: schema is not 1")

        if vector.startswith("invariance/"):
            sid = case.get("semanticScenarioId")
            profile = case.get("profile")
            if not isinstance(sid, str) or not sid:
                errors.append(f"{case_path}: invariance case requires non-empty semanticScenarioId")
            else:
                invariance_groups[sid].append((vector, profile if isinstance(profile, str) else None))

            assertions = expect.get("assertions")
            if not isinstance(assertions, list) or not assertions:
                errors.append(f"{expect_path}: invariance expect requires non-empty assertions list")
            else:
                text = " ".join(str(x) for x in assertions).lower()
                if "kernel verdict" not in text:
                    errors.append(f"{expect_path}: invariance assertions must mention kernel verdict")
                if "gate failure" not in text:
                    errors.append(f"{expect_path}: invariance assertions must mention Gate failure classes")

    if not invariance_groups:
        warnings.append(f"{manifest_path}: no invariance cases found")
    else:
        for sid, rows in sorted(invariance_groups.items()):
            if len(rows) != 2:
                errors.append(
                    f"{manifest_path}: invariance scenario '{sid}' must have exactly 2 vectors, found {len(rows)}"
                )
                continue
            profiles = {p for _, p in rows if p}
            if len(profiles) < 2:
                errors.append(
                    f"{manifest_path}: invariance scenario '{sid}' should have two distinct profiles; got {sorted(profiles)}"
                )

    return checked


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    default_fixtures = root / "tests" / "conformance" / "fixtures" / "capabilities"
    parser = argparse.ArgumentParser(description="Validate conformance capability fixture stubs.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=default_fixtures,
        help=f"Path to capability fixture root (default: {default_fixtures})",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixtures_root = args.fixtures

    errors: List[str] = []
    warnings: List[str] = []

    if not fixtures_root.exists():
        print(f"[error] fixtures path does not exist: {fixtures_root}")
        return 2
    if not fixtures_root.is_dir():
        print(f"[error] fixtures path is not a directory: {fixtures_root}")
        return 2

    capability_dirs = sorted(
        d
        for d in fixtures_root.iterdir()
        if d.is_dir() and not d.name.startswith(".")
    )

    if not capability_dirs:
        print(f"[error] no capability directories found under: {fixtures_root}")
        return 2

    checked_vectors = 0
    for d in capability_dirs:
        checked_vectors += validate_capability_dir(d, errors, warnings)

    if errors:
        print(f"[conformance-check] FAIL ({len(errors)} errors, {len(warnings)} warnings)")
        for e in errors:
            print(f"  - {e}")
        if warnings:
            print("[warnings]")
            for w in warnings:
                print(f"  - {w}")
        return 1

    print(
        f"[conformance-check] OK "
        f"(capabilities={len(capability_dirs)}, vectors={checked_vectors}, warnings={len(warnings)})"
    )
    for w in warnings:
        print(f"  [warn] {w}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
