#!/usr/bin/env python3
"""Shared generation/validation helpers for doctrine-site contracts."""

from __future__ import annotations

import hashlib
import json
import re
from collections import deque
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Set, Tuple


SITE_INPUT_KIND = "premath.doctrine_operation_site.input.v1"
SITE_SOURCE_KIND = "premath.doctrine_operation_site.source.v1"
OP_REGISTRY_KIND = "premath.doctrine_operation_registry.v1"
SITE_PACKAGE_KIND = "premath.site_package.v1"
WORLD_ROUTE_BINDINGS_KIND = "premath.world_route_bindings.v1"
DOCTRINE_SITE_CUTOVER_KIND = "premath.doctrine_site_cutover.v1"
DEFAULT_CUTOVER_CONTRACT_REL_PATH = Path(
    "specs/premath/draft/DOCTRINE-SITE-CUTOVER.json"
)
OPERATION_CLASS_POLICY_KIND = "premath.doctrine_operation_class_policy.v1"
OP_CLASS_ROUTE_BOUND = "route_bound"
OP_CLASS_READ_ONLY = "read_only_projection"
OP_CLASS_TOOLING_ONLY = "tooling_only"
REQUIRED_OPERATION_CLASSES: Tuple[str, ...] = (
    OP_CLASS_ROUTE_BOUND,
    OP_CLASS_READ_ONLY,
    OP_CLASS_TOOLING_ONLY,
)

DECLARATION_HEADING_RE = re.compile(
    r"^##\s+.*Doctrine Preservation Declaration \(v0\)\s*$", re.MULTILINE
)
SECTION_SPLIT_RE = re.compile(r"^##\s+", re.MULTILINE)
MORPHISM_RE = re.compile(r"`(dm\.[a-z0-9_.-]+)`")
REGISTRY_SECTION_RE = re.compile(
    r"^##\s+3\.\s+Doctrine morphism registry \(v0\)\s*$" r"(.*?)" r"^##\s+",
    re.MULTILINE | re.DOTALL,
)


def load_json_object(path: Path) -> Dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: top-level JSON object required")
    return payload


def ensure_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label}: non-empty string required")
    return value.strip()


def ensure_string_list(value: object, label: str) -> List[str]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: non-empty list required")
    out: List[str] = []
    seen: Set[str] = set()
    for idx, row in enumerate(value):
        parsed = ensure_string(row, f"{label}[{idx}]")
        if parsed in seen:
            raise ValueError(f"{label}: duplicate entry {parsed!r}")
        seen.add(parsed)
        out.append(parsed)
    return out


def ensure_bool(value: object, label: str) -> bool:
    if not isinstance(value, bool):
        raise ValueError(f"{label}: boolean required")
    return value


def canonicalize_cutover_contract(contract: Dict[str, Any], *, label: str) -> Dict[str, Any]:
    if contract.get("schema") != 1:
        raise ValueError(f"{label}.schema must equal 1")
    cutover_kind = ensure_string(contract.get("cutoverKind"), f"{label}.cutoverKind")
    if cutover_kind != DOCTRINE_SITE_CUTOVER_KIND:
        raise ValueError(
            f"{label}.cutoverKind must equal {DOCTRINE_SITE_CUTOVER_KIND!r}"
        )
    cutover_id = ensure_string(contract.get("cutoverId"), f"{label}.cutoverId")
    current_phase_id = ensure_string(
        contract.get("currentPhaseId"), f"{label}.currentPhaseId"
    )
    phases_raw = contract.get("phases")
    if not isinstance(phases_raw, list) or not phases_raw:
        raise ValueError(f"{label}.phases must be a non-empty list")

    phases: List[Dict[str, Any]] = []
    phase_ids: Set[str] = set()
    saw_legacy_enabled = False
    saw_cutover_phase = False
    for idx, row in enumerate(phases_raw):
        if not isinstance(row, dict):
            raise ValueError(f"{label}.phases[{idx}] must be an object")
        phase_id = ensure_string(row.get("phaseId"), f"{label}.phases[{idx}].phaseId")
        if phase_id in phase_ids:
            raise ValueError(f"{label}.phases duplicate phaseId {phase_id!r}")
        phase_ids.add(phase_id)
        phase_mode = ensure_string(row.get("phaseMode"), f"{label}.phases[{idx}].phaseMode")
        allow_legacy_source_kind = ensure_bool(
            row.get("allowLegacySourceKind"),
            f"{label}.phases[{idx}].allowLegacySourceKind",
        )
        allow_operation_registry_override = ensure_bool(
            row.get("allowOperationRegistryOverride"),
            f"{label}.phases[{idx}].allowOperationRegistryOverride",
        )
        if allow_legacy_source_kind or allow_operation_registry_override:
            saw_legacy_enabled = True
            ensure_string(
                row.get("windowStartDate"),
                f"{label}.phases[{idx}].windowStartDate",
            )
            ensure_string(
                row.get("windowEndDate"),
                f"{label}.phases[{idx}].windowEndDate",
            )
        else:
            saw_cutover_phase = True
            ensure_string(
                row.get("effectiveFromDate"),
                f"{label}.phases[{idx}].effectiveFromDate",
            )
        phases.append(
            {
                "phaseId": phase_id,
                "phaseMode": phase_mode,
                "allowLegacySourceKind": allow_legacy_source_kind,
                "allowOperationRegistryOverride": allow_operation_registry_override,
                "windowStartDate": str(row.get("windowStartDate", "")).strip(),
                "windowEndDate": str(row.get("windowEndDate", "")).strip(),
                "effectiveFromDate": str(row.get("effectiveFromDate", "")).strip(),
            }
        )

    if current_phase_id not in phase_ids:
        raise ValueError(
            f"{label}.currentPhaseId {current_phase_id!r} must reference one phases[*].phaseId"
        )
    if not saw_legacy_enabled:
        raise ValueError(
            f"{label}.phases must include at least one bounded compatibility phase"
        )
    if not saw_cutover_phase:
        raise ValueError(f"{label}.phases must include at least one cutover phase")

    return {
        "schema": 1,
        "cutoverKind": DOCTRINE_SITE_CUTOVER_KIND,
        "cutoverId": cutover_id,
        "currentPhaseId": current_phase_id,
        "phases": sorted(phases, key=lambda value: value["phaseId"]),
    }


def load_cutover_contract(
    *,
    repo_root: Path,
    cutover_contract_path: Path | None = None,
) -> Dict[str, Any]:
    path = (
        cutover_contract_path.resolve()
        if cutover_contract_path is not None
        else (repo_root / DEFAULT_CUTOVER_CONTRACT_REL_PATH).resolve()
    )
    contract_raw = load_json_object(path)
    return canonicalize_cutover_contract(contract_raw, label=str(path))


