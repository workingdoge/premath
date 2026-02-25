#!/usr/bin/env python3
"""Run cross-frontend host-action parity vectors using core site-resolve authority."""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURES = ROOT / "tests" / "conformance" / "fixtures" / "frontend-parity"
DEFAULT_CONTROL_PLANE_CONTRACT = ROOT / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
DEFAULT_DOCTRINE_SITE_INPUT = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json"
DEFAULT_DOCTRINE_SITE = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json"
DEFAULT_DOCTRINE_OP_REGISTRY = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"
DEFAULT_CAPABILITY_REGISTRY = ROOT / "specs" / "premath" / "draft" / "CAPABILITY-REGISTRY.json"
SITE_RESOLVE_COMMAND_PREFIX = (
    "cargo",
    "run",
    "--package",
    "premath-cli",
    "--",
    "site-resolve",
)

FAILURE_FRONTEND_REQUIRED_MISSING = "frontend_required_missing"
FAILURE_KERNEL_VERDICT_DRIFT = "frontend_kernel_verdict_drift"
FAILURE_FAILURE_CLASS_PARITY_DRIFT = "frontend_failure_class_parity_drift"
FAILURE_WITNESS_REF_PARITY_DRIFT = "frontend_witness_ref_parity_drift"
FAILURE_WORLD_ROUTE_DRIFT = "frontend_world_route_drift"
FAILURE_TRANSPORT_PROFILE_MISMATCH = "frontend_transport_profile_mismatch"
FAILURE_RESOLVER_WITNESS_PARITY_DRIFT = "frontend_resolver_witness_parity_drift"


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
    return ensure_string(value, label)


def ensure_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    return out


def canonical_list(values: Sequence[str]) -> List[str]:
    return sorted(set(values))


