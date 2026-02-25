#!/usr/bin/env python3
"""Generate doctrine-site inventory artifacts for newcomer/operator navigation."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Dict, List

import doctrine_site_contract

INVENTORY_KIND = "premath.doctrine_site_inventory.v1"


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Generate doctrine-site inventory JSON + docs index from canonical "
            "site-package and generated doctrine artifacts."
        )
    )
    parser.add_argument(
        "--packages-root",
        type=Path,
        default=repo_root / "specs" / "premath" / "site-packages",
        help="Site-package source root",
    )
    parser.add_argument(
        "--site-input",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json",
        help="Tracked doctrine-site input artifact",
    )
    parser.add_argument(
        "--site-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json",
        help="Tracked doctrine-site map artifact",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
        help="Tracked doctrine operation-registry artifact",
    )
    parser.add_argument(
        "--control-plane-contract",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json",
        help="Control-plane contract for host/runtime command-surface references",
    )
    parser.add_argument(
        "--cutover-contract",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-CUTOVER.json",
        help="Doctrine-site cutover contract",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        default=repo_root / "docs" / "design" / "generated" / "DOCTRINE-SITE-INVENTORY.json",
        help="Generated inventory JSON output path",
    )
    parser.add_argument(
        "--docs-output",
        type=Path,
        default=repo_root / "docs" / "design" / "generated" / "DOCTRINE-SITE-INVENTORY.md",
        help="Generated inventory docs index output path",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check mode; fail on drift instead of writing output files.",
    )
    return parser.parse_args()


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def stable_digest(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def rel(path: Path, repo_root: Path) -> str:
    return path.resolve().relative_to(repo_root).as_posix()


def _build_host_action_bindings(control_plane_contract: Dict[str, Any]) -> Dict[str, List[str]]:
    out: Dict[str, List[str]] = {}
    required_actions = (
        control_plane_contract.get("hostActionSurface", {})
        .get("requiredActions", {})
    )
    if not isinstance(required_actions, dict):
        return out
    for action_id, row in required_actions.items():
        if not isinstance(action_id, str) or not action_id.strip():
            continue
        if not isinstance(row, dict):
            continue
        op_id = row.get("operationId")
        if not isinstance(op_id, str) or not op_id.strip():
            continue
        out.setdefault(op_id.strip(), []).append(action_id.strip())
    for op_id in list(out):
        out[op_id] = sorted(set(out[op_id]))
    return out


def _build_runtime_route_bindings(control_plane_contract: Dict[str, Any]) -> Dict[str, List[str]]:
    out: Dict[str, List[str]] = {}
    required_routes = (
        control_plane_contract.get("runtimeRouteBindings", {})
        .get("requiredOperationRoutes", {})
    )
    if not isinstance(required_routes, dict):
        return out
    for route_id, row in required_routes.items():
        if not isinstance(route_id, str) or not route_id.strip():
            continue
        if not isinstance(row, dict):
            continue
        op_id = row.get("operationId")
        if not isinstance(op_id, str) or not op_id.strip():
            continue
        out.setdefault(op_id.strip(), []).append(route_id.strip())
    for op_id in list(out):
        out[op_id] = sorted(set(out[op_id]))
    return out


def build_inventory(
    *,
    repo_root: Path,
    packages_root: Path,
    site_input_path: Path,
    site_map_path: Path,
    operation_registry_path: Path,
    control_plane_contract_path: Path,
    cutover_contract_path: Path,
) -> Dict[str, Any]:
    generated_input = doctrine_site_contract.generate_site_input_from_packages(
        repo_root=repo_root,
        packages_root=packages_root,
    )
    tracked_input_raw = doctrine_site_contract.load_json_object(site_input_path)
    tracked_input = doctrine_site_contract.canonicalize_site_input(tracked_input_raw)
    generated_input_canonical = doctrine_site_contract.canonicalize_site_input(generated_input)
    if generated_input_canonical != tracked_input:
        raise ValueError(
            "tracked DOCTRINE-SITE-INPUT drifted from site-package source; "
            "run python3 tools/conformance/generate_doctrine_site.py"
        )

    site_map = doctrine_site_contract.canonicalize_site_map(
        doctrine_site_contract.load_json_object(site_map_path)
    )
    operation_registry = doctrine_site_contract.canonicalize_operation_registry(
        doctrine_site_contract.load_json_object(operation_registry_path)
    )
    control_plane_contract = doctrine_site_contract.load_json_object(control_plane_contract_path)
    cutover_contract = doctrine_site_contract.load_cutover_contract(
        repo_root=repo_root,
        cutover_contract_path=cutover_contract_path,
    )
    cutover_phase = doctrine_site_contract.current_cutover_phase_policy(cutover_contract)

    world_route_rows = tracked_input["worldRouteBindings"]["rows"]
    world_by_family = {row["routeFamilyId"]: row for row in world_route_rows}

    host_action_bindings = _build_host_action_bindings(control_plane_contract)
    runtime_route_bindings = _build_runtime_route_bindings(control_plane_contract)

    operations: List[Dict[str, Any]] = []
    for op in operation_registry["operations"]:
        op_id = op["id"]
        route_eligibility = op.get("routeEligibility")
        route_family_id = None
        if isinstance(route_eligibility, dict):
            route_family_id = route_eligibility.get("routeFamilyId")
            if not isinstance(route_family_id, str) or not route_family_id.strip():
                route_family_id = None
            else:
                route_family_id = route_family_id.strip()
        world_row = world_by_family.get(route_family_id) if route_family_id else None

        command_surface_refs = [f"path:{op['path']}"]
        for action_id in host_action_bindings.get(op_id, []):
            command_surface_refs.append(f"hostAction:{action_id}")
        for route_id in runtime_route_bindings.get(op_id, []):
            command_surface_refs.append(f"runtimeRoute:{route_id}")

        operations.append(
            {
                "operationId": op_id,
                "operationClass": op["operationClass"],
                "path": op["path"],
                "kind": op["kind"],
                "edgeId": op["edgeId"],
                "morphisms": list(op["morphisms"]),
                "routeFamilyId": route_family_id,
                "worldId": world_row["worldId"] if world_row else None,
                "morphismRowId": world_row["morphismRowId"] if world_row else None,
                "commandSurfaceRefs": sorted(set(command_surface_refs)),
            }
        )
    operations.sort(key=lambda row: row["operationId"])

    operations_by_route: Dict[str, List[str]] = {}
    for row in operations:
        route_family_id = row.get("routeFamilyId")
        if isinstance(route_family_id, str) and route_family_id:
            operations_by_route.setdefault(route_family_id, []).append(row["operationId"])

    route_families: List[Dict[str, Any]] = []
    for row in world_route_rows:
        route_family_id = row["routeFamilyId"]
        route_families.append(
            {
                "routeFamilyId": route_family_id,
                "worldId": row["worldId"],
                "morphismRowId": row["morphismRowId"],
                "requiredMorphisms": list(row["requiredMorphisms"]),
                "operationIds": sorted(operations_by_route.get(route_family_id, [])),
            }
        )
    route_families.sort(key=lambda row: row["routeFamilyId"])

    inventory: Dict[str, Any] = {
        "schema": 1,
        "inventoryKind": INVENTORY_KIND,
        "source": {
            "packagesRoot": rel(packages_root, repo_root),
            "siteInput": rel(site_input_path, repo_root),
            "siteMap": rel(site_map_path, repo_root),
            "operationRegistry": rel(operation_registry_path, repo_root),
            "controlPlaneContract": rel(control_plane_contract_path, repo_root),
            "cutoverContract": rel(cutover_contract_path, repo_root),
            "siteInputDigest": doctrine_site_contract.site_input_digest(tracked_input),
            "siteMapDigest": doctrine_site_contract.site_map_digest(site_map),
            "operationRegistryDigest": doctrine_site_contract.operation_registry_digest(
                operation_registry
            ),
        },
        "site": {
            "siteId": site_map["siteId"],
            "version": site_map["version"],
            "doctrineSpecPath": site_map["doctrineSpecPath"],
            "nodeCount": len(site_map["nodes"]),
            "coverCount": len(site_map["covers"]),
            "edgeCount": len(site_map["edges"]),
            "operationCount": len(operations),
        },
        "cutover": {
            "cutoverId": cutover_contract["cutoverId"],
            "currentPhaseId": cutover_contract["currentPhaseId"],
            "phaseMode": cutover_phase["phaseMode"],
            "allowLegacySourceKind": cutover_phase["allowLegacySourceKind"],
            "allowOperationRegistryOverride": cutover_phase[
                "allowOperationRegistryOverride"
            ],
        },
        "routeFamilies": route_families,
        "operations": operations,
    }
    inventory["inventoryDigest"] = stable_digest(inventory)
    return inventory


def render_markdown(inventory: Dict[str, Any]) -> str:
    site = inventory["site"]
    source = inventory["source"]
    cutover = inventory["cutover"]

    lines: List[str] = []
    lines.append("# Doctrine Site Inventory (Generated)")
    lines.append("")
    lines.append("This file is generated by `python3 tools/conformance/generate_doctrine_site_inventory.py`.")
    lines.append("Do not edit manually.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- `siteId`: `{site['siteId']}`")
    lines.append(f"- `version`: `{site['version']}`")
    lines.append(f"- `inventoryDigest`: `{inventory['inventoryDigest']}`")
    lines.append(
        f"- topology: `{site['nodeCount']} nodes / {site['coverCount']} covers / {site['edgeCount']} edges / {site['operationCount']} operations`"
    )
    lines.append(
        "- cutover: "
        f"`{cutover['currentPhaseId']}` (mode `{cutover['phaseMode']}`, "
        f"legacySourceKind={str(cutover['allowLegacySourceKind']).lower()}, "
        f"operationRegistryOverride={str(cutover['allowOperationRegistryOverride']).lower()})"
    )
    lines.append("")
    lines.append("## Source Artifacts")
    lines.append("")
    lines.append(f"- site input: `{source['siteInput']}` (`{source['siteInputDigest']}`)")
    lines.append(f"- site map: `{source['siteMap']}` (`{source['siteMapDigest']}`)")
    lines.append(
        f"- operation registry: `{source['operationRegistry']}` (`{source['operationRegistryDigest']}`)"
    )
    lines.append(f"- control-plane contract: `{source['controlPlaneContract']}`")
    lines.append(f"- cutover contract: `{source['cutoverContract']}`")
    lines.append("")
    lines.append("## Route Families")
    lines.append("")
    lines.append("| Route family | World | Morphism row | Operations |")
    lines.append("| --- | --- | --- | --- |")
    for row in inventory["routeFamilies"]:
        operations = ", ".join(f"`{op_id}`" for op_id in row["operationIds"])
        lines.append(
            "| "
            f"`{row['routeFamilyId']}` | `{row['worldId']}` | `{row['morphismRowId']}` | {operations} |"
        )
    lines.append("")
    lines.append("## Operations")
    lines.append("")
    lines.append("| Operation | Class | Route family | World | Command surfaces |")
    lines.append("| --- | --- | --- | --- | --- |")
    for row in inventory["operations"]:
        command_refs = ", ".join(f"`{token}`" for token in row["commandSurfaceRefs"])
        route_family = f"`{row['routeFamilyId']}`" if row["routeFamilyId"] else "-"
        world_id = f"`{row['worldId']}`" if row["worldId"] else "-"
        lines.append(
            "| "
            f"`{row['operationId']}` | `{row['operationClass']}` | {route_family} | {world_id} | {command_refs} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[2]
    packages_root = args.packages_root.resolve()
    site_input_path = args.site_input.resolve()
    site_map_path = args.site_map.resolve()
    operation_registry_path = args.operation_registry.resolve()
    control_plane_contract_path = args.control_plane_contract.resolve()
    cutover_contract_path = args.cutover_contract.resolve()
    json_output_path = args.json_output.resolve()
    docs_output_path = args.docs_output.resolve()

    try:
        inventory = build_inventory(
            repo_root=repo_root,
            packages_root=packages_root,
            site_input_path=site_input_path,
            site_map_path=site_map_path,
            operation_registry_path=operation_registry_path,
            control_plane_contract_path=control_plane_contract_path,
            cutover_contract_path=cutover_contract_path,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-site-inventory] FAIL generate: {exc}")
        return 1

    json_text = json.dumps(inventory, indent=2, sort_keys=False) + "\n"
    docs_text = render_markdown(inventory)

    if args.check:
        errors: List[str] = []
        if not json_output_path.exists() or not docs_output_path.exists():
            if not json_output_path.exists():
                errors.append(f"missing inventory JSON: {json_output_path}")
            if not docs_output_path.exists():
                errors.append(f"missing inventory docs index: {docs_output_path}")
        else:
            current_json = json_output_path.read_text(encoding="utf-8")
            current_docs = docs_output_path.read_text(encoding="utf-8")
            if current_json != json_text:
                errors.append("inventory JSON drifted from generated output")
            if current_docs != docs_text:
                errors.append("inventory docs index drifted from generated output")
        if errors:
            print("[doctrine-site-inventory] FAIL drift")
            for error in errors:
                print(f"  - {error}")
            print("[hint] run: python3 tools/conformance/generate_doctrine_site_inventory.py")
            return 1
        print(
            "[doctrine-site-inventory] OK "
            f"(mode=check, routes={len(inventory['routeFamilies'])}, operations={len(inventory['operations'])})"
        )
        return 0

    json_output_path.parent.mkdir(parents=True, exist_ok=True)
    docs_output_path.parent.mkdir(parents=True, exist_ok=True)
    json_output_path.write_text(json_text, encoding="utf-8")
    docs_output_path.write_text(docs_text, encoding="utf-8")
    print(
        "[doctrine-site-inventory] OK "
        f"(mode=write, routes={len(inventory['routeFamilies'])}, operations={len(inventory['operations'])}, "
        f"json={json_output_path}, docs={docs_output_path})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
