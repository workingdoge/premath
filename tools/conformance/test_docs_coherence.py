#!/usr/bin/env python3
"""Unit tests for docs coherence checker parser helpers."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import check_docs_coherence


class DocsCoherenceParserTests(unittest.TestCase):
    def test_parse_capability_registry(self) -> None:
        payload = {
            "schema": 1,
            "registryKind": "premath.capability_registry.v1",
            "executableCapabilities": [
                "capabilities.alpha",
                "capabilities.beta",
            ],
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-cap-registry-") as tmp:
            path = Path(tmp) / "CAPABILITY-REGISTRY.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            values = check_docs_coherence.parse_capability_registry(path)
            self.assertEqual(values, ["capabilities.alpha", "capabilities.beta"])

    def test_extract_section_between(self) -> None:
        text = "prefix START body END suffix"
        self.assertEqual(
            check_docs_coherence.extract_section_between(text, "START", "END").strip(),
            "body",
        )

    def test_parse_mise_task_commands(self) -> None:
        text = """
[tasks.baseline]
run = [
  "mise run fmt",
  "mise run test",
]

[tasks.other]
run = "echo ok"
"""
        commands = check_docs_coherence.parse_mise_task_commands(text, "baseline")
        self.assertEqual(commands, ["mise run fmt", "mise run test"])
        task_ids = check_docs_coherence.parse_baseline_task_ids_from_commands(commands)
        self.assertEqual(task_ids, ["fmt", "test"])

    def test_parse_control_plane_projection_checks(self) -> None:
        payload = {
            "schema": 1,
            "contractKind": "premath.control_plane.contract.v1",
            "requiredGateProjection": {
                "projectionPolicy": "ci-topos-v0",
                "checkOrder": ["baseline", "build"],
            },
        }
        with tempfile.TemporaryDirectory(prefix="docs-coherence-control-plane-") as tmp:
            path = Path(tmp) / "CONTROL-PLANE-CONTRACT.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            checks = check_docs_coherence.parse_control_plane_projection_checks(path)
            self.assertEqual(checks, ["baseline", "build"])

    def test_parse_doctrine_check_commands(self) -> None:
        text = """
[tasks.doctrine-check]
run = [
  "python3 tools/conformance/check_doctrine_site.py",
  "python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf",
]
"""
        commands = check_docs_coherence.parse_mise_task_commands(text, "doctrine-check")
        self.assertEqual(commands, list(check_docs_coherence.EXPECTED_DOCTRINE_CHECK_COMMANDS))

    def test_conditional_normative_entry(self) -> None:
        section = """
- `raw/SQUEAK-SITE` — runtime-location site contracts
  (normative only when `capabilities.squeak_site` is claimed).
"""
        self.assertTrue(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/SQUEAK-SITE",
                "capabilities.squeak_site",
            )
        )
        self.assertFalse(
            check_docs_coherence.verify_conditional_normative_entry(
                section,
                "raw/PREMATH-CI",
                "capabilities.ci_witnesses",
            )
        )

    def test_find_missing_markers(self) -> None:
        text = "alpha beta gamma"
        missing = check_docs_coherence.find_missing_markers(text, ("alpha", "delta", "gamma"))
        self.assertEqual(missing, ["delta"])

    def test_find_missing_markers_all_present(self) -> None:
        text = "alpha beta gamma"
        missing = check_docs_coherence.find_missing_markers(text, ("alpha", "beta"))
        self.assertEqual(missing, [])

    def test_unification_evidence_markers_all_present(self) -> None:
        text = """
### 10.2 Universal factoring rule
there MUST be one deterministic natural transformation:
`eta_F : F => Ev`
### 10.5 Fail-closed factorization boundary
`unification.evidence_factorization.missing`
`unification.evidence_factorization.ambiguous`
`unification.evidence_factorization.unbound`
"""
        missing = check_docs_coherence.find_missing_markers(
            text, check_docs_coherence.UNIFICATION_EVIDENCE_MARKERS
        )
        self.assertEqual(missing, [])

    def test_spec_index_unified_factoring_regex_matches(self) -> None:
        text = (
            "Unified evidence factoring MUST route control-plane artifact families through\n"
            "one attested surface."
        )
        self.assertIsNotNone(
            check_docs_coherence.SPEC_INDEX_UNIFIED_FACTORIZATION_RE.search(text)
        )


if __name__ == "__main__":
    unittest.main()