def current_cutover_phase_policy(cutover_contract: Dict[str, Any]) -> Dict[str, Any]:
    current_phase_id = ensure_string(
        cutover_contract.get("currentPhaseId"),
        "cutover.currentPhaseId",
    )
    phases_raw = cutover_contract.get("phases")
    if not isinstance(phases_raw, list) or not phases_raw:
        raise ValueError("cutover.phases must be a non-empty list")
    for row in phases_raw:
        if not isinstance(row, dict):
            continue
        if row.get("phaseId") == current_phase_id:
            return row
    raise ValueError(f"cutover current phase missing: {current_phase_id!r}")


def _canonicalize_site_source(site_map: Dict[str, Any], *, label: str) -> Dict[str, Any]:
    canonical: Dict[str, Any] = {
        "schema": 1,
        "sourceKind": SITE_SOURCE_KIND,
        "siteId": ensure_string(site_map.get("siteId"), f"{label}.siteId"),
        "version": ensure_string(site_map.get("version"), f"{label}.version"),
        "doctrineSpecPath": ensure_string(
            site_map.get("doctrineSpecPath"), f"{label}.doctrineSpecPath"
        ),
    }
    source_kind = site_map.get("sourceKind")
    if source_kind is not None and ensure_string(source_kind, f"{label}.sourceKind") != SITE_SOURCE_KIND:
        raise ValueError(f"{label}.sourceKind must equal {SITE_SOURCE_KIND!r}")
    schema_raw = site_map.get("schema")
    if schema_raw is not None and schema_raw != 1:
        raise ValueError(f"{label}.schema must equal 1")

    nodes_raw = site_map.get("nodes")
    if not isinstance(nodes_raw, list) or not nodes_raw:
        raise ValueError(f"{label}.nodes must be a non-empty list")
    nodes: List[Dict[str, Any]] = []
    node_ids: Set[str] = set()
    for idx, row in enumerate(nodes_raw):
        node = _parse_node(row, f"{label}.nodes[{idx}]")
        node_id = node["id"]
        if node_id in node_ids:
            raise ValueError(f"{label}.nodes duplicate id {node_id!r}")
        node_ids.add(node_id)
        nodes.append(node)
    canonical["nodes"] = sorted(nodes, key=lambda row: row["id"])

    covers_raw = site_map.get("covers")
    if not isinstance(covers_raw, list) or not covers_raw:
        raise ValueError(f"{label}.covers must be a non-empty list")
    covers: List[Dict[str, Any]] = []
    cover_ids: Set[str] = set()
    for idx, row in enumerate(covers_raw):
        cover = _parse_cover(row, f"{label}.covers[{idx}]")
        cover_id = cover["id"]
        if cover_id in cover_ids:
            raise ValueError(f"{label}.covers duplicate id {cover_id!r}")
        cover_ids.add(cover_id)
        cover["parts"] = sorted(set(cover["parts"]))
        covers.append(cover)
    canonical["covers"] = sorted(covers, key=lambda row: row["id"])

    edges_raw = site_map.get("edges")
    if not isinstance(edges_raw, list) or not edges_raw:
        raise ValueError(f"{label}.edges must be a non-empty list")
    edges: List[Dict[str, Any]] = []
    edge_ids: Set[str] = set()
    for idx, row in enumerate(edges_raw):
        edge = _parse_edge(row, f"{label}.edges[{idx}]")
        edge_id = edge["id"]
        if edge_id in edge_ids:
            raise ValueError(f"{label}.edges duplicate id {edge_id!r}")
        edge_ids.add(edge_id)
        edge["morphisms"] = sorted(set(edge["morphisms"]))
        edges.append(edge)
    canonical["edges"] = sorted(edges, key=lambda row: row["id"])

    return canonical


def canonicalize_world_route_bindings(block: Dict[str, Any]) -> Dict[str, Any]:
    canonical: Dict[str, Any] = {
        "schema": 1,
        "bindingKind": WORLD_ROUTE_BINDINGS_KIND,
    }
    schema_raw = block.get("schema")
    if schema_raw is not None and schema_raw != 1:
        raise ValueError("worldRouteBindings.schema must equal 1")
    binding_kind = ensure_string(block.get("bindingKind"), "worldRouteBindings.bindingKind")
    if binding_kind != WORLD_ROUTE_BINDINGS_KIND:
        raise ValueError(
            f"worldRouteBindings.bindingKind must equal {WORLD_ROUTE_BINDINGS_KIND!r}"
        )
    rows_raw = block.get("rows")
    if not isinstance(rows_raw, list) or not rows_raw:
        raise ValueError("worldRouteBindings.rows must be a non-empty list")

    rows: List[Dict[str, Any]] = []
    family_ids: Set[str] = set()
    operation_membership: Dict[str, str] = {}
    for idx, row in enumerate(rows_raw):
        if not isinstance(row, dict):
            raise ValueError(f"worldRouteBindings.rows[{idx}] must be an object")
        route_family_id = ensure_string(
            row.get("routeFamilyId"), f"worldRouteBindings.rows[{idx}].routeFamilyId"
        )
        if route_family_id in family_ids:
            raise ValueError(
                f"worldRouteBindings.rows duplicate routeFamilyId {route_family_id!r}"
            )
        family_ids.add(route_family_id)
        operation_ids = sorted(
            set(
                ensure_string_list(
                    row.get("operationIds"),
                    f"worldRouteBindings.rows[{idx}].operationIds",
                )
            )
        )
        for operation_id in operation_ids:
            existing = operation_membership.get(operation_id)
            if existing is not None and existing != route_family_id:
                raise ValueError(
                    "worldRouteBindings.rows defines duplicate operation binding "
                    f"for {operation_id!r}: {existing!r} vs {route_family_id!r}"
                )
            operation_membership[operation_id] = route_family_id
        rows.append(
            {
                "routeFamilyId": route_family_id,
                "operationIds": operation_ids,
                "worldId": ensure_string(
                    row.get("worldId"),
                    f"worldRouteBindings.rows[{idx}].worldId",
                ),
                "morphismRowId": ensure_string(
                    row.get("morphismRowId"),
                    f"worldRouteBindings.rows[{idx}].morphismRowId",
                ),
                "requiredMorphisms": sorted(
                    set(
                        ensure_string_list(
                            row.get("requiredMorphisms"),
                            f"worldRouteBindings.rows[{idx}].requiredMorphisms",
                        )
                    )
                ),
                "failureClassUnbound": ensure_string(
                    row.get("failureClassUnbound"),
                    f"worldRouteBindings.rows[{idx}].failureClassUnbound",
                ),
            }
        )
    canonical["rows"] = sorted(rows, key=lambda value: value["routeFamilyId"])
    return canonical


