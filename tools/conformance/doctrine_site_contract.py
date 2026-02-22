#!/usr/bin/env python3
"""Shared generation/validation helpers for doctrine-site contracts."""

from __future__ import annotations

import hashlib
import json
import re
from collections import deque
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Set, Tuple


SITE_SOURCE_KIND = "premath.doctrine_operation_site.source.v1"
OP_REGISTRY_KIND = "premath.doctrine_operation_registry.v1"

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


def generate_site_map(
    *,
    repo_root: Path,
    source_map_path: Path,
    operation_registry_path: Path | None = None,
) -> Dict[str, Any]:
    source = load_json_object(source_map_path)
    if source.get("schema") != 1:
        raise ValueError(f"{source_map_path}: schema must be 1")
    if source.get("sourceKind") != SITE_SOURCE_KIND:
        raise ValueError(f"{source_map_path}: sourceKind must be {SITE_SOURCE_KIND!r}")

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
    if registry.get("schema") != 1:
        raise ValueError(f"{registry_path}: schema must be 1")
    if registry.get("registryKind") != OP_REGISTRY_KIND:
        raise ValueError(f"{registry_path}: registryKind must be {OP_REGISTRY_KIND!r}")

    nodes_raw = source.get("nodes")
    if not isinstance(nodes_raw, list) or not nodes_raw:
        raise ValueError(f"{source_map_path}: nodes must be a non-empty list")
    covers_raw = source.get("covers")
    if not isinstance(covers_raw, list) or not covers_raw:
        raise ValueError(f"{source_map_path}: covers must be a non-empty list")
    edges_raw = source.get("edges")
    if not isinstance(edges_raw, list) or not edges_raw:
        raise ValueError(f"{source_map_path}: edges must be a non-empty list")

    operations_raw = registry.get("operations")
    if not isinstance(operations_raw, list) or not operations_raw:
        raise ValueError(f"{registry_path}: operations must be a non-empty list")

    doctrine_spec_path = ensure_string(source.get("doctrineSpecPath"), "doctrineSpecPath")

    nodes: List[Dict[str, Any]] = []
    node_ids: Set[str] = set()
    for idx, row in enumerate(nodes_raw):
        node = _parse_node(row, f"{source_map_path}:nodes[{idx}]")
        node_id = node["id"]
        if node_id in node_ids:
            raise ValueError(f"{source_map_path}: duplicate node id {node_id!r}")
        node_ids.add(node_id)

        node_path = (repo_root / node["path"]).resolve()
        if not node_path.exists():
            raise ValueError(f"{source_map_path}: node {node_id!r} path missing: {node['path']}")

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
            f"{registry_path}: parentNodeId {parent_node_id!r} must exist in source nodes"
        )

    for idx, row in enumerate(operations_raw):
        if not isinstance(row, dict):
            raise ValueError(f"{registry_path}: operations[{idx}] must be an object")
        op_id = ensure_string(row.get("id"), f"{registry_path}:operations[{idx}].id")
        if op_id in node_ids:
            raise ValueError(f"{registry_path}: duplicate operation/node id {op_id!r}")
        node_ids.add(op_id)
        generated_operation_ids.append(op_id)

        op_path = ensure_string(row.get("path"), f"{registry_path}:operations[{idx}].path")
        op_kind = ensure_string(row.get("kind"), f"{registry_path}:operations[{idx}].kind")
        edge_id = ensure_string(row.get("edgeId"), f"{registry_path}:operations[{idx}].edgeId")
        morphisms = ensure_string_list(
            row.get("morphisms"), f"{registry_path}:operations[{idx}].morphisms"
        )

        path_on_disk = (repo_root / op_path).resolve()
        if not path_on_disk.exists():
            raise ValueError(
                f"{registry_path}: operation {op_id!r} path missing: {op_path}"
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
        covers.append(_parse_cover(row, f"{source_map_path}:covers[{idx}]"))

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
        edges.append(_parse_edge(row, f"{source_map_path}:edges[{idx}]"))
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

