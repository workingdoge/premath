#!/usr/bin/env python3
"""Compatibility wrapper for runtime orchestration checks.

Canonical authority is `premath runtime-orchestration-check`. This module is
kept as a bounded compatibility alias for existing conformance wiring.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, List

import core_command_client

SCHEMA = 1
CHECK_KIND = "conformance.runtime_orchestration.v1"
RUNTIME_ORCHESTRATION_CHECK_COMMAND_PREFIX = (
    core_command_client.RUNTIME_ORCHESTRATION_CHECK_COMMAND_PREFIX
)


def parse_args(root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Validate runtime orchestration bindings through the canonical "
            "premath runtime-orchestration-check command surface."
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


def _validate_runtime_orchestration_check_command(cmd: List[str]) -> None:
    core_command_client.validate_runtime_orchestration_check_command(cmd)


def _resolve_runtime_orchestration_check_command() -> List[str]:
    return core_command_client.resolve_runtime_orchestration_check_command()


def _run_kernel_runtime_orchestration_check(
    *,
    control_plane_contract: Dict[str, Any],
    operation_registry: Dict[str, Any],
    harness_runtime_text: str,
    doctrine_site_input: Dict[str, Any] | None = None,
) -> Dict[str, Any]:
    return core_command_client.run_runtime_orchestration_check(
        control_plane_contract=control_plane_contract,
        operation_registry=operation_registry,
        harness_runtime_text=harness_runtime_text,
        doctrine_site_input=doctrine_site_input,
    )


def evaluate_runtime_orchestration(
    *,
    control_plane_contract: Dict[str, Any],
    operation_registry: Dict[str, Any],
    harness_runtime_text: str,
    doctrine_site_input: Dict[str, Any] | None = None,
) -> Dict[str, Any]:
    payload = _run_kernel_runtime_orchestration_check(
        control_plane_contract=control_plane_contract,
        operation_registry=operation_registry,
        harness_runtime_text=harness_runtime_text,
        doctrine_site_input=doctrine_site_input,
    )

    result = payload.get("result")
    if not isinstance(result, str) or result.strip() not in {"accepted", "rejected"}:
        raise ValueError("kernel.result must be 'accepted' or 'rejected'")

    failure_classes = payload.get("failureClasses", [])
    if not isinstance(failure_classes, list):
        raise ValueError("kernel.failureClasses must be a list")
    for idx, failure_class in enumerate(failure_classes):
        if not isinstance(failure_class, str) or not failure_class.strip():
            raise ValueError(
                f"kernel.failureClasses[{idx}] must be a non-empty string"
            )

    return payload


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
        if args.json:
            print(
                json.dumps(
                    {
                        "schema": SCHEMA,
                        "checkKind": CHECK_KIND,
                        "result": "error",
                        "errors": [str(exc)],
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print(f"[runtime-orchestration] ERROR: {exc}")
        return 2

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        summary = payload.get("summary") if isinstance(payload.get("summary"), dict) else {}
        checked_routes = int(summary.get("checkedRoutes", 0))
        error_count = int(summary.get("errors", 0))
        if payload.get("result") == "accepted":
            print(
                "[runtime-orchestration] OK "
                f"(routes={checked_routes}, errors=0)"
            )
        else:
            print(
                "[runtime-orchestration] FAIL "
                f"(errors={error_count})"
            )
            payload_errors = payload.get("errors", [])
            if isinstance(payload_errors, list):
                for error in payload_errors:
                    print(f"  - {error}")
    return 0 if payload.get("result") == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