def parse_registry(doctrine_spec_path: Path) -> Set[str]:
    text = doctrine_spec_path.read_text(encoding="utf-8")
    section_match = REGISTRY_SECTION_RE.search(text + "\n## ")
    if not section_match:
        raise ValueError(
            f"{doctrine_spec_path}: cannot locate 'Doctrine morphism registry (v0)' section"
        )
    section = section_match.group(1)
    ids = set(MORPHISM_RE.findall(section))
    if not ids:
        raise ValueError(f"{doctrine_spec_path}: no doctrine morphism IDs found in registry")
    return ids


def parse_declaration(spec_path: Path) -> Tuple[Set[str], Set[str]]:
    text = spec_path.read_text(encoding="utf-8")
    heading = DECLARATION_HEADING_RE.search(text)
    if not heading:
        raise ValueError(
            f"{spec_path}: missing 'Doctrine Preservation Declaration (v0)' section"
        )

    tail = text[heading.end() :]
    next_heading = SECTION_SPLIT_RE.search(tail)
    section = tail[: next_heading.start()] if next_heading else tail

    preserved: Set[str] = set()
    not_preserved: Set[str] = set()
    mode = ""

    for line in section.splitlines():
        stripped = line.strip()
        lowered = stripped.lower()
        if lowered.startswith("preserved morphisms"):
            mode = "preserved"
            continue
        if lowered.startswith("not preserved"):
            mode = "not_preserved"
            continue
        if not stripped.startswith("-"):
            continue

        ids = MORPHISM_RE.findall(stripped)
        if not ids:
            continue
        if mode == "preserved":
            preserved.update(ids)
        elif mode == "not_preserved":
            not_preserved.update(ids)

    if not preserved and not not_preserved:
        raise ValueError(
            f"{spec_path}: declaration section present but morphism lists were not parsed"
        )
    return preserved, not_preserved


def _parse_node(node_raw: object, label: str) -> Dict[str, Any]:
    if not isinstance(node_raw, dict):
        raise ValueError(f"{label}: object required")
    node_id = ensure_string(node_raw.get("id"), f"{label}.id")
    node_path = ensure_string(node_raw.get("path"), f"{label}.path")
    node_kind = ensure_string(node_raw.get("kind"), f"{label}.kind")
    requires_declaration = bool(node_raw.get("requiresDeclaration", False))
    out: Dict[str, Any] = {
        "id": node_id,
        "path": node_path,
        "kind": node_kind,
        "requiresDeclaration": requires_declaration,
    }
    return out


def _parse_cover(cover_raw: object, label: str) -> Dict[str, Any]:
    if not isinstance(cover_raw, dict):
        raise ValueError(f"{label}: object required")
    return {
        "id": ensure_string(cover_raw.get("id"), f"{label}.id"),
        "over": ensure_string(cover_raw.get("over"), f"{label}.over"),
        "parts": ensure_string_list(cover_raw.get("parts"), f"{label}.parts"),
    }


def _parse_edge(edge_raw: object, label: str) -> Dict[str, Any]:
    if not isinstance(edge_raw, dict):
        raise ValueError(f"{label}: object required")
    return {
        "id": ensure_string(edge_raw.get("id"), f"{label}.id"),
        "from": ensure_string(edge_raw.get("from"), f"{label}.from"),
        "to": ensure_string(edge_raw.get("to"), f"{label}.to"),
        "morphisms": ensure_string_list(edge_raw.get("morphisms"), f"{label}.morphisms"),
    }


def _parse_operation_class_policy(policy_raw: object, label: str) -> Dict[str, Any]:
    if not isinstance(policy_raw, dict):
        raise ValueError(f"{label}: object required")
    schema = policy_raw.get("schema")
    if schema != 1:
        raise ValueError(f"{label}.schema must equal 1")
    policy_kind = ensure_string(policy_raw.get("policyKind"), f"{label}.policyKind")
    if policy_kind != OPERATION_CLASS_POLICY_KIND:
        raise ValueError(
            f"{label}.policyKind must equal {OPERATION_CLASS_POLICY_KIND!r}"
        )
    classes_raw = policy_raw.get("classes")
    if not isinstance(classes_raw, dict):
        raise ValueError(f"{label}.classes: object required")
    class_rows: Dict[str, Dict[str, Any]] = {}
    for class_id, row in classes_raw.items():
        parsed_class_id = ensure_string(class_id, f"{label}.classes key")
        if not isinstance(row, dict):
            raise ValueError(f"{label}.classes.{parsed_class_id}: object required")
        authority_mode = ensure_string(
            row.get("authorityMode"), f"{label}.classes.{parsed_class_id}.authorityMode"
        )
        resolver_eligible = ensure_bool(
            row.get("resolverEligible"),
            f"{label}.classes.{parsed_class_id}.resolverEligible",
        )
        mutation_allowed = ensure_bool(
            row.get("mutationAllowed"),
            f"{label}.classes.{parsed_class_id}.mutationAllowed",
        )
        class_rows[parsed_class_id] = {
            "authorityMode": authority_mode,
            "resolverEligible": resolver_eligible,
            "mutationAllowed": mutation_allowed,
        }
    missing = sorted(set(REQUIRED_OPERATION_CLASSES).difference(class_rows))
    if missing:
        raise ValueError(f"{label}.classes missing required classes: {missing}")
    if class_rows[OP_CLASS_ROUTE_BOUND]["resolverEligible"] is not True:
        raise ValueError(
            f"{label}.classes.{OP_CLASS_ROUTE_BOUND}.resolverEligible must be true"
        )
    if class_rows[OP_CLASS_READ_ONLY]["mutationAllowed"] is not False:
        raise ValueError(
            f"{label}.classes.{OP_CLASS_READ_ONLY}.mutationAllowed must be false"
        )
    if class_rows[OP_CLASS_READ_ONLY]["resolverEligible"] is not False:
        raise ValueError(
            f"{label}.classes.{OP_CLASS_READ_ONLY}.resolverEligible must be false"
        )
    if class_rows[OP_CLASS_TOOLING_ONLY]["resolverEligible"] is not False:
        raise ValueError(
            f"{label}.classes.{OP_CLASS_TOOLING_ONLY}.resolverEligible must be false"
        )
    return {
        "schema": 1,
        "policyKind": OPERATION_CLASS_POLICY_KIND,
        "classes": {key: class_rows[key] for key in sorted(class_rows)},
    }


