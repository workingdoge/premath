#!/usr/bin/env python3
"""Validate doctrine op-registry coverage + morphism class parity for MCP tools."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Dict, Set

MCP_TOOL_NAME_RE = re.compile(r'name\s*=\s*"([a-z0-9_]+)"')
MCP_TOOL_BLOCK_RE = re.compile(r"#\[mcp_tool\((.*?)\)\]", re.DOTALL)
MCP_READ_ONLY_RE = re.compile(r"read_only_hint\s*=\s*(true|false)")
REGISTRY_MCP_PREFIX = "op/mcp."
MORPHISM_IDENTITY = "dm.identity"
MORPHISM_PRESENTATION = "dm.presentation.projection"
MORPHISM_EXECUTION = "dm.profile.execution"


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Check parity between MCP tool names declared in "
            "crates/premath-cli/src/commands/mcp_serve.rs and doctrine "
            "operation mappings in specs/premath/draft/DOCTRINE-OP-REGISTRY.json."
        )
    )
    parser.add_argument(
        "--mcp-source",
        type=Path,
        default=root / "crates" / "premath-cli" / "src" / "commands" / "mcp_serve.rs",
        help="Path to mcp_serve.rs source file",
    )
    parser.add_argument(
        "--registry",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
        help="Path to doctrine operation registry JSON",
    )
    return parser.parse_args()


def extract_mcp_tool_names(mcp_source: Path) -> Set[str]:
    return set(extract_mcp_tool_specs(mcp_source).keys())


def extract_mcp_tool_specs(mcp_source: Path) -> Dict[str, bool]:
    text = mcp_source.read_text(encoding="utf-8")
    specs: Dict[str, bool] = {}
    for block in MCP_TOOL_BLOCK_RE.findall(text):
        name_match = MCP_TOOL_NAME_RE.search(block)
        if name_match is None:
            continue
        name = name_match.group(1)
        read_only_match = MCP_READ_ONLY_RE.search(block)
        if read_only_match is None:
            raise ValueError(f"MCP tool {name!r} missing explicit read_only_hint")
        read_only = read_only_match.group(1) == "true"
        if name in specs:
            raise ValueError(f"duplicate MCP tool name: {name}")
        specs[name] = read_only
    return specs


def extract_registry_tool_names(registry_path: Path) -> Set[str]:
    return set(extract_registry_tool_morphisms(registry_path).keys())


def extract_registry_tool_morphisms(registry_path: Path) -> Dict[str, Set[str]]:
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    operations = registry.get("operations")
    if not isinstance(operations, list):
        raise ValueError("registry.operations must be a list")

    names: Dict[str, Set[str]] = {}
    for raw in operations:
        if not isinstance(raw, dict):
            raise ValueError("registry operation entries must be objects")
        op_id = raw.get("id")
        if not isinstance(op_id, str):
            raise ValueError("registry operation id must be a string")
        if not op_id.startswith(REGISTRY_MCP_PREFIX):
            continue
        tool_name = op_id[len(REGISTRY_MCP_PREFIX) :]
        morphisms = raw.get("morphisms")
        if not isinstance(morphisms, list):
            raise ValueError(f"registry morphisms must be a list for {op_id}")
        normalized: Set[str] = set()
        for morphism in morphisms:
            if not isinstance(morphism, str):
                raise ValueError(f"registry morphism entries must be strings for {op_id}")
            normalized.add(morphism)
        if MORPHISM_IDENTITY not in normalized:
            raise ValueError(f"registry MCP op missing {MORPHISM_IDENTITY}: {op_id}")
        names[tool_name] = normalized
    return names


def main() -> int:
    args = parse_args()
    mcp_source = args.mcp_source.resolve()
    registry_path = args.registry.resolve()

    if not mcp_source.exists():
        print(f"[doctrine-mcp-parity] ERROR: MCP source missing: {mcp_source}")
        return 2
    if not registry_path.exists():
        print(f"[doctrine-mcp-parity] ERROR: registry missing: {registry_path}")
        return 2

    try:
        mcp_specs = extract_mcp_tool_specs(mcp_source)
        registry_morphisms = extract_registry_tool_morphisms(registry_path)
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-mcp-parity] ERROR: failed to parse inputs: {exc}")
        return 2

    mcp_names = set(mcp_specs.keys())
    registry_names = set(registry_morphisms.keys())
    missing_in_registry = sorted(mcp_names - registry_names)
    stale_in_registry = sorted(registry_names - mcp_names)
    classification_errors: list[str] = []

    for name in sorted(mcp_names & registry_names):
        morphisms = registry_morphisms[name]
        if mcp_specs[name]:
            if MORPHISM_PRESENTATION not in morphisms:
                classification_errors.append(
                    f"read-only MCP tool {name} missing {MORPHISM_PRESENTATION}"
                )
            if MORPHISM_EXECUTION in morphisms:
                classification_errors.append(
                    f"read-only MCP tool {name} must not declare {MORPHISM_EXECUTION}"
                )
        elif MORPHISM_EXECUTION not in morphisms:
            classification_errors.append(
                f"mutating MCP tool {name} missing {MORPHISM_EXECUTION}"
            )

    if missing_in_registry or stale_in_registry or classification_errors:
        print(
            "[doctrine-mcp-parity] FAIL "
            f"(mcpTools={len(mcp_names)}, registryMcpOps={len(registry_names)})"
        )
        for name in missing_in_registry:
            print(f"  - missing registry mapping for MCP tool: {name}")
        for name in stale_in_registry:
            print(f"  - stale registry mapping without MCP tool: {name}")
        for error in classification_errors:
            print(f"  - {error}")
        return 1

    print(
        "[doctrine-mcp-parity] OK "
        f"(mcpTools={len(mcp_names)}, registryMcpOps={len(registry_names)})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
