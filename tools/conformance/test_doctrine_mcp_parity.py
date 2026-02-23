#!/usr/bin/env python3
"""Unit tests for doctrine MCP parity checks."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_doctrine_mcp_parity as parity


ROOT = Path(__file__).resolve().parents[2]
MCP_SOURCE = ROOT / "crates" / "premath-cli" / "src" / "commands" / "mcp_serve.rs"
REGISTRY_PATH = ROOT / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json"


class DoctrineMcpParityTests(unittest.TestCase):
    def test_repository_inputs_are_in_sync(self) -> None:
        mcp_specs = parity.extract_mcp_tool_specs(MCP_SOURCE)
        registry_morphisms = parity.extract_registry_tool_morphisms(REGISTRY_PATH)
        mcp_names = set(mcp_specs.keys())
        registry_names = set(registry_morphisms.keys())
        self.assertEqual(mcp_names - registry_names, set())
        self.assertEqual(registry_names - mcp_names, set())

    def test_detects_missing_registry_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            mcp_source = tmp / "mcp_serve.rs"
            registry = tmp / "registry.json"

            mcp_source.write_text(
                '\n'.join(
                    [
                        '#[mcp_tool(name = "observe_latest", read_only_hint = true)]',
                        '#[mcp_tool(name = "instruction_check", read_only_hint = true)]',
                    ]
                ),
                encoding="utf-8",
            )
            registry.write_text(
                json.dumps(
                    {
                        "operations": [
                            {
                                "id": "op/mcp.observe_latest",
                                "morphisms": ["dm.identity", "dm.presentation.projection"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )

            mcp_names = parity.extract_mcp_tool_names(mcp_source)
            registry_names = parity.extract_registry_tool_names(registry)
            self.assertEqual(mcp_names - registry_names, {"instruction_check"})

    def test_detects_stale_registry_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            mcp_source = tmp / "mcp_serve.rs"
            registry = tmp / "registry.json"

            mcp_source.write_text(
                '#[mcp_tool(name = "issue_list", read_only_hint = true)]\n',
                encoding="utf-8",
            )
            registry.write_text(
                json.dumps(
                    {
                        "operations": [
                            {
                                "id": "op/mcp.issue_list",
                                "morphisms": ["dm.identity", "dm.presentation.projection"],
                            },
                            {
                                "id": "op/mcp.issue_ready",
                                "morphisms": ["dm.identity", "dm.presentation.projection"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )

            mcp_names = parity.extract_mcp_tool_names(mcp_source)
            registry_names = parity.extract_registry_tool_names(registry)
            self.assertEqual(registry_names - mcp_names, {"issue_ready"})

    def test_extracts_read_only_flag(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            mcp_source = tmp / "mcp_serve.rs"
            mcp_source.write_text(
                "\n".join(
                    [
                        "#[mcp_tool(name = \"observe_latest\", read_only_hint = true)]",
                        "#[mcp_tool(name = \"instruction_run\", read_only_hint = false)]",
                    ]
                ),
                encoding="utf-8",
            )
            specs = parity.extract_mcp_tool_specs(mcp_source)
            self.assertEqual(specs, {"observe_latest": True, "instruction_run": False})

    def test_registry_morphisms_are_parsed(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            registry = tmp / "registry.json"
            registry.write_text(
                json.dumps(
                    {
                        "operations": [
                            {
                                "id": "op/mcp.observe_latest",
                                "morphisms": ["dm.identity", "dm.presentation.projection"],
                            },
                            {
                                "id": "op/mcp.instruction_run",
                                "morphisms": ["dm.identity", "dm.profile.execution"],
                            },
                        ]
                    }
                ),
                encoding="utf-8",
            )
            parsed = parity.extract_registry_tool_morphisms(registry)
            self.assertEqual(
                parsed,
                {
                    "observe_latest": {"dm.identity", "dm.presentation.projection"},
                    "instruction_run": {"dm.identity", "dm.profile.execution"},
                },
            )


if __name__ == "__main__":
    unittest.main()