def _parse_operation(operation_raw: object, label: str) -> Dict[str, Any]:
    if not isinstance(operation_raw, dict):
        raise ValueError(f"{label}: object required")
    operation_class = ensure_string(
        operation_raw.get("operationClass"),
        f"{label}.operationClass",
    )
    parsed: Dict[str, Any] = {
        "id": ensure_string(operation_raw.get("id"), f"{label}.id"),
        "edgeId": ensure_string(operation_raw.get("edgeId"), f"{label}.edgeId"),
        "path": ensure_string(operation_raw.get("path"), f"{label}.path"),
        "kind": ensure_string(operation_raw.get("kind"), f"{label}.kind"),
        "operationClass": operation_class,
        "morphisms": ensure_string_list(operation_raw.get("morphisms"), f"{label}.morphisms"),
    }
    route_eligibility_raw = operation_raw.get("routeEligibility")
    if route_eligibility_raw is not None:
        if not isinstance(route_eligibility_raw, dict):
            raise ValueError(f"{label}.routeEligibility: object required")
        parsed["routeEligibility"] = {
            "resolverEligible": ensure_bool(
                route_eligibility_raw.get("resolverEligible"),
                f"{label}.routeEligibility.resolverEligible",
            ),
            "worldRouteRequired": ensure_bool(
                route_eligibility_raw.get("worldRouteRequired"),
                f"{label}.routeEligibility.worldRouteRequired",
            ),
            "routeFamilyId": ensure_string(
                route_eligibility_raw.get("routeFamilyId"),
                f"{label}.routeEligibility.routeFamilyId",
            ),
        }
    return parsed


def canonicalize_operation_registry(registry_map: Dict[str, Any]) -> Dict[str, Any]:
    canonical: Dict[str, Any] = {
        "schema": 1,
        "registryKind": ensure_string(registry_map.get("registryKind"), "registryKind"),
        "parentNodeId": ensure_string(registry_map.get("parentNodeId"), "parentNodeId"),
        "coverId": ensure_string(registry_map.get("coverId"), "coverId"),
    }
    if canonical["registryKind"] != OP_REGISTRY_KIND:
        raise ValueError(f"registryKind must be {OP_REGISTRY_KIND!r}")

    operation_class_policy = _parse_operation_class_policy(
        registry_map.get("operationClassPolicy"),
        "operationClassPolicy",
    )
    canonical["operationClassPolicy"] = operation_class_policy

    base_cover_parts = ensure_string_list(
        registry_map.get("baseCoverParts"), "baseCoverParts"
    )
    canonical["baseCoverParts"] = sorted(set(base_cover_parts))

    operations_raw = registry_map.get("operations")
    if not isinstance(operations_raw, list) or not operations_raw:
        raise ValueError("operations: non-empty list required")
    operations: List[Dict[str, Any]] = []
    op_ids: Set[str] = set()
    edge_ids: Set[str] = set()
    for idx, row in enumerate(operations_raw):
        operation = _parse_operation(row, f"operations[{idx}]")
        op_id = operation["id"]
        edge_id = operation["edgeId"]
        operation_class = operation["operationClass"]
        if operation_class not in operation_class_policy["classes"]:
            raise ValueError(
                f"operations[{idx}].operationClass {operation_class!r} is not declared in operationClassPolicy.classes"
            )
        if op_id in op_ids:
            raise ValueError(f"duplicate operation id: {op_id}")
        if edge_id in edge_ids:
            raise ValueError(f"duplicate operation edgeId: {edge_id}")
        op_ids.add(op_id)
        edge_ids.add(edge_id)
        operation["morphisms"] = sorted(set(operation["morphisms"]))
        route_eligibility = operation.get("routeEligibility")
        if operation_class == OP_CLASS_ROUTE_BOUND:
            if not isinstance(route_eligibility, dict):
                raise ValueError(
                    f"operations[{idx}] ({op_id}) route_bound requires routeEligibility object"
                )
            if route_eligibility.get("resolverEligible") is not True:
                raise ValueError(
                    f"operations[{idx}] ({op_id}) route_bound requires routeEligibility.resolverEligible=true"
                )
            if route_eligibility.get("worldRouteRequired") is not True:
                raise ValueError(
                    f"operations[{idx}] ({op_id}) route_bound requires routeEligibility.worldRouteRequired=true"
                )
        else:
            if isinstance(route_eligibility, dict) and route_eligibility.get(
                "resolverEligible"
            ):
                raise ValueError(
                    f"operations[{idx}] ({op_id}) non-route class {operation_class!r} must not be resolverEligible"
                )
        operations.append(operation)
    canonical["operations"] = sorted(operations, key=lambda row: row["id"])
    return canonical


def _collect_world_route_membership(site_input: Dict[str, Any]) -> Dict[str, str]:
    block = site_input.get("worldRouteBindings")
    if block is None:
        return {}
    if not isinstance(block, dict):
        raise ValueError("worldRouteBindings must be an object when present")
    rows = block.get("rows")
    if not isinstance(rows, list):
        raise ValueError("worldRouteBindings.rows must be a list when present")
    membership: Dict[str, str] = {}
    for idx, row in enumerate(rows):
        if not isinstance(row, dict):
            raise ValueError(f"worldRouteBindings.rows[{idx}] must be an object")
        route_family_id = ensure_string(
            row.get("routeFamilyId"), f"worldRouteBindings.rows[{idx}].routeFamilyId"
        )
        operation_ids = ensure_string_list(
            row.get("operationIds"), f"worldRouteBindings.rows[{idx}].operationIds"
        )
        for operation_id in operation_ids:
            existing = membership.get(operation_id)
            if existing is not None and existing != route_family_id:
                raise ValueError(
                    "worldRouteBindings.rows defines duplicate operation binding "
                    f"for {operation_id!r}: {existing!r} vs {route_family_id!r}"
                )
            membership[operation_id] = route_family_id
    return membership


