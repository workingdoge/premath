#!/usr/bin/env python3
"""
Validate doctrine-to-operation site coherence.

Checks:
- doctrine morphism registry extraction from DOCTRINE-INF
- declaration-bearing spec nodes have Doctrine Preservation declarations
- declaration sets match DOCTRINE-SITE.json
- edge/covers coherence and morphism ID validity
- doctrine root reaches every operation node
"""

from __future__ import annotations

import argparse
import json
import re
from collections import deque
from pathlib import Path
from typing import Dict, List, Set, Tuple


DECLARATION_HEADING_RE = re.compile(
    r"^##\s+.*Doctrine Preservation Declaration \(v0\)\s*$", re.MULTILINE
)
SECTION_SPLIT_RE = re.compile(r"^##\s+", re.MULTILINE)
MORPHISM_RE = re.compile(r"`(dm\.[a-z0-9_.-]+)`")
REGISTRY_SECTION_RE = re.compile(
    r"^##\s+3\.\s+Doctrine morphism registry \(v0\)\s*$" r"(.*?)" r"^##\s+",
    re.MULTILINE | re.DOTALL,
)


def load_json(path: Path) -> Dict[str, object]:
    with path.open("r", encoding="utf-8") as fh:
        data = json.load(fh)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: top-level JSON object required")
    return data


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


def ensure_string(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ValueError(f"{label}: non-empty string required")
    return value


def ensure_string_list(value: object, label: str) -> List[str]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: non-empty list required")
    out: List[str] = []
    for i, item in enumerate(value):
        out.append(ensure_string(item, f"{label}[{i}]"))
    return out


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    default_site_map = repo_root / "specs/premath/draft/DOCTRINE-SITE.json"

    parser = argparse.ArgumentParser(description="Validate doctrine-to-operation site coherence.")
    parser.add_argument(
        "--site-map",
        default=str(default_site_map),
        help=f"Path to doctrine site map JSON (default: {default_site_map})",
    )
    args = parser.parse_args()

    errors: List[str] = []

    try:
        site_map_path = Path(args.site_map).resolve()
        site = load_json(site_map_path)
    except Exception as exc:
        print(f"[error] failed to load site map: {exc}")
        return 1

    doctrine_spec_rel = ensure_string(site.get("doctrineSpecPath"), "doctrineSpecPath")
    doctrine_spec_path = (repo_root / doctrine_spec_rel).resolve()
    if not doctrine_spec_path.exists():
        errors.append(f"missing doctrine spec path: {doctrine_spec_rel}")
        doctrine_registry: Set[str] = set()
    else:
        try:
            doctrine_registry = parse_registry(doctrine_spec_path)
        except Exception as exc:
            errors.append(str(exc))
            doctrine_registry = set()

    nodes_raw = site.get("nodes")
    edges_raw = site.get("edges")
    covers_raw = site.get("covers")

    if not isinstance(nodes_raw, list) or not nodes_raw:
        errors.append("nodes: non-empty list required")
        nodes_raw = []
    if not isinstance(edges_raw, list) or not edges_raw:
        errors.append("edges: non-empty list required")
        edges_raw = []
    if not isinstance(covers_raw, list) or not covers_raw:
        errors.append("covers: non-empty list required")
        covers_raw = []

    nodes: Dict[str, Dict[str, object]] = {}
    parsed_declarations: Dict[str, Tuple[Set[str], Set[str]]] = {}
    doctrine_root_id = ""
    operation_ids: List[str] = []

    for idx, node_raw in enumerate(nodes_raw):
        if not isinstance(node_raw, dict):
            errors.append(f"nodes[{idx}]: object required")
            continue
        try:
            node_id = ensure_string(node_raw.get("id"), f"nodes[{idx}].id")
            node_path_rel = ensure_string(node_raw.get("path"), f"nodes[{idx}].path")
            node_kind = ensure_string(node_raw.get("kind"), f"nodes[{idx}].kind")
            requires_decl = bool(node_raw.get("requiresDeclaration", False))
        except Exception as exc:
            errors.append(str(exc))
            continue

        if node_id in nodes:
            errors.append(f"duplicate node id: {node_id}")
            continue

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
        except Exception as exc:
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
                ensure_string_list(declared_map.get("preserved"), f"{node_id}.declares.preserved")
            )
            not_preserved_expected = set(
                ensure_string_list(
                    declared_map.get("notPreserved"), f"{node_id}.declares.notPreserved"
                )
            )
        except Exception as exc:
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

    for idx, edge_raw in enumerate(edges_raw):
        if not isinstance(edge_raw, dict):
            errors.append(f"edges[{idx}]: object required")
            continue

        try:
            edge_id = ensure_string(edge_raw.get("id"), f"edges[{idx}].id")
            from_id = ensure_string(edge_raw.get("from"), f"{edge_id}.from")
            to_id = ensure_string(edge_raw.get("to"), f"{edge_id}.to")
            morphisms = ensure_string_list(edge_raw.get("morphisms"), f"{edge_id}.morphisms")
        except Exception as exc:
            errors.append(str(exc))
            continue

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
    for idx, cover_raw in enumerate(covers_raw):
        if not isinstance(cover_raw, dict):
            errors.append(f"covers[{idx}]: object required")
            continue
        try:
            cover_id = ensure_string(cover_raw.get("id"), f"covers[{idx}].id")
            over = ensure_string(cover_raw.get("over"), f"{cover_id}.over")
            parts = ensure_string_list(cover_raw.get("parts"), f"{cover_id}.parts")
        except Exception as exc:
            errors.append(str(exc))
            continue

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

    if errors:
        for err in errors:
            print(f"[error] {err}")
        print(f"[fail] doctrine site check failed (errors={len(errors)})")
        return 1

    print(
        "[ok] doctrine site check passed "
        f"(nodes={len(nodes)}, edges={len(edge_ids)}, covers={len(cover_ids)}, "
        f"operations={len(operation_ids)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