def ensure_bool(value: Any, label: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{label} must be a boolean")
    return value


def parse_resolver_witness(value: Any, label: str) -> Dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError(f"{label} must be an object when provided")
    return {
        "siteId": ensure_string(value.get("siteId"), f"{label}.siteId"),
        "operationId": ensure_string(value.get("operationId"), f"{label}.operationId"),
        "routeFamilyId": ensure_string(value.get("routeFamilyId"), f"{label}.routeFamilyId"),
        "worldId": ensure_string(value.get("worldId"), f"{label}.worldId"),
        "morphismRowId": ensure_string(value.get("morphismRowId"), f"{label}.morphismRowId"),
        "semanticDigest": ensure_string(value.get("semanticDigest"), f"{label}.semanticDigest"),
        "failureClasses": canonical_list(
            ensure_string_list(value.get("failureClasses", []), f"{label}.failureClasses")
        ),
    }


def validate_manifest(fixtures: Path) -> List[str]:
    manifest = load_json(fixtures / "manifest.json")
    suite_id = ensure_string(manifest.get("suiteId"), "manifest.suiteId")
    if suite_id != "frontend-parity":
        raise ValueError("manifest.suiteId must be 'frontend-parity'")
    vectors = ensure_string_list(manifest.get("vectors", []), "manifest.vectors")
    if len(set(vectors)) != len(vectors):
        raise ValueError("manifest.vectors contains duplicates")
    return vectors


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


def _resolve_operation_id_for_host_action(
    host_action_id: str,
    control_plane_contract: Dict[str, Any],
    case_path: Path,
) -> str:
    host_action_surface = control_plane_contract.get("hostActionSurface")
    if not isinstance(host_action_surface, dict):
        raise ValueError(f"{case_path}: control-plane hostActionSurface must be an object")
    required_actions = host_action_surface.get("requiredActions")
    if not isinstance(required_actions, dict):
        raise ValueError(f"{case_path}: control-plane requiredActions must be an object")
    action_row = required_actions.get(host_action_id)
    if not isinstance(action_row, dict):
        raise ValueError(
            f"{case_path}: control-plane missing requiredActions row for hostActionId={host_action_id!r}"
        )
    operation_id = action_row.get("operationId")
    if not isinstance(operation_id, str) or not operation_id.strip():
        raise ValueError(
            f"{case_path}: control-plane requiredActions.{host_action_id}.operationId must be a non-empty string"
        )
    return operation_id.strip()


def _run_kernel_site_resolve(
    *,
    operation_id: str,
    route_family_hint: str | None,
    claimed_capabilities: List[str],
    policy_digest: str,
    profile_id: str,
    context_ref: str,
) -> Dict[str, Any]:
    root = _repo_root()
    command = _resolve_site_resolve_command()
    request: Dict[str, Any] = {
        "schema": 1,
        "requestKind": "premath.site_resolve.request.v1",
        "operationId": operation_id,
        "claimedCapabilities": claimed_capabilities,
        "policyDigest": policy_digest,
        "profileId": profile_id,
        "contextRef": context_ref,
    }
    if route_family_hint is not None:
        request["routeFamilyHint"] = route_family_hint

    with tempfile.TemporaryDirectory(prefix="premath-frontend-parity-site-resolve-") as tmp:
        tmp_root = Path(tmp)
        request_path = tmp_root / "request.json"
        request_path.write_text(json.dumps(request, indent=2, sort_keys=True), encoding="utf-8")

        cmd = [
            *command,
            "--request",
            str(request_path),
            "--doctrine-site-input",
            str(DEFAULT_DOCTRINE_SITE_INPUT),
            "--doctrine-site",
            str(DEFAULT_DOCTRINE_SITE),
            "--doctrine-op-registry",
            str(DEFAULT_DOCTRINE_OP_REGISTRY),
            "--control-plane-contract",
            str(DEFAULT_CONTROL_PLANE_CONTRACT),
            "--capability-registry",
            str(DEFAULT_CAPABILITY_REGISTRY),
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


def _normalize_core_resolver_witness(payload: Dict[str, Any], case_path: Path) -> Dict[str, Any]:
    witness = payload.get("witness")
    if not isinstance(witness, dict):
        raise ValueError(f"{case_path}: core site-resolve payload missing witness object")
    return {
        "siteId": ensure_string(witness.get("siteId"), f"{case_path}: core.witness.siteId"),
        "operationId": ensure_string(
            witness.get("operationId"),
            f"{case_path}: core.witness.operationId",
        ),
        "routeFamilyId": ensure_optional_string(
            witness.get("routeFamilyId"),
            f"{case_path}: core.witness.routeFamilyId",
        ),
        "worldId": ensure_optional_string(
            witness.get("worldId"),
            f"{case_path}: core.witness.worldId",
        ),
        "morphismRowId": ensure_optional_string(
            witness.get("morphismRowId"),
            f"{case_path}: core.witness.morphismRowId",
        ),
        "semanticDigest": ensure_string(
            witness.get("semanticDigest"),
            f"{case_path}: core.witness.semanticDigest",
        ),
        "failureClasses": canonical_list(
            ensure_string_list(
                witness.get("failureClasses", []),
                f"{case_path}: core.witness.failureClasses",
            )
        ),
    }


def evaluate_case(
    case: Dict[str, Any],
    case_path: Path,
    control_plane_contract: Dict[str, Any],
) -> Tuple[str, List[str]]:
    scenario = case.get("scenario")
    if not isinstance(scenario, dict):
        raise ValueError(f"{case_path}: scenario must be an object")
    host_action_id = ensure_string(
        scenario.get("hostActionId"),
        f"{case_path}: scenario.hostActionId",
    )

    required_frontends = ensure_string_list(
        scenario.get("requiredFrontends", []),
        f"{case_path}: scenario.requiredFrontends",
    )
    if not required_frontends:
        raise ValueError(f"{case_path}: scenario.requiredFrontends must be non-empty")
    optional_frontends = ensure_string_list(
        scenario.get("optionalFrontends", []),
        f"{case_path}: scenario.optionalFrontends",
    )
    frontends = scenario.get("frontends")
    if not isinstance(frontends, dict):
        raise ValueError(f"{case_path}: scenario.frontends must be an object")

    rows: Dict[str, Dict[str, Any]] = {}
    for frontend_id, row in frontends.items():
        if not isinstance(frontend_id, str) or not frontend_id.strip():
            raise ValueError(f"{case_path}: scenario.frontends keys must be non-empty strings")
        if not isinstance(row, dict):
            raise ValueError(
                f"{case_path}: scenario.frontends.{frontend_id} must be an object"
            )
        rows[frontend_id.strip()] = {
            "result": ensure_string(row.get("result"), f"{case_path}: {frontend_id}.result"),
            "failureClasses": canonical_list(
                ensure_string_list(
                    row.get("failureClasses", []),
                    f"{case_path}: {frontend_id}.failureClasses",
                )
            ),
            "witnessRefs": canonical_list(
                ensure_string_list(
                    row.get("witnessRefs", []),
                    f"{case_path}: {frontend_id}.witnessRefs",
                )
            ),
            "worldRouteId": ensure_string(
                row.get("worldRouteId"),
                f"{case_path}: {frontend_id}.worldRouteId",
            ),
            "transportProfile": ensure_string(
                row.get("transportProfile"),
                f"{case_path}: {frontend_id}.transportProfile",
            ),
            "resolverWitness": parse_resolver_witness(
                row.get("resolverWitness"),
                f"{case_path}: {frontend_id}.resolverWitness",
            ),
        }

    failure_classes: set[str] = set()
    missing_required = [frontend_id for frontend_id in required_frontends if frontend_id not in rows]
    if missing_required:
        failure_classes.add(FAILURE_FRONTEND_REQUIRED_MISSING)
        # Missing required rows mean we cannot build a complete parity baseline.
        return "rejected", sorted(failure_classes)

    operation_id = _resolve_operation_id_for_host_action(
        host_action_id,
        control_plane_contract,
        case_path,
    )
    site_resolve_cfg = scenario.get("siteResolve")
    if site_resolve_cfg is None:
        site_resolve_cfg = {}
    if not isinstance(site_resolve_cfg, dict):
        raise ValueError(f"{case_path}: scenario.siteResolve must be an object when provided")
    claimed_capabilities = ensure_string_list(
        site_resolve_cfg.get("claimedCapabilities", []),
        f"{case_path}: scenario.siteResolve.claimedCapabilities",
    )
    policy_digest = ensure_optional_string(
        site_resolve_cfg.get("policyDigest"),
        f"{case_path}: scenario.siteResolve.policyDigest",
    ) or "pol1_test"
    profile_id = ensure_optional_string(
        site_resolve_cfg.get("profileId"),
        f"{case_path}: scenario.siteResolve.profileId",
    ) or "cp.bundle.v0"
    context_ref = ensure_optional_string(
        site_resolve_cfg.get("contextRef"),
        f"{case_path}: scenario.siteResolve.contextRef",
    ) or "ctx.main"
    route_family_hint = ensure_optional_string(
        site_resolve_cfg.get("routeFamilyHint"),
        f"{case_path}: scenario.siteResolve.routeFamilyHint",
    )
    if route_family_hint is None:
        route_family_hint = rows[required_frontends[0]]["worldRouteId"]
    enforce_resolver_semantic_digest_raw = site_resolve_cfg.get(
        "enforceResolverSemanticDigest",
        False,
    )
    enforce_resolver_semantic_digest = ensure_bool(
        enforce_resolver_semantic_digest_raw,
        f"{case_path}: scenario.siteResolve.enforceResolverSemanticDigest",
    )
    core_payload = _run_kernel_site_resolve(
        operation_id=operation_id,
        route_family_hint=route_family_hint,
        claimed_capabilities=claimed_capabilities,
        policy_digest=policy_digest,
        profile_id=profile_id,
        context_ref=context_ref,
    )
    core_result = ensure_string(core_payload.get("result"), f"{case_path}: core.result")
    if core_result not in {"accepted", "rejected"}:
        raise ValueError(f"{case_path}: core.result must be accepted|rejected")
    core_failure_classes = canonical_list(
        ensure_string_list(
            core_payload.get("failureClasses", []),
            f"{case_path}: core.failureClasses",
        )
    )
    core_resolver_witness = _normalize_core_resolver_witness(core_payload, case_path)
    selected = core_payload.get("selected")
    core_route_family_id: str | None = None
    if isinstance(selected, dict):
        core_route_family_id = ensure_optional_string(
            selected.get("routeFamilyId"),
            f"{case_path}: core.selected.routeFamilyId",
        )
    if core_route_family_id is None:
        core_route_family_id = core_resolver_witness["routeFamilyId"]

    baseline_ids = required_frontends + [
        frontend_id for frontend_id in optional_frontends if frontend_id in rows
    ]
    baseline = rows[baseline_ids[0]]

    for frontend_id in baseline_ids:
        row = rows[frontend_id]
        if row["result"] != core_result:
            failure_classes.add(FAILURE_KERNEL_VERDICT_DRIFT)
        if row["failureClasses"] != core_failure_classes:
            failure_classes.add(FAILURE_FAILURE_CLASS_PARITY_DRIFT)
        if row["witnessRefs"] != baseline["witnessRefs"]:
            failure_classes.add(FAILURE_WITNESS_REF_PARITY_DRIFT)
        if (
            core_route_family_id is not None
            and row["worldRouteId"] != core_route_family_id
        ):
            failure_classes.add(FAILURE_WORLD_ROUTE_DRIFT)
        if row["transportProfile"] != baseline["transportProfile"]:
            failure_classes.add(FAILURE_TRANSPORT_PROFILE_MISMATCH)
        resolver_witness = row["resolverWitness"]
        if resolver_witness is None:
            failure_classes.add(FAILURE_RESOLVER_WITNESS_PARITY_DRIFT)
        else:
            if (
                resolver_witness["siteId"] != core_resolver_witness["siteId"]
                or resolver_witness["operationId"] != core_resolver_witness["operationId"]
                or resolver_witness["routeFamilyId"] != core_resolver_witness["routeFamilyId"]
                or resolver_witness["worldId"] != core_resolver_witness["worldId"]
                or resolver_witness["morphismRowId"] != core_resolver_witness["morphismRowId"]
                or resolver_witness["failureClasses"] != core_resolver_witness["failureClasses"]
            ):
                failure_classes.add(FAILURE_RESOLVER_WITNESS_PARITY_DRIFT)
            if (
                enforce_resolver_semantic_digest
                and resolver_witness["semanticDigest"] != core_resolver_witness["semanticDigest"]
            ):
                failure_classes.add(FAILURE_RESOLVER_WITNESS_PARITY_DRIFT)

    result = "accepted" if not failure_classes else "rejected"
    return result, sorted(failure_classes)


def run(fixtures: Path) -> int:
    vectors = validate_manifest(fixtures)
    control_plane_contract = load_json(DEFAULT_CONTROL_PLANE_CONTRACT)
    errors: List[str] = []
    executed = 0
    invariance_rows: Dict[str, List[Tuple[str, str, str, Tuple[str, ...]]]] = {}

    for vector_id in vectors:
        try:
            case_path = fixtures / vector_id / "case.json"
            expect_path = fixtures / vector_id / "expect.json"
            case = load_json(case_path)
            expect = load_json(expect_path)

            if case.get("schema") != 1:
                raise ValueError(f"{case_path}: schema must be 1")
            if case.get("suiteId") != "frontend-parity":
                raise ValueError(f"{case_path}: suiteId must be 'frontend-parity'")
            if case.get("vectorId") != vector_id:
                raise ValueError(f"{case_path}: vectorId must equal '{vector_id}'")
            if expect.get("schema") != 1:
                raise ValueError(f"{expect_path}: schema must be 1")

            semantic_scenario_id = ensure_optional_string(
                case.get("semanticScenarioId"),
                f"{case_path}: semanticScenarioId",
            )
            profile = ensure_optional_string(case.get("profile"), f"{case_path}: profile")
            if vector_id.startswith("invariance/"):
                if semantic_scenario_id is None:
                    raise ValueError(
                        f"{case_path}: invariance vectors require semanticScenarioId"
                    )
                if profile is None:
                    raise ValueError(f"{case_path}: invariance vectors require profile")

            expected_result = ensure_string(expect.get("result"), f"{expect_path}: result")
            if expected_result not in {"accepted", "rejected"}:
                raise ValueError(f"{expect_path}: result must be accepted|rejected")
            expected_failure_classes = canonical_list(
                ensure_string_list(
                    expect.get("expectedFailureClasses", []),
                    f"{expect_path}: expectedFailureClasses",
                )
            )

            got_result, got_failure_classes = evaluate_case(
                case,
                case_path,
                control_plane_contract,
            )
            if got_result != expected_result or got_failure_classes != expected_failure_classes:
                raise ValueError(
                    f"expect/got mismatch for {vector_id}\n"
                    f"expect={{'result': {expected_result!r}, 'failureClasses': {expected_failure_classes!r}}}\n"
                    f"got={{'result': {got_result!r}, 'failureClasses': {got_failure_classes!r}}}"
                )

            if semantic_scenario_id is not None:
                invariance_rows.setdefault(semantic_scenario_id, []).append(
                    (
                        vector_id,
                        profile or "default",
                        got_result,
                        tuple(got_failure_classes),
                    )
                )

            print(f"[ok] frontend-parity/{vector_id}")
            executed += 1
        except Exception as exc:  # noqa: BLE001
            errors.append(f"{vector_id}: {exc}")

    for scenario_id in sorted(invariance_rows):
        rows = invariance_rows[scenario_id]
        if len(rows) < 2:
            errors.append(f"invariance scenario {scenario_id!r} has fewer than 2 vectors")
            continue
        baseline_result = rows[0][2]
        baseline_failures = rows[0][3]
        for vector_id, profile, result, failure_classes in rows[1:]:
            if result != baseline_result or failure_classes != baseline_failures:
                errors.append(
                    "invariance mismatch for "
                    f"{scenario_id!r}: baseline=({baseline_result}, {list(baseline_failures)}) "
                    f"vs {vector_id}@{profile}=({result}, {list(failure_classes)})"
                )

    if errors:
        print(f"[frontend-parity-run] FAIL (vectors={executed}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(
        "[frontend-parity-run] OK "
        f"(vectors={executed}, invarianceScenarios={len(invariance_rows)})"
    )
    return 0


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run cross-frontend host-action parity vectors."
    )
    parser.add_argument(
        "--fixtures",
        type=Path,
        default=DEFAULT_FIXTURES,
        help=f"Frontend parity fixture root (default: {DEFAULT_FIXTURES})",
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
        print(f"[frontend-parity-run] ERROR: {exc}")
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
