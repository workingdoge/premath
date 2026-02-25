#!/usr/bin/env python3
"""Run doctrine-inf fixture vectors via core CLI semantics."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
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
CAPABILITY_REGISTRY_PATH = (
    Path(__file__).resolve().parents[2]
    / "specs"
    / "premath"
    / "draft"
    / "CAPABILITY-REGISTRY.json"
)
CAPABILITY_REGISTRY_KIND = "premath.capability_registry.v1"
DOCTRINE_INF_CHECK_COMMAND_PREFIX = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "doctrine-inf-check",
)

GOVERNANCE_PROFILE_CLAIM_ID = "profile.doctrine_inf_governance.v0"


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


def load_profile_overlay_claims(registry_path: Path) -> Set[str]:
    payload = load_json(registry_path)
    if payload.get("schema") != 1:
        raise ValueError(f"{registry_path}: schema must be 1")
    if payload.get("registryKind") != CAPABILITY_REGISTRY_KIND:
        raise ValueError(
            f"{registry_path}: registryKind must be {CAPABILITY_REGISTRY_KIND!r}"
        )
    claims = payload.get("profileOverlayClaims", [])
    if not isinstance(claims, list):
        raise ValueError(f"{registry_path}: profileOverlayClaims must be a list")
    out: Set[str] = set()
    for idx, claim in enumerate(claims):
        if not isinstance(claim, str) or not claim:
            raise ValueError(
                f"{registry_path}: profileOverlayClaims[{idx}] must be a non-empty string"
            )
        out.add(claim)
    return out


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "doctrine-inf":
        raise ValueError("manifest.suiteId must be 'doctrine-inf'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _validate_doctrine_inf_check_command(cmd: List[str]) -> None:
    prefix = DOCTRINE_INF_CHECK_COMMAND_PREFIX
    if tuple(cmd[: len(prefix)]) != prefix:
        raise ValueError(
            "doctrine-inf-check command surface drift: expected prefix "
            f"{list(prefix)!r}, got {cmd!r}"
        )


def _resolve_doctrine_inf_check_command() -> List[str]:
    override = os.environ.get("PREMATH_DOCTRINE_INF_CHECK_CMD", "").strip()
    if override:
        command = shlex.split(override)
    else:
        command = list(DOCTRINE_INF_CHECK_COMMAND_PREFIX)
    _validate_doctrine_inf_check_command(command)
    return command


def _run_kernel_doctrine_inf_check(case_path: Path) -> Dict[str, Any]:
    command = _resolve_doctrine_inf_check_command()
    completed = subprocess.run(
        [*command, "--input", str(case_path), "--json"],
        cwd=_repo_root(),
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode not in {0, 1}:
        raise ValueError(
            "kernel doctrine-inf-check command failed: "
            f"exit={completed.returncode}, stderr={completed.stderr.strip()!r}"
        )
    stdout = completed.stdout.strip()
    if not stdout:
        raise ValueError("kernel doctrine-inf-check produced empty stdout")
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise ValueError(
            "kernel doctrine-inf-check emitted invalid json: "
            f"{exc}"
        ) from exc
    if not isinstance(payload, dict):
        raise ValueError("kernel doctrine-inf-check payload must be an object")
    if "result" not in payload or "failureClasses" not in payload:
        raise ValueError("kernel doctrine-inf-check payload missing result/failureClasses")
    return payload


def run(fixtures: Path, registry_path: Path, enforce_repo_claims: bool) -> int:
    vectors = validate_manifest(fixtures)
    profile_overlay_claims = load_profile_overlay_claims(registry_path)
    repo_governance_claimed = GOVERNANCE_PROFILE_CLAIM_ID in profile_overlay_claims
    errors: List[str] = []
    executed = 0
    skipped_repo_claim = 0
    executed_governance_claimed = 0

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

            governance_profile = case.get("governanceProfile")
            if governance_profile is not None:
                if not isinstance(governance_profile, dict):
                    raise ValueError("governanceProfile must be an object when provided")
                governance_case_claimed = governance_profile.get("claimed")
                if not isinstance(governance_case_claimed, bool):
                    raise ValueError("governanceProfile.claimed must be a boolean")
                if (
                    enforce_repo_claims
                    and governance_case_claimed
                    and not repo_governance_claimed
                ):
                    print(
                        f"[skip] doctrine-inf/{vector_id} "
                        "(governance profile not claimed in CAPABILITY-REGISTRY)"
                    )
                    skipped_repo_claim += 1
                    continue
                if governance_case_claimed:
                    executed_governance_claimed += 1

            got = _run_kernel_doctrine_inf_check(case_path)

            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{expect_path}: expectedFailureClasses",
                )
            )

            got_result = ensure_string(got.get("result"), f"{case_path}: got.result")
            got_failure_classes = canonical_set(
                ensure_string_list(
                    got.get("failureClasses", []),
                    f"{case_path}: got.failureClasses",
                )
            )
            if got_result != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={{'result': {got_result!r}, 'failureClasses': {got_failure_classes!r}}}"
                )

            print(f"[ok] doctrine-inf/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    if enforce_repo_claims and repo_governance_claimed and executed_governance_claimed == 0:
        errors.append(
            "repository claims profile.doctrine_inf_governance.v0 but no governanceProfile.claimed=true vectors were executed"
        )

    if errors:
        print(
            "[doctrine-inf-run] FAIL "
            f"(vectors={executed}, skipped={skipped_repo_claim}, errors={len(errors)}, "
            f"repoGovernanceClaimed={str(repo_governance_claimed).lower()})"
        )
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[doctrine-inf-run] OK "
        f"(vectors={executed}, skipped={skipped_repo_claim}, "
        f"repoGovernanceClaimed={str(repo_governance_claimed).lower()})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run doctrine-inf semantic boundary vectors.")
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Doctrine-inf fixture root (default: {DEFAULT_FIXTURES})",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=CAPABILITY_REGISTRY_PATH,
        help=f"Capability registry path for profile-overlay claim binding (default: {CAPABILITY_REGISTRY_PATH})",
    )
    parser.add_argument(
        "--ignore-repo-claims",
        action="store_true",
        help="Ignore CAPABILITY-REGISTRY profileOverlayClaims gating and execute all vectors as listed in the fixture manifest.",
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
    registry = args.registry
    if not registry.exists():
        print(f"[error] registry path does not exist: {registry}")
        return 2
    if not registry.is_file():
        print(f"[error] registry path is not a file: {registry}")
        return 2
    try:
        return run(
            fixtures=fixtures,
            registry_path=registry,
            enforce_repo_claims=not args.ignore_repo_claims,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-inf-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
