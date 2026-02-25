#!/usr/bin/env python3
"""Validate Harness+Squeak runtime orchestration route bindings."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
from typing import Any, Dict, List, Tuple


SCHEMA = 1
CHECK_KIND = "conformance.runtime_orchestration.v1"
FAILURE_CLASS_ROUTE_MISSING = "runtime_route_missing"
FAILURE_CLASS_MORPHISM_DRIFT = "runtime_route_morphism_drift"
FAILURE_CLASS_CONTRACT_UNBOUND = "runtime_route_contract_unbound"
FAILURE_CLASS_KCIR_MAPPING_CONTRACT_VIOLATION = "kcir_mapping_contract_violation"
FAILURE_CLASS_WORLD_ROUTE_UNBOUND = "world_route_unbound"
FAILURE_CLASS_WORLD_ROUTE_MORPHISM_DRIFT = "world_route_morphism_drift"
WORLD_REGISTRY_CHECK_COMMAND_PREFIX = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "world-registry-check",
)
REQUIRED_HANDOFF_HEADING = "## 1.2 Harness-Squeak composition boundary (required)"
REQUIRED_HANDOFF_TOKENS = (
    "Harness computes deterministic work context and witness lineage refs.",
    "Squeak performs transport/runtime-placement mapping",
    "Destination Tusk/Gate performs destination-local admissibility checks",
    "Harness records the resulting references in session/trajectory projections.",
)
REQUIRED_KCIR_MAPPING_ROWS = (
    "instructionEnvelope",
    "proposalPayload",
    "coherenceCheckPayload",
    "requiredDecisionInput",
    "coherenceObligations",
    "doctrineRouteBinding",
    "fiberLifecycleAction",
)
REQUIRED_KCIR_MAPPING_ROW_FIELDS = (
    "sourceKind",
    "targetDomain",
    "targetKind",
)
REQUIRED_PHASE3_COMMAND_SURFACES: Dict[str, Tuple[str, ...]] = {
    "governancePromotionCheck": (
        "cargo",
        "run",
        "--package",
        "premath-cli",
        "--",
        "governance-promotion-check",
    ),
    "kcirMappingCheck": (
        "cargo",
        "run",
        "--package",
        "premath-cli",
        "--",
        "kcir-mapping-check",
    ),
}


def parse_args(root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate runtime orchestration bindings from CONTROL-PLANE-CONTRACT "
            "to DOCTRINE-OP-REGISTRY with explicit Harness/Squeak handoff checks."
        )
    )
    parser.add_argument(
        "--control-plane-contract",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json",
        help="Control-plane contract JSON path",
    )
    parser.add_argument(
        "--doctrine-op-registry",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
        help="Doctrine operation registry JSON path",
    )
    parser.add_argument(
        "--harness-runtime",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "HARNESS-RUNTIME.md",
        help="Harness runtime contract markdown path",
    )
    parser.add_argument(
        "--doctrine-site-input",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json",
        help="Doctrine site input JSON path (worldRouteBindings declaration source)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit deterministic JSON output",
    )
    return parser.parse_args()


def _load_json(path: Path) -> Dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: root must be an object")
    return payload


def _as_sorted_strings(values: Any) -> List[str]:
    if not isinstance(values, list):
        return []
    out: List[str] = []
    for value in values:
        if isinstance(value, str) and value.strip():
            out.append(value.strip())
    return sorted(set(out))


def _extract_runtime_routes(control_plane_contract: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    runtime = control_plane_contract.get("runtimeRouteBindings")
    if not isinstance(runtime, dict):
        raise ValueError("runtimeRouteBindings must be an object")
    routes = runtime.get("requiredOperationRoutes")
    if not isinstance(routes, dict) or not routes:
        raise ValueError("runtimeRouteBindings.requiredOperationRoutes must be a non-empty object")
    out: Dict[str, Dict[str, Any]] = {}
    for route_id in sorted(routes):
        route = routes.get(route_id)
        if not isinstance(route_id, str) or not route_id.strip():
            raise ValueError("runtimeRouteBindings.requiredOperationRoutes keys must be non-empty")
        if not isinstance(route, dict):
            raise ValueError(
                f"runtimeRouteBindings.requiredOperationRoutes.{route_id} must be an object"
            )
        operation_id = route.get("operationId")
        if not isinstance(operation_id, str) or not operation_id.strip():
            raise ValueError(
                f"runtimeRouteBindings.requiredOperationRoutes.{route_id}.operationId must be non-empty"
            )
        required_morphisms = _as_sorted_strings(route.get("requiredMorphisms"))
        if not required_morphisms:
            raise ValueError(
                f"runtimeRouteBindings.requiredOperationRoutes.{route_id}.requiredMorphisms must be non-empty"
            )
        out[route_id.strip()] = {
            "operationId": operation_id.strip(),
            "requiredMorphisms": required_morphisms,
        }
    return out


def _extract_registry_operations(operation_registry: Dict[str, Any]) -> Dict[str, Dict[str, Any]]:
    operations = operation_registry.get("operations")
    if not isinstance(operations, list) or not operations:
        raise ValueError("DOCTRINE-OP-REGISTRY.operations must be a non-empty list")
    out: Dict[str, Dict[str, Any]] = {}
    for idx, row in enumerate(operations):
        if not isinstance(row, dict):
            raise ValueError(f"DOCTRINE-OP-REGISTRY.operations[{idx}] must be an object")
        operation_id = row.get("id")
        if not isinstance(operation_id, str) or not operation_id.strip():
            raise ValueError(f"DOCTRINE-OP-REGISTRY.operations[{idx}].id must be non-empty")
        operation_id = operation_id.strip()
        if operation_id in out:
            raise ValueError(f"duplicate operation id in DOCTRINE-OP-REGISTRY: {operation_id!r}")
        out[operation_id] = {
            "path": str(row.get("path", "")).strip(),
            "morphisms": _as_sorted_strings(row.get("morphisms")),
        }
    return out


def _check_handoff_contract(harness_runtime_text: str) -> List[str]:
    errors: List[str] = []
    if REQUIRED_HANDOFF_HEADING not in harness_runtime_text:
        errors.append(
            "HARNESS-RUNTIME missing required Harness-Squeak composition boundary heading"
        )
    missing_tokens = [
        token for token in REQUIRED_HANDOFF_TOKENS if token not in harness_runtime_text
    ]
    if missing_tokens:
        errors.append(
            "HARNESS-RUNTIME missing required handoff tokens: "
            + ", ".join(missing_tokens)
        )
    return errors


def _check_kcir_mapping_rows(control_plane_contract: Dict[str, Any]) -> Tuple[List[str], List[Dict[str, Any]]]:
    mapping_rows: List[Dict[str, Any]] = []
    errors: List[str] = []
    mappings = control_plane_contract.get("controlPlaneKcirMappings")
    if mappings is None:
        return errors, mapping_rows
    if not isinstance(mappings, dict):
        return ["controlPlaneKcirMappings must be an object when provided"], mapping_rows

    mapping_table = mappings.get("mappingTable")
    if not isinstance(mapping_table, dict):
        return ["controlPlaneKcirMappings.mappingTable must be an object"], mapping_rows

    for row_id in REQUIRED_KCIR_MAPPING_ROWS:
        row = mapping_table.get(row_id)
        row_errors: List[str] = []
        if not isinstance(row, dict):
            row_errors.append("missing row")
            mapping_rows.append(
                {
                    "rowId": row_id,
                    "status": "missing",
                    "errors": row_errors,
                }
            )
            errors.append(f"controlPlaneKcirMappings.mappingTable missing required row: {row_id}")
            continue

        for field in REQUIRED_KCIR_MAPPING_ROW_FIELDS:
            value = row.get(field)
            if not isinstance(value, str) or not value.strip():
                row_errors.append(f"missing field {field}")
        identity_fields = row.get("identityFields")
        if not isinstance(identity_fields, list) or not identity_fields:
            row_errors.append("identityFields must be a non-empty list")
        else:
            for idx, value in enumerate(identity_fields):
                if not isinstance(value, str) or not value.strip():
                    row_errors.append(f"identityFields[{idx}] must be a non-empty string")

        status = "ok" if not row_errors else "invalid"
        mapping_rows.append({"rowId": row_id, "status": status, "errors": row_errors})
        if row_errors:
            errors.append(
                "controlPlaneKcirMappings.mappingTable."
                f"{row_id} invalid: {', '.join(row_errors)}"
            )

    return errors, mapping_rows


def _check_phase3_command_surfaces(
    control_plane_contract: Dict[str, Any]
) -> Tuple[List[str], List[Dict[str, Any]]]:
    command_rows: List[Dict[str, Any]] = []
    errors: List[str] = []

    command_surface = control_plane_contract.get("commandSurface")
    if command_surface is None:
        errors.append("commandSurface is required for phase-3 command-surface parity checks")
        command_surface = {}
    elif not isinstance(command_surface, dict):
        errors.append("commandSurface must be an object")
        command_surface = {}

    for surface_id, expected_tokens in REQUIRED_PHASE3_COMMAND_SURFACES.items():
        row = command_surface.get(surface_id)
        row_errors: List[str] = []
        actual_tokens: List[str] = []
        if not isinstance(row, dict):
            row_errors.append("missing row")
            status = "missing"
        else:
            canonical = row.get("canonicalEntrypoint")
            if not isinstance(canonical, list) or not canonical:
                row_errors.append("canonicalEntrypoint must be a non-empty list")
            else:
                for idx, token in enumerate(canonical):
                    if not isinstance(token, str) or not token.strip():
                        row_errors.append(
                            f"canonicalEntrypoint[{idx}] must be a non-empty string"
                        )
                        continue
                    actual_tokens.append(token.strip())
                if not row_errors and tuple(actual_tokens) != expected_tokens:
                    row_errors.append("canonicalEntrypoint mismatch")
            status = "ok" if not row_errors else "invalid"

        command_rows.append(
            {
                "surfaceId": surface_id,
                "status": status,
                "expectedEntrypoint": list(expected_tokens),
                "actualEntrypoint": actual_tokens,
                "errors": row_errors,
            }
        )
        if row_errors:
            errors.append(
                "commandSurface."
                f"{surface_id} invalid: {', '.join(row_errors)}"
            )

    return errors, command_rows


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _validate_world_registry_check_command(cmd: List[str]) -> None:
    prefix = WORLD_REGISTRY_CHECK_COMMAND_PREFIX
    if tuple(cmd[: len(prefix)]) != prefix:
        raise ValueError(
            "world-registry-check command surface drift: expected prefix "
            f"{list(prefix)!r}, got {cmd!r}"
        )


def _resolve_world_registry_check_command() -> List[str]:
    override = os.environ.get("PREMATH_WORLD_REGISTRY_CHECK_CMD", "").strip()
    if override:
        command = shlex.split(override)
    else:
        command = list(WORLD_REGISTRY_CHECK_COMMAND_PREFIX)
    _validate_world_registry_check_command(command)
    return command


def _run_kernel_world_registry_check(
    *,
    doctrine_site_input: Dict[str, Any],
    doctrine_operation_registry: Dict[str, Any],
    control_plane_contract: Dict[str, Any] | None = None,
    required_route_families: Tuple[str, ...] | None = None,
    required_route_bindings: Dict[str, List[str]] | None = None,
) -> Dict[str, Any]:
    root = _repo_root()
    command = _resolve_world_registry_check_command()

    with tempfile.TemporaryDirectory(prefix="premath-world-registry-check-") as tmp:
        tmp_root = Path(tmp)
        site_input_path = tmp_root / "doctrine_site_input.json"
        operations_path = tmp_root / "doctrine_operation_registry.json"
        site_input_path.write_text(
            json.dumps(doctrine_site_input, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        operations_path.write_text(
            json.dumps(doctrine_operation_registry, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        cmd = [
            *command,
            "--site-input",
            str(site_input_path),
            "--operations",
            str(operations_path),
            "--json",
        ]
        if control_plane_contract is not None:
            control_plane_contract_path = tmp_root / "control_plane_contract.json"
            control_plane_contract_path.write_text(
                json.dumps(control_plane_contract, indent=2, sort_keys=True),
                encoding="utf-8",
            )
            cmd.extend(
                [
                    "--control-plane-contract",
                    str(control_plane_contract_path),
                ]
            )
        if required_route_families:
            for family in required_route_families:
                cmd.extend(["--required-route-family", family])
        if required_route_bindings:
            for family in sorted(required_route_bindings):
                for operation_id in sorted(set(required_route_bindings[family])):
                    cmd.extend(
                        [
                            "--required-route-binding",
                            f"{family}={operation_id}",
                        ]
                    )
        completed = subprocess.run(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode != 0:
            raise ValueError(
                "kernel world-registry-check command failed: "
                f"exit={completed.returncode}, stderr={completed.stderr.strip()!r}"
            )
        stdout = completed.stdout.strip()
        if not stdout:
            raise ValueError("kernel world-registry-check produced empty stdout")
        payload = json.loads(stdout)
        if not isinstance(payload, dict):
            raise ValueError("kernel world-registry-check payload must be an object")
        return payload


def _check_world_route_bindings(
    doctrine_site_input: Dict[str, Any],
    doctrine_operation_registry: Dict[str, Any],
    control_plane_contract: Dict[str, Any],
) -> Tuple[List[str], List[Dict[str, Any]], List[str]]:
    errors: List[str] = []
    rows: List[Dict[str, Any]] = []
    failure_classes: set[str] = set()

    try:
        kernel_payload = _run_kernel_world_registry_check(
            doctrine_site_input=doctrine_site_input,
            doctrine_operation_registry=doctrine_operation_registry,
            control_plane_contract=control_plane_contract,
        )
        kernel_failure_classes = _as_sorted_strings(kernel_payload.get("failureClasses"))
        failure_classes.update(kernel_failure_classes)

        issues = kernel_payload.get("issues")
        if not isinstance(issues, list):
            raise ValueError("kernel payload `issues` must be a list")
        for issue in issues:
            if not isinstance(issue, dict):
                continue
            path = str(issue.get("path", "")).strip()
            message = str(issue.get("message", "")).strip()
            if not path or not message:
                continue
            errors.append(f"{path}: {message}")
            if path.startswith("controlPlaneContract."):
                failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)

        kernel_rows = kernel_payload.get("worldRouteBindings")
        if isinstance(kernel_rows, list):
            for row in kernel_rows:
                if isinstance(row, dict):
                    rows.append(row)
        else:
            errors.append("worldRouteBindings kernel payload missing `worldRouteBindings` list")
            failure_classes.add(FAILURE_CLASS_WORLD_ROUTE_UNBOUND)
    except Exception as exc:  # noqa: BLE001
        errors.append(f"worldRouteBindings kernel check failed: {exc}")
        failure_classes.add(FAILURE_CLASS_WORLD_ROUTE_UNBOUND)

    return errors, rows, sorted(failure_classes)


def evaluate_runtime_orchestration(
    *,
    control_plane_contract: Dict[str, Any],
    operation_registry: Dict[str, Any],
    harness_runtime_text: str,
    doctrine_site_input: Dict[str, Any] | None = None,
) -> Dict[str, Any]:
    errors: List[str] = []
    failure_classes: set[str] = set()
    route_rows: List[Dict[str, Any]] = []
    mapping_rows: List[Dict[str, Any]] = []
    command_rows: List[Dict[str, Any]] = []
    world_rows: List[Dict[str, Any]] = []

    try:
        runtime_routes = _extract_runtime_routes(control_plane_contract)
    except Exception as exc:  # noqa: BLE001
        errors.append(str(exc))
        failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        runtime_routes = {}

    try:
        registry_operations = _extract_registry_operations(operation_registry)
    except Exception as exc:  # noqa: BLE001
        errors.append(str(exc))
        failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        registry_operations = {}

    handoff_errors = _check_handoff_contract(harness_runtime_text)
    if handoff_errors:
        errors.extend(handoff_errors)
        failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)

    mapping_errors, mapping_rows = _check_kcir_mapping_rows(control_plane_contract)
    if mapping_errors:
        errors.extend(mapping_errors)
        failure_classes.add(FAILURE_CLASS_KCIR_MAPPING_CONTRACT_VIOLATION)

    command_errors, command_rows = _check_phase3_command_surfaces(control_plane_contract)
    if command_errors:
        errors.extend(command_errors)
        failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)

    if doctrine_site_input is not None:
        try:
            world_errors, world_rows, world_failure_classes = _check_world_route_bindings(
                doctrine_site_input,
                operation_registry,
                control_plane_contract,
            )
            if world_errors:
                errors.extend(world_errors)
                failure_classes.update(world_failure_classes)
        except Exception as exc:  # noqa: BLE001
            errors.append(str(exc))
            failure_classes.add(FAILURE_CLASS_WORLD_ROUTE_UNBOUND)

    for route_id in sorted(runtime_routes):
        route = runtime_routes[route_id]
        operation_id = route["operationId"]
        required_morphisms = route["requiredMorphisms"]
        operation_row = registry_operations.get(operation_id)
        if operation_row is None:
            errors.append(
                f"missing runtime route operation in DOCTRINE-OP-REGISTRY: {operation_id}"
            )
            failure_classes.add(FAILURE_CLASS_ROUTE_MISSING)
            route_rows.append(
                {
                    "routeId": route_id,
                    "operationId": operation_id,
                    "status": "missing_operation",
                    "requiredMorphisms": required_morphisms,
                    "actualMorphisms": [],
                    "missingMorphisms": required_morphisms,
                }
            )
            continue

        actual_morphisms = _as_sorted_strings(operation_row.get("morphisms", []))
        operation_path = str(operation_row.get("path", "")).strip()
        status_fragments: List[str] = []
        if not operation_path.startswith("tools/ci/"):
            errors.append(
                f"runtime route {route_id} operation path outside canonical CI adapter boundary: {operation_path!r}"
            )
            failure_classes.add(FAILURE_CLASS_CONTRACT_UNBOUND)
            status_fragments.append("path_unbound")

        missing_morphisms = sorted(set(required_morphisms) - set(actual_morphisms))
        if missing_morphisms:
            errors.append(
                f"runtime route {route_id} missing morphisms on {operation_id}: "
                + ", ".join(missing_morphisms)
            )
            failure_classes.add(FAILURE_CLASS_MORPHISM_DRIFT)
            status_fragments.append("missing_morphisms")

        status = "+".join(status_fragments) if status_fragments else "ok"
        route_rows.append(
            {
                "routeId": route_id,
                "operationId": operation_id,
                "operationPath": operation_path,
                "status": status,
                "requiredMorphisms": required_morphisms,
                "actualMorphisms": actual_morphisms,
                "missingMorphisms": missing_morphisms,
            }
        )

    return {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": "rejected" if errors else "accepted",
        "failureClasses": sorted(failure_classes),
        "summary": {
            "requiredRoutes": len(runtime_routes),
            "checkedRoutes": len(route_rows),
            "checkedKcirMappingRows": len(mapping_rows),
            "checkedPhase3CommandSurfaces": len(command_rows),
            "checkedWorldRouteFamilies": len(world_rows),
            "errors": len(errors),
        },
        "routes": route_rows,
        "kcirMappingRows": mapping_rows,
        "phase3CommandSurfaces": command_rows,
        "worldRouteBindings": world_rows,
        "errors": errors,
    }


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    args = parse_args(root)

    try:
        control_plane_contract = _load_json(args.control_plane_contract.resolve())
        operation_registry = _load_json(args.doctrine_op_registry.resolve())
        doctrine_site_input = _load_json(args.doctrine_site_input.resolve())
        harness_runtime_text = args.harness_runtime.resolve().read_text(encoding="utf-8")
        payload = evaluate_runtime_orchestration(
            control_plane_contract=control_plane_contract,
            operation_registry=operation_registry,
            harness_runtime_text=harness_runtime_text,
            doctrine_site_input=doctrine_site_input,
        )
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": SCHEMA,
            "checkKind": CHECK_KIND,
            "result": "rejected",
            "failureClasses": [FAILURE_CLASS_CONTRACT_UNBOUND],
            "summary": {
                "requiredRoutes": 0,
                "checkedRoutes": 0,
                "checkedKcirMappingRows": 0,
                "checkedPhase3CommandSurfaces": 0,
                "checkedWorldRouteFamilies": 0,
                "errors": 1,
            },
            "routes": [],
            "kcirMappingRows": [],
            "phase3CommandSurfaces": [],
            "worldRouteBindings": [],
            "errors": [str(exc)],
        }

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        if payload["result"] == "accepted":
            print(
                "[runtime-orchestration] OK "
                f"(routes={payload['summary']['checkedRoutes']}, errors=0)"
            )
        else:
            print(
                "[runtime-orchestration] FAIL "
                f"(errors={payload['summary']['errors']})"
            )
            for error in payload["errors"]:
                print(f"  - {error}")
    return 0 if payload["result"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
