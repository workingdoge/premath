#!/usr/bin/env python3
"""Validate that Observation Surface v0 is a pure projection of CI witnesses."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict

OBSERVATION_SCHEMA = 1
OBSERVATION_KIND = "ci.observation.surface.v0"


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def load_json(path: Path) -> Dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"expected object JSON: {path}")
    return data


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check Observation Surface semantic projection invariants.")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    parser.add_argument(
        "--ciwitness-dir",
        type=Path,
        default=None,
        help="CI witness artifact directory (default: <repo-root>/artifacts/ciwitness).",
    )
    parser.add_argument(
        "--surface",
        type=Path,
        default=None,
        help="Observation surface JSON path (default: <repo-root>/artifacts/observation/latest.json).",
    )
    parser.add_argument(
        "--issues-path",
        type=Path,
        default=None,
        help="Issue memory path (default: <repo-root>/.premath/issues.jsonl).",
    )
    return parser.parse_args()


def _resolve(root: Path, path: Path | None, default_rel: str) -> Path:
    if path is None:
        return (root / default_rel).resolve()
    if path.is_absolute():
        return path
    return (root / path).resolve()


def validate_summary(surface: Dict[str, Any]) -> None:
    summary = surface.get("summary")
    if not isinstance(summary, dict):
        raise ValueError("surface.summary must be an object")

    state = summary.get("state")
    if state not in {"accepted", "rejected", "running", "error", "empty"}:
        raise ValueError(f"invalid summary.state: {state!r}")

    needs_attention = summary.get("needsAttention")
    if not isinstance(needs_attention, bool):
        raise ValueError("summary.needsAttention must be a boolean")

    coherence = summary.get("coherence")
    if coherence is not None and not isinstance(coherence, dict):
        raise ValueError("summary.coherence must be null or an object")

    coherence_needs_attention = False
    if isinstance(coherence, dict):
        attention_reasons = coherence.get("attentionReasons")
        if not isinstance(attention_reasons, list):
            raise ValueError("summary.coherence.attentionReasons must be a list")
        coherence_needs_attention = bool(coherence.get("needsAttention"))

    expected_needs_attention = state in {"rejected", "error"} or coherence_needs_attention
    if needs_attention != expected_needs_attention:
        raise ValueError(
            "summary.needsAttention mismatch "
            f"(expected={expected_needs_attention}, actual={needs_attention})"
        )


def main() -> int:
    tool_root = Path(__file__).resolve().parents[2]
    repo_root = tool_root
    args = parse_args(repo_root)
    root = args.repo_root.resolve()
    ciwitness_dir = _resolve(root, args.ciwitness_dir, "artifacts/ciwitness")
    surface_path = _resolve(root, args.surface, "artifacts/observation/latest.json")
    issues_path = _resolve(root, args.issues_path, ".premath/issues.jsonl")

    if not surface_path.exists():
        print(f"[observation-semantics] FAIL (missing surface: {surface_path})")
        return 1

    try:
        actual = load_json(surface_path)
        with tempfile.TemporaryDirectory(prefix="premath-observe-build-check-") as tmp:
            tmp_root = Path(tmp)
            out_json = tmp_root / "latest.json"
            out_jsonl = tmp_root / "events.jsonl"
            cmd = [
                "cargo",
                "run",
                "--package",
                "premath-cli",
                "--",
                "observe-build",
                "--repo-root",
                str(root),
                "--ciwitness-dir",
                str(ciwitness_dir),
                "--issues-path",
                str(issues_path),
                "--out-json",
                str(out_json),
                "--out-jsonl",
                str(out_jsonl),
                "--json",
            ]
            completed = subprocess.run(
                cmd,
                cwd=tool_root,
                capture_output=True,
                text=True,
            )
            if completed.returncode != 0:
                raise ValueError(
                    "observe-build command failed: "
                    + (completed.stderr.strip() or completed.stdout.strip() or "unknown error")
                )
            expected = json.loads(completed.stdout)
            if not isinstance(expected, dict):
                raise ValueError("observe-build JSON output must be an object")

        if canonical_json(actual) != canonical_json(expected):
            raise ValueError(
                "surface payload mismatch: output is not a pure projection "
                "of current CI witness artifacts"
            )

        if actual.get("schema") != OBSERVATION_SCHEMA:
            raise ValueError(
                f"surface.schema mismatch (expected={OBSERVATION_SCHEMA}, actual={actual.get('schema')})"
            )
        if actual.get("surfaceKind") != OBSERVATION_KIND:
            raise ValueError(
                "surface.surfaceKind mismatch "
                f"(expected={OBSERVATION_KIND!r}, actual={actual.get('surfaceKind')!r})"
            )

        validate_summary(actual)
    except Exception as exc:
        print(f"[observation-semantics] FAIL ({exc})")
        return 1

    print(
        "[observation-semantics] OK "
        f"(surface={surface_path}, ciwitness={ciwitness_dir})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