def _validate_operation_classes_against_world_routes(
    *,
    registry: Dict[str, Any],
    world_route_membership: Dict[str, str],
    label: str,
) -> None:
    operations = registry.get("operations")
    if not isinstance(operations, list):
        raise ValueError(f"{label}: operations must be a list")
    by_id: Dict[str, Dict[str, Any]] = {}
    for idx, row in enumerate(operations):
        if not isinstance(row, dict):
            raise ValueError(f"{label}.operations[{idx}] must be an object")
        operation_id = ensure_string(row.get("id"), f"{label}.operations[{idx}].id")
        by_id[operation_id] = row

    for operation_id, route_family_id in sorted(world_route_membership.items()):
        operation = by_id.get(operation_id)
        if operation is None:
            raise ValueError(
                f"{label}: worldRouteBindings references unknown operation {operation_id!r}"
            )
        operation_class = ensure_string(
            operation.get("operationClass"),
            f"{label}.operations[{operation_id}].operationClass",
        )
        if operation_class != OP_CLASS_ROUTE_BOUND:
            raise ValueError(
                f"{label}: operation {operation_id!r} is world-route bound to {route_family_id!r} "
                f"but operationClass is {operation_class!r} (expected {OP_CLASS_ROUTE_BOUND!r})"
            )
        route_eligibility = operation.get("routeEligibility")
        if not isinstance(route_eligibility, dict):
            raise ValueError(
                f"{label}: operation {operation_id!r} missing routeEligibility while world-route bound"
            )
        bound_family = ensure_string(
            route_eligibility.get("routeFamilyId"),
            f"{label}.operations[{operation_id}].routeEligibility.routeFamilyId",
        )
        if bound_family != route_family_id:
            raise ValueError(
                f"{label}: operation {operation_id!r} routeEligibility.routeFamilyId {bound_family!r} "
                f"does not match worldRouteBindings routeFamilyId {route_family_id!r}"
            )

    for operation_id, operation in sorted(by_id.items()):
        operation_class = ensure_string(
            operation.get("operationClass"),
            f"{label}.operations[{operation_id}].operationClass",
        )
        if operation_class == OP_CLASS_ROUTE_BOUND:
            if operation_id not in world_route_membership:
                raise ValueError(
                    f"{label}: route_bound operation {operation_id!r} is missing worldRouteBindings entry"
                )
            continue
        if operation_id in world_route_membership:
            raise ValueError(
                f"{label}: non-route operation {operation_id!r} must not appear in worldRouteBindings"
            )


def canonicalize_site_input(site_input: Dict[str, Any]) -> Dict[str, Any]:
    if site_input.get("schema") != 1:
        raise ValueError("DOCTRINE-SITE-INPUT.schema must equal 1")
    input_kind = ensure_string(site_input.get("inputKind"), "DOCTRINE-SITE-INPUT.inputKind")
    if input_kind != SITE_INPUT_KIND:
        raise ValueError(f"DOCTRINE-SITE-INPUT.inputKind must equal {SITE_INPUT_KIND!r}")
    source_raw = site_input.get("site")
    if not isinstance(source_raw, dict):
        raise ValueError("DOCTRINE-SITE-INPUT.site must be an object")
    registry_raw = site_input.get("operationRegistry")
    if not isinstance(registry_raw, dict):
        raise ValueError("DOCTRINE-SITE-INPUT.operationRegistry must be an object")
    world_routes_raw = site_input.get("worldRouteBindings")
    if not isinstance(world_routes_raw, dict):
        raise ValueError("DOCTRINE-SITE-INPUT.worldRouteBindings must be an object")

    source = _canonicalize_site_source(source_raw, label="DOCTRINE-SITE-INPUT.site")
    registry = canonicalize_operation_registry(registry_raw)
    world_routes = canonicalize_world_route_bindings(world_routes_raw)
    world_route_membership = _collect_world_route_membership(
        {"worldRouteBindings": world_routes}
    )
    _validate_operation_classes_against_world_routes(
        registry=registry,
        world_route_membership=world_route_membership,
        label="DOCTRINE-SITE-INPUT.operationRegistry",
    )
    return {
        "schema": 1,
        "inputKind": SITE_INPUT_KIND,
        "site": source,
        "operationRegistry": registry,
        "worldRouteBindings": world_routes,
    }


def canonical_site_input_json(site_input: Dict[str, Any], *, pretty: bool) -> str:
    canonical = canonicalize_site_input(site_input)
    if pretty:
        return json.dumps(canonical, indent=2, sort_keys=False) + "\n"
    return json.dumps(canonical, separators=(",", ":"), sort_keys=True)


def site_input_digest(site_input: Dict[str, Any]) -> str:
    canonical = canonical_site_input_json(site_input, pretty=False).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def site_input_equality_diff(expected: Dict[str, Any], actual: Dict[str, Any]) -> List[str]:
    expected_canonical = canonicalize_site_input(expected)
    actual_canonical = canonicalize_site_input(actual)
    if expected_canonical == actual_canonical:
        return []
    return [
        "roundtrip mismatch: tracked doctrine site input differs from generated output",
        f"  - expectedDigest={site_input_digest(expected_canonical)}",
        f"  - actualDigest={site_input_digest(actual_canonical)}",
    ]


def _canonicalize_site_package(package: Dict[str, Any], *, label: str) -> Dict[str, Any]:
    if package.get("schema") != 1:
        raise ValueError(f"{label}.schema must equal 1")
    package_kind = ensure_string(package.get("packageKind"), f"{label}.packageKind")
    if package_kind != SITE_PACKAGE_KIND:
        raise ValueError(f"{label}.packageKind must equal {SITE_PACKAGE_KIND!r}")
    package_id = ensure_string(package.get("packageId"), f"{label}.packageId")
    source_raw = package.get("site")
    if not isinstance(source_raw, dict):
        raise ValueError(f"{label}.site must be an object")
    registry_raw = package.get("operationRegistry")
    if not isinstance(registry_raw, dict):
        raise ValueError(f"{label}.operationRegistry must be an object")
    world_routes_raw = package.get("worldRouteBindings")
    if not isinstance(world_routes_raw, dict):
        raise ValueError(f"{label}.worldRouteBindings must be an object")

    source = _canonicalize_site_source(source_raw, label=f"{label}.site")
    registry = canonicalize_operation_registry(registry_raw)
    world_routes = canonicalize_world_route_bindings(world_routes_raw)
    world_route_membership = _collect_world_route_membership(
        {"worldRouteBindings": world_routes}
    )
    _validate_operation_classes_against_world_routes(
        registry=registry,
        world_route_membership=world_route_membership,
        label=f"{label}.operationRegistry",
    )
    return {
        "schema": 1,
        "packageKind": SITE_PACKAGE_KIND,
        "packageId": package_id,
        "site": source,
        "operationRegistry": registry,
        "worldRouteBindings": world_routes,
    }


def generate_site_input_from_packages(*, repo_root: Path, packages_root: Path) -> Dict[str, Any]:
    root = packages_root.resolve()
    if not root.exists() or not root.is_dir():
        raise ValueError(f"site package root missing: {root}")
    package_files = sorted(root.rglob("SITE-PACKAGE.json"))
    if not package_files:
        raise ValueError(f"no site package files found under {root}")
    if len(package_files) != 1:
        listed = ", ".join(str(path.relative_to(repo_root)) for path in package_files)
        raise ValueError(
            "expected exactly one site package file for v0 source layout, found "
            f"{len(package_files)}: {listed}"
        )
    package_path = package_files[0]
    package_raw = load_json_object(package_path)
    canonical_package = _canonicalize_site_package(
        package_raw,
        label=f"{package_path}",
    )
    return canonicalize_site_input(
        {
            "schema": 1,
            "inputKind": SITE_INPUT_KIND,
            "site": canonical_package["site"],
            "operationRegistry": canonical_package["operationRegistry"],
            "worldRouteBindings": canonical_package["worldRouteBindings"],
        }
    )


