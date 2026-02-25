#!/usr/bin/env python3
"""Run world-core conformance vectors against core world/site command outputs."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

import core_command_client

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "world-core"
SITE_RESOLVE_COMMAND_PREFIX = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "site-resolve",
)


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
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string when provided")
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


def canonical_set(values: Sequence[str]) -> List[str]:
    return sorted(set(values))


def ensure_bool(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{label} must be a boolean")
    return value


def parse_required_route_families(value: Any, label: str) -> List[str]:
    if value is None:
        return []
    return canonical_set(ensure_string_list(value, label))


def parse_required_route_bindings(
    value: Any,
    label: str,
) -> Dict[str, List[str]]:
    if value is None:
        return {}
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list when provided")
    out: Dict[str, List[str]] = {}
    for idx, row in enumerate(value):
        if not isinstance(row, dict):
            raise ValueError(f"{label}[{idx}] must be an object")
        family = ensure_string(row.get("routeFamilyId"), f"{label}[{idx}].routeFamilyId")
        operation_ids = canonical_set(
            ensure_string_list(row.get("operationIds"), f"{label}[{idx}].operationIds")
        )
        if not operation_ids:
            raise ValueError(f"{label}[{idx}].operationIds must be non-empty")
        merged = set(out.get(family, [])) | set(operation_ids)
        out[family] = sorted(merged)
    return out


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "world-core":
        raise ValueError("manifest.suiteId must be 'world-core'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


@dataclass(frozen=True)
class EvaluationResult:
    result: str
    failure_classes: List[str]
    projection_signature: Tuple[str, ...] | None


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _validate_site_resolve_command(cmd: List[str]) -> None:
    prefix = SITE_RESOLVE_COMMAND_PREFIX
    if tuple(cmd[: len(prefix)]) != prefix:
        raise ValueError(
            "site-resolve command surface drift: expected prefix "
            f"{list(prefix)!r}, got {cmd!r}"
        )


def _resolve_site_resolve_command() -> List[str]:
    override = os.environ.get("PREMATH_SITE_RESOLVE_CMD", "").strip()
    if override:
        command = shlex.split(override)
    else:
        command = list(SITE_RESOLVE_COMMAND_PREFIX)
    _validate_site_resolve_command(command)
    return command


def _run_kernel_site_resolve(
    *,
    request: Dict[str, Any],
    doctrine_site_input: Dict[str, Any],
    doctrine_site: Dict[str, Any],
    doctrine_operation_registry: Dict[str, Any],
    control_plane_contract: Dict[str, Any],
    capability_registry: Dict[str, Any],
) -> Dict[str, Any]:
    root = _repo_root()
    command = _resolve_site_resolve_command()
    with tempfile.TemporaryDirectory(prefix="premath-site-resolve-") as tmp:
        tmp_root = Path(tmp)
        request_path = tmp_root / "request.json"
        site_input_path = tmp_root / "doctrine_site_input.json"
        site_path = tmp_root / "doctrine_site.json"
        operations_path = tmp_root / "doctrine_operation_registry.json"
        control_plane_path = tmp_root / "control_plane_contract.json"
        capability_path = tmp_root / "capability_registry.json"
        request_path.write_text(
            json.dumps(request, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        site_input_path.write_text(
            json.dumps(doctrine_site_input, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        site_path.write_text(
            json.dumps(doctrine_site, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        operations_path.write_text(
            json.dumps(doctrine_operation_registry, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        control_plane_path.write_text(
            json.dumps(control_plane_contract, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        capability_path.write_text(
            json.dumps(capability_registry, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        cmd = [
            *command,
            "--request",
            str(request_path),
            "--doctrine-site-input",
            str(site_input_path),
            "--doctrine-site",
            str(site_path),
            "--doctrine-op-registry",
            str(operations_path),
            "--control-plane-contract",
            str(control_plane_path),
            "--capability-registry",
            str(capability_path),
            "--json",
        ]
        completed = subprocess.run(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode not in {0, 1}:
            raise ValueError(
                "kernel site-resolve command failed: "
                f"exit={completed.returncode}, stderr={completed.stderr.strip()!r}"
            )
        stdout = completed.stdout.strip()
        if not stdout:
            raise ValueError("kernel site-resolve produced empty stdout")
        payload = json.loads(stdout)
        if not isinstance(payload, dict):
            raise ValueError("kernel site-resolve payload must be an object")
        return payload


def _site_resolve_projection_signature(payload: Dict[str, Any]) -> Tuple[str, ...] | None:
    projection = payload.get("projection")
    witness = payload.get("witness")
    selected = payload.get("selected")
    if not isinstance(projection, dict) or not isinstance(witness, dict):
        return None

    selected_operation = ""
    selected_route_family = ""
    selected_world = ""
    selected_morphism_row = ""
    selected_site_node = ""
    selected_cover = ""
    if isinstance(selected, dict):
        selected_operation = str(selected.get("operationId", "")).strip()
        selected_route_family = str(selected.get("routeFamilyId", "")).strip()
        selected_world = str(selected.get("worldId", "")).strip()
        selected_morphism_row = str(selected.get("morphismRowId", "")).strip()
        selected_site_node = str(selected.get("siteNodeId", "")).strip()
        selected_cover = str(selected.get("coverId", "")).strip()

    return (
        str(projection.get("sitePackageDigest", "")).strip(),
        str(projection.get("worldRouteDigest", "")).strip(),
        str(witness.get("semanticDigest", "")).strip(),
        selected_operation,
        selected_route_family,
        selected_world,
        selected_morphism_row,
        selected_site_node,
        selected_cover,
    )


def evaluate_world_registry_vector(case: Dict[str, Any]) -> EvaluationResult:
    control_plane_contract = case.get("controlPlaneContract")
    if not isinstance(control_plane_contract, dict):
        raise ValueError("case.controlPlaneContract must be an object")
    doctrine_op_registry = case.get("doctrineOpRegistry")
    if not isinstance(doctrine_op_registry, dict):
        raise ValueError("case.doctrineOpRegistry must be an object")
    doctrine_site_input = case.get("doctrineSiteInput")
    if not isinstance(doctrine_site_input, dict):
        raise ValueError("case.doctrineSiteInput must be an object")
    required_route_families = parse_required_route_families(
        case.get("requiredRouteFamilies"),
        "case.requiredRouteFamilies",
    )
    required_route_bindings = parse_required_route_bindings(
        case.get("requiredRouteBindings"),
        "case.requiredRouteBindings",
    )

    core_payload = core_command_client.run_world_registry_check(
        doctrine_site_input=doctrine_site_input,
        doctrine_operation_registry=doctrine_op_registry,
        control_plane_contract=control_plane_contract,
        required_route_families=tuple(required_route_families) if required_route_families else None,
        required_route_bindings=required_route_bindings or None,
    )
    core_result = ensure_string(core_payload.get("result"), "core.result")
    if core_result not in {"accepted", "rejected"}:
        raise ValueError("core.result must be 'accepted' or 'rejected'")
    core_failure_classes = canonical_set(
        ensure_string_list(core_payload.get("failureClasses", []), "core.failureClasses")
    )

    return EvaluationResult(
        result=core_result,
        failure_classes=core_failure_classes,
        projection_signature=None,
    )


def evaluate_site_resolve_vector(case: Dict[str, Any]) -> EvaluationResult:
    request = case.get("request")
    if not isinstance(request, dict):
        raise ValueError("case.request must be an object for mode=site-resolve")
    control_plane_contract = case.get("controlPlaneContract")
    if not isinstance(control_plane_contract, dict):
        raise ValueError("case.controlPlaneContract must be an object")
    doctrine_op_registry = case.get("doctrineOpRegistry")
    if not isinstance(doctrine_op_registry, dict):
        raise ValueError("case.doctrineOpRegistry must be an object")
    doctrine_site_input = case.get("doctrineSiteInput")
    if not isinstance(doctrine_site_input, dict):
        raise ValueError("case.doctrineSiteInput must be an object")
    doctrine_site = case.get("doctrineSite")
    if not isinstance(doctrine_site, dict):
        raise ValueError("case.doctrineSite must be an object for mode=site-resolve")
    capability_registry = case.get("capabilityRegistry")
    if not isinstance(capability_registry, dict):
        raise ValueError(
            "case.capabilityRegistry must be an object for mode=site-resolve"
        )

    payload = _run_kernel_site_resolve(
        request=request,
        doctrine_site_input=doctrine_site_input,
        doctrine_site=doctrine_site,
        doctrine_operation_registry=doctrine_op_registry,
        control_plane_contract=control_plane_contract,
        capability_registry=capability_registry,
    )
    result = ensure_string(payload.get("result"), "core.result")
    if result not in {"accepted", "rejected"}:
        raise ValueError("core.result must be 'accepted' or 'rejected'")
    failure_classes = canonical_set(
        ensure_string_list(payload.get("failureClasses", []), "core.failureClasses")
    )
    return EvaluationResult(
        result=result,
        failure_classes=failure_classes,
        projection_signature=_site_resolve_projection_signature(payload),
    )


def evaluate_vector(case: Dict[str, Any]) -> EvaluationResult:
    mode = ensure_optional_string(case.get("mode"), "case.mode") or "world-registry"
    if mode == "world-registry":
        return evaluate_world_registry_vector(case)
    if mode == "site-resolve":
        return evaluate_site_resolve_vector(case)
    raise ValueError("case.mode must be world-registry|site-resolve")


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    errors: List[str] = []
    executed = 0
    invariance_rows: Dict[
        str,
        List[Tuple[str, str, str, Tuple[str, ...], Tuple[str, ...] | None, bool]],
    ] = {}

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "world-core":
                raise ValueError(f"{case_path}: suiteId must be 'world-core'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            semantic_scenario_id = ensure_optional_string(
                case.get("semanticScenarioId"),
                f"{case_path}: semanticScenarioId",
            )
            profile = ensure_optional_string(case.get("profile"), f"{case_path}: profile")
            require_projection_invariance = False
            if "requireProjectionInvariance" in case:
                require_projection_invariance = ensure_bool(
                    case.get("requireProjectionInvariance"),
                    f"{case_path}: requireProjectionInvariance",
                )
            if vector_id.startswith("invariance/"):
                if semantic_scenario_id is None:
                    raise ValueError(
                        f"{case_path}: invariance vectors require semanticScenarioId"
                    )
                if profile is None:
                    raise ValueError(f"{case_path}: invariance vectors require profile")

            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be 'accepted' or 'rejected'")
            expected_failure_classes = canonical_set(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{expect_path}: expectedFailureClasses",
                )
            )

            evaluation = evaluate_vector(case)
            if (
                evaluation.result != expected_result
                or evaluation.failure_classes != expected_failure_classes
            ):
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={{'result': {evaluation.result!r}, 'failureClasses': {evaluation.failure_classes!r}}}"
                )

            if semantic_scenario_id is not None:
                invariance_rows.setdefault(semantic_scenario_id, []).append(
                    (
                        vector_id,
                        profile or "default",
                        evaluation.result,
                        tuple(evaluation.failure_classes),
                        evaluation.projection_signature,
                        require_projection_invariance,
                    )
                )

            print(f"[ok] world-core/{vector_id}")
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
        for vector_id, profile, result, failure_classes, _, _ in rows[1:]:
            if result != baseline_result or failure_classes != baseline_failures:
                errors.append(
                    "invariance mismatch for "
                    f"{scenario_id!r}: baseline=({baseline_result}, {list(baseline_failures)}) "
                    f"vs {vector_id}@{profile}=({result}, {list(failure_classes)})"
                )
        projection_rows = [row for row in rows if row[5]]
        if projection_rows:
            baseline_projection = projection_rows[0][4]
            if baseline_projection is None:
                errors.append(
                    f"invariance scenario {scenario_id!r} requires projection invariance but baseline signature is missing"
                )
            else:
                for vector_id, profile, _, _, projection_signature, _ in projection_rows[1:]:
                    if projection_signature != baseline_projection:
                        errors.append(
                            "projection invariance mismatch for "
                            f"{scenario_id!r}: baseline={list(baseline_projection)} "
                            f"vs {vector_id}@{profile}={list(projection_signature or ())}"
                        )

    if errors:
        print(f"[world-core-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[world-core-run] OK "
        f"(vectors={executed}, invarianceScenarios={len(invariance_rows)})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run world-core conformance vectors."
    )
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"World-core fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[world-core-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