def _load_source_and_registry(
    *,
    repo_root: Path,
    site_input_path: Path,
    operation_registry_path: Path | None = None,
    cutover_contract_path: Path | None = None,
) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    cutover_contract = load_cutover_contract(
        repo_root=repo_root,
        cutover_contract_path=cutover_contract_path,
    )
    phase_policy = current_cutover_phase_policy(cutover_contract)
    current_phase_id = ensure_string(
        phase_policy.get("phaseId"), "cutover.currentPhase.phaseId"
    )
    allow_legacy_source_kind = ensure_bool(
        phase_policy.get("allowLegacySourceKind"),
        f"cutover.phases[{current_phase_id}].allowLegacySourceKind",
    )
    allow_operation_registry_override = ensure_bool(
        phase_policy.get("allowOperationRegistryOverride"),
        f"cutover.phases[{current_phase_id}].allowOperationRegistryOverride",
    )

    raw_input = load_json_object(site_input_path)

    source: Dict[str, Any]
    registry: Dict[str, Any]

    input_kind = raw_input.get("inputKind")
    if input_kind == SITE_INPUT_KIND:
        canonical_input = canonicalize_site_input(raw_input)
        source = dict(canonical_input["site"])
        if operation_registry_path is not None:
            if not allow_operation_registry_override:
                raise ValueError(
                    "operation-registry override path is disabled by cutover phase "
                    f"{current_phase_id!r}; use generated "
                    "`draft/DOCTRINE-SITE-INPUT.json` authority only"
                )
            registry = load_json_object(operation_registry_path.resolve())
            registry["schema"] = 1
            registry = canonicalize_operation_registry(registry)
            world_route_membership = _collect_world_route_membership(
                {"worldRouteBindings": canonical_input["worldRouteBindings"]}
            )
            _validate_operation_classes_against_world_routes(
                registry=registry,
                world_route_membership=world_route_membership,
                label=f"{site_input_path}:operationRegistry",
            )
        else:
            registry = dict(canonical_input["operationRegistry"])
    else:
        if not allow_legacy_source_kind:
            raise ValueError(
                "legacy doctrine-site sourceKind fallback is disabled by cutover "
                f"phase {current_phase_id!r}; inputKind must equal {SITE_INPUT_KIND!r}"
            )
        if operation_registry_path is not None and not allow_operation_registry_override:
            raise ValueError(
                "operation-registry override path is disabled by cutover phase "
                f"{current_phase_id!r}"
            )
        # Compatibility-only fallback: source map + external operation registry.
        source = dict(raw_input)
        if source.get("sourceKind") != SITE_SOURCE_KIND:
            raise ValueError(
                f"{site_input_path}: inputKind must be {SITE_INPUT_KIND!r} "
                f"or sourceKind must be {SITE_SOURCE_KIND!r}"
            )
        registry_rel = (
            str(operation_registry_path)
            if operation_registry_path is not None
            else ensure_string(source.get("operationRegistryPath"), "operationRegistryPath")
        )
        registry_path = (
            operation_registry_path.resolve()
            if operation_registry_path is not None
            else (repo_root / registry_rel).resolve()
        )
        registry = load_json_object(registry_path)
        registry["schema"] = 1
        registry = canonicalize_operation_registry(registry)

    if source.get("schema") != 1:
        raise ValueError(f"{site_input_path}: source schema must be 1")
    if source.get("sourceKind") != SITE_SOURCE_KIND:
        raise ValueError(f"{site_input_path}: sourceKind must be {SITE_SOURCE_KIND!r}")

    return source, registry


def canonicalize_site_map(site_map: Dict[str, Any]) -> Dict[str, Any]:
    canonical: Dict[str, Any] = {
        "siteId": ensure_string(site_map.get("siteId"), "siteId"),
        "version": ensure_string(site_map.get("version"), "version"),
        "doctrineSpecPath": ensure_string(site_map.get("doctrineSpecPath"), "doctrineSpecPath"),
    }

    nodes_raw = site_map.get("nodes")
    if not isinstance(nodes_raw, list) or not nodes_raw:
        raise ValueError("nodes: non-empty list required")
    covers_raw = site_map.get("covers")
    if not isinstance(covers_raw, list) or not covers_raw:
        raise ValueError("covers: non-empty list required")
    edges_raw = site_map.get("edges")
    if not isinstance(edges_raw, list) or not edges_raw:
        raise ValueError("edges: non-empty list required")

    nodes: List[Dict[str, Any]] = []
    node_ids: Set[str] = set()
    for idx, row in enumerate(nodes_raw):
        node = _parse_node(row, f"nodes[{idx}]")
        node_id = node["id"]
        if node_id in node_ids:
            raise ValueError(f"duplicate node id: {node_id}")
        node_ids.add(node_id)

        if node["requiresDeclaration"]:
            declared = row.get("declares") if isinstance(row, dict) else None
            if not isinstance(declared, dict):
                raise ValueError(f"nodes[{idx}] ({node_id}): declares object required")
            node["declares"] = {
                "preserved": sorted(
                    ensure_string_list(
                        declared.get("preserved"), f"nodes[{idx}] ({node_id}).declares.preserved"
                    )
                ),
                "notPreserved": sorted(
                    ensure_string_list(
                        declared.get("notPreserved"),
                        f"nodes[{idx}] ({node_id}).declares.notPreserved",
                    )
                ),
            }
        nodes.append(node)
    canonical["nodes"] = sorted(nodes, key=lambda row: row["id"])

    covers: List[Dict[str, Any]] = []
    cover_ids: Set[str] = set()
    for idx, row in enumerate(covers_raw):
        cover = _parse_cover(row, f"covers[{idx}]")
        cover_id = cover["id"]
        if cover_id in cover_ids:
            raise ValueError(f"duplicate cover id: {cover_id}")
        cover_ids.add(cover_id)
        cover["parts"] = sorted(set(cover["parts"]))
        covers.append(cover)
    canonical["covers"] = sorted(covers, key=lambda row: row["id"])

    edges: List[Dict[str, Any]] = []
    edge_ids: Set[str] = set()
    for idx, row in enumerate(edges_raw):
        edge = _parse_edge(row, f"edges[{idx}]")
        edge_id = edge["id"]
        if edge_id in edge_ids:
            raise ValueError(f"duplicate edge id: {edge_id}")
        edge_ids.add(edge_id)
        edge["morphisms"] = sorted(set(edge["morphisms"]))
        edges.append(edge)
    canonical["edges"] = sorted(edges, key=lambda row: row["id"])

    return canonical


def canonical_site_map_json(site_map: Dict[str, Any], *, pretty: bool) -> str:
    canonical = canonicalize_site_map(site_map)
    if pretty:
        return json.dumps(canonical, indent=2, sort_keys=False) + "\n"
    return json.dumps(canonical, separators=(",", ":"), sort_keys=True)


def site_map_digest(site_map: Dict[str, Any]) -> str:
    canonical = canonical_site_map_json(site_map, pretty=False).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def generate_operation_registry(
    *,
    repo_root: Path,
    site_input_path: Path,
    operation_registry_path: Path | None = None,
    cutover_contract_path: Path | None = None,
) -> Dict[str, Any]:
    _source, registry = _load_source_and_registry(
        repo_root=repo_root,
        site_input_path=site_input_path,
        operation_registry_path=operation_registry_path,
        cutover_contract_path=cutover_contract_path,
    )
    return canonicalize_operation_registry(registry)


def canonical_operation_registry_json(registry_map: Dict[str, Any], *, pretty: bool) -> str:
    canonical = canonicalize_operation_registry(registry_map)
    if pretty:
        return json.dumps(canonical, indent=2, sort_keys=False) + "\n"
    return json.dumps(canonical, separators=(",", ":"), sort_keys=True)


def operation_registry_digest(registry_map: Dict[str, Any]) -> str:
    canonical = canonical_operation_registry_json(registry_map, pretty=False).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def generate_site_map(
    *,
    repo_root: Path,
    site_input_path: Path,
    operation_registry_path: Path | None = None,
    cutover_contract_path: Path | None = None,
) -> Dict[str, Any]:
    source, registry = _load_source_and_registry(
        repo_root=repo_root,
        site_input_path=site_input_path,
        operation_registry_path=operation_registry_path,
        cutover_contract_path=cutover_contract_path,
    )

    nodes_raw = source.get("nodes")
    if not isinstance(nodes_raw, list) or not nodes_raw:
        raise ValueError(f"{site_input_path}: nodes must be a non-empty list")
    covers_raw = source.get("covers")
    if not isinstance(covers_raw, list) or not covers_raw:
        raise ValueError(f"{site_input_path}: covers must be a non-empty list")
    edges_raw = source.get("edges")
    if not isinstance(edges_raw, list) or not edges_raw:
        raise ValueError(f"{site_input_path}: edges must be a non-empty list")

    operations_raw = registry.get("operations")
    if not isinstance(operations_raw, list) or not operations_raw:
        raise ValueError(f"{site_input_path}: operationRegistry.operations must be a non-empty list")

    doctrine_spec_path = ensure_string(source.get("doctrineSpecPath"), "doctrineSpecPath")

    nodes: List[Dict[str, Any]] = []
    node_ids: Set[str] = set()
    for idx, row in enumerate(nodes_raw):
        node = _parse_node(row, f"{site_input_path}:nodes[{idx}]")
        node_id = node["id"]
        if node_id in node_ids:
            raise ValueError(f"{site_input_path}: duplicate node id {node_id!r}")
        node_ids.add(node_id)

        node_path = (repo_root / node["path"]).resolve()
        if not node_path.exists():
            raise ValueError(f"{site_input_path}: node {node_id!r} path missing: {node['path']}")

        if node["requiresDeclaration"]:
            preserved, not_preserved = parse_declaration(node_path)
            node["declares"] = {
                "preserved": sorted(preserved),
                "notPreserved": sorted(not_preserved),
            }
        nodes.append(node)

    generated_operation_ids: List[str] = []
    generated_operation_edges: List[Dict[str, Any]] = []
    parent_node_id = ensure_string(registry.get("parentNodeId"), "parentNodeId")
    if parent_node_id not in node_ids:
        raise ValueError(
            f"{site_input_path}: operationRegistry.parentNodeId {parent_node_id!r} "
            "must exist in source nodes"
        )

    for idx, row in enumerate(operations_raw):
        operation = _parse_operation(row, f"{site_input_path}:operationRegistry.operations[{idx}]")
        op_id = operation["id"]
        if op_id in node_ids:
            raise ValueError(f"{site_input_path}: duplicate operation/node id {op_id!r}")
        node_ids.add(op_id)
        generated_operation_ids.append(op_id)

        op_path = operation["path"]
        op_kind = operation["kind"]
        edge_id = operation["edgeId"]
        morphisms = operation["morphisms"]

        path_on_disk = (repo_root / op_path).resolve()
        if not path_on_disk.exists():
            raise ValueError(
                f"{site_input_path}: operation {op_id!r} path missing: {op_path}"
            )

        nodes.append(
            {
                "id": op_id,
                "path": op_path,
                "kind": op_kind,
                "requiresDeclaration": False,
            }
        )
        generated_operation_edges.append(
            {
                "id": edge_id,
                "from": parent_node_id,
                "to": op_id,
                "morphisms": morphisms,
            }
        )

    covers: List[Dict[str, Any]] = []
    for idx, row in enumerate(covers_raw):
        covers.append(_parse_cover(row, f"{site_input_path}:covers[{idx}]"))

    generated_cover_id = ensure_string(registry.get("coverId"), "coverId")
    base_cover_parts = ensure_string_list(registry.get("baseCoverParts"), "baseCoverParts")
    covers.append(
        {
            "id": generated_cover_id,
            "over": parent_node_id,
            "parts": base_cover_parts + generated_operation_ids,
        }
    )

    edges: List[Dict[str, Any]] = []
    for idx, row in enumerate(edges_raw):
        edges.append(_parse_edge(row, f"{site_input_path}:edges[{idx}]"))
    edges.extend(generated_operation_edges)

    generated = {
        "siteId": ensure_string(source.get("siteId"), "siteId"),
        "version": ensure_string(source.get("version"), "version"),
        "doctrineSpecPath": doctrine_spec_path,
        "nodes": nodes,
        "covers": covers,
        "edges": edges,
    }
    return canonicalize_site_map(generated)


def validate_site_map(*, repo_root: Path, site_map: Dict[str, Any]) -> List[str]:
    errors: List[str] = []

    try:
        site = canonicalize_site_map(site_map)
    except Exception as exc:  # noqa: BLE001
        return [str(exc)]

    doctrine_spec_rel = ensure_string(site.get("doctrineSpecPath"), "doctrineSpecPath")
    doctrine_spec_path = (repo_root / doctrine_spec_rel).resolve()
    if not doctrine_spec_path.exists():
        errors.append(f"missing doctrine spec path: {doctrine_spec_rel}")
        doctrine_registry: Set[str] = set()
    else:
        try:
            doctrine_registry = parse_registry(doctrine_spec_path)
        except Exception as exc:  # noqa: BLE001
            errors.append(str(exc))
            doctrine_registry = set()

    nodes_raw = site["nodes"]
    edges_raw = site["edges"]
    covers_raw = site["covers"]

    nodes: Dict[str, Dict[str, Any]] = {}
    parsed_declarations: Dict[str, Tuple[Set[str], Set[str]]] = {}
    doctrine_root_id = ""
    operation_ids: List[str] = []

    for idx, node_raw in enumerate(nodes_raw):
        node_id = node_raw["id"]
        node_path_rel = node_raw["path"]
        node_kind = node_raw["kind"]
        requires_decl = bool(node_raw.get("requiresDeclaration", False))
        node_path = (repo_root / node_path_rel).resolve()
        if not node_path.exists():
            errors.append(f"{node_id}: missing path '{node_path_rel}'")

        nodes[node_id] = dict(node_raw)

        if node_kind == "doctrine":
            if doctrine_root_id:
                errors.append(
                    f"multiple doctrine roots found: '{doctrine_root_id}' and '{node_id}'"
                )
            doctrine_root_id = node_id

        if node_kind == "operation":
            operation_ids.append(node_id)

        if not requires_decl:
            continue

        if not node_path.exists():
            continue

        try:
            preserved_actual, not_preserved_actual = parse_declaration(node_path)
        except Exception as exc:  # noqa: BLE001
            errors.append(str(exc))
            continue

        parsed_declarations[node_id] = (preserved_actual, not_preserved_actual)

        if doctrine_registry:
            unknown = sorted(
                (preserved_actual | not_preserved_actual).difference(doctrine_registry)
            )
            if unknown:
                errors.append(
                    f"{node_id}: declaration uses unknown doctrine morphism IDs: {unknown}"
                )

        declared_map = node_raw.get("declares")
        if not isinstance(declared_map, dict):
            errors.append(f"{node_id}: requiresDeclaration=true but 'declares' object missing")
            continue

        try:
            preserved_expected = set(
                ensure_string_list(
                    declared_map.get("preserved"), f"{node_id}.declares.preserved"
                )
            )
            not_preserved_expected = set(
                ensure_string_list(
                    declared_map.get("notPreserved"), f"{node_id}.declares.notPreserved"
                )
            )
        except Exception as exc:  # noqa: BLE001
            errors.append(str(exc))
            continue

        if doctrine_registry:
            unknown_expected = sorted(
                (preserved_expected | not_preserved_expected).difference(doctrine_registry)
            )
            if unknown_expected:
                errors.append(f"{node_id}: map declares unknown doctrine IDs: {unknown_expected}")

        if preserved_expected != preserved_actual:
            errors.append(
                f"{node_id}: preserved mismatch map={sorted(preserved_expected)} "
                f"spec={sorted(preserved_actual)}"
            )
        if not_preserved_expected != not_preserved_actual:
            errors.append(
                f"{node_id}: notPreserved mismatch map={sorted(not_preserved_expected)} "
                f"spec={sorted(not_preserved_actual)}"
            )

    if not doctrine_root_id:
        errors.append("exactly one doctrine root node (kind='doctrine') is required")
    if not operation_ids:
        errors.append("at least one operation node (kind='operation') is required")

    edge_ids: Set[str] = set()
    adjacency: Dict[str, List[str]] = {}
    for edge_raw in edges_raw:
        edge_id = edge_raw["id"]
        from_id = edge_raw["from"]
        to_id = edge_raw["to"]
        morphisms = edge_raw["morphisms"]

        if edge_id in edge_ids:
            errors.append(f"duplicate edge id: {edge_id}")
            continue
        edge_ids.add(edge_id)

        if from_id not in nodes:
            errors.append(f"{edge_id}: from node '{from_id}' is missing")
        if to_id not in nodes:
            errors.append(f"{edge_id}: to node '{to_id}' is missing")

        if doctrine_registry:
            unknown = sorted(set(morphisms).difference(doctrine_registry))
            if unknown:
                errors.append(f"{edge_id}: unknown morphism IDs: {unknown}")

        if to_id in parsed_declarations:
            preserved_to = parsed_declarations[to_id][0]
            missing = sorted(set(morphisms).difference(preserved_to))
            if missing:
                errors.append(
                    f"{edge_id}: morphisms not preserved by destination '{to_id}': {missing}"
                )

        adjacency.setdefault(from_id, []).append(to_id)

    cover_ids: Set[str] = set()
    for cover_raw in covers_raw:
        cover_id = cover_raw["id"]
        over = cover_raw["over"]
        parts = cover_raw["parts"]

        if cover_id in cover_ids:
            errors.append(f"duplicate cover id: {cover_id}")
            continue
        cover_ids.add(cover_id)

        if over not in nodes:
            errors.append(f"{cover_id}: over node '{over}' is missing")
        for part in parts:
            if part not in nodes:
                errors.append(f"{cover_id}: part node '{part}' is missing")

    if doctrine_root_id and operation_ids:
        visited: Set[str] = set()
        queue: deque[str] = deque([doctrine_root_id])
        while queue:
            current = queue.popleft()
            if current in visited:
                continue
            visited.add(current)
            for nxt in adjacency.get(current, []):
                if nxt not in visited:
                    queue.append(nxt)

        for op_id in operation_ids:
            if op_id not in visited:
                errors.append(
                    f"unreachable operation node '{op_id}' from doctrine root '{doctrine_root_id}'"
                )

    return errors


def summarize_site_map(site_map: Dict[str, Any]) -> Tuple[int, int, int, int]:
    canonical = canonicalize_site_map(site_map)
    nodes = canonical["nodes"]
    edges = canonical["edges"]
    covers = canonical["covers"]
    operations = [row for row in nodes if row.get("kind") == "operation"]
    return len(nodes), len(edges), len(covers), len(operations)


def equality_diff(expected: Dict[str, Any], actual: Dict[str, Any]) -> List[str]:
    expected_canonical = canonicalize_site_map(expected)
    actual_canonical = canonicalize_site_map(actual)
    if expected_canonical == actual_canonical:
        return []
    return [
        "roundtrip mismatch: tracked doctrine site map differs from generated output",
        f"  - expectedDigest={site_map_digest(expected_canonical)}",
        f"  - actualDigest={site_map_digest(actual_canonical)}",
    ]


def operation_registry_equality_diff(expected: Dict[str, Any], actual: Dict[str, Any]) -> List[str]:
    expected_canonical = canonicalize_operation_registry(expected)
    actual_canonical = canonicalize_operation_registry(actual)
    if expected_canonical == actual_canonical:
        return []
    return [
        "roundtrip mismatch: tracked doctrine operation registry differs from generated output",
        f"  - expectedDigest={operation_registry_digest(expected_canonical)}",
        f"  - actualDigest={operation_registry_digest(actual_canonical)}",
    ]
