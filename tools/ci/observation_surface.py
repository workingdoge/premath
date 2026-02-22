#!/usr/bin/env python3
"""Compatibility wrapper for Observation Surface projection/query."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List, Optional

from control_plane_contract import (
    INSTRUCTION_WITNESS_KIND,
    REQUIRED_DECISION_KIND,
    REQUIRED_WITNESS_KIND,
)

SCHEMA = 1
SURFACE_KIND = "ci.observation.surface.v0"
REQUIRED_EVENT_KIND = f"{REQUIRED_WITNESS_KIND}.summary"
REQUIRED_DECISION_EVENT_KIND = f"{REQUIRED_DECISION_KIND}.summary"
INSTRUCTION_EVENT_KIND = f"{INSTRUCTION_WITNESS_KIND}.summary"


def _load_json(path: Path) -> Optional[Dict[str, Any]]:
    if not path.exists():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"expected object JSON: {path}")
    return data


def build_surface(
    repo_root: Path,
    ciwitness_dir: Path,
    issues_path: Optional[Path] = None,
) -> Dict[str, Any]:
    """Build surface through the core CLI projector (`premath observe-build`)."""

    tool_root = Path(__file__).resolve().parents[2]
    root = repo_root.resolve()
    witness_dir = (
        ciwitness_dir
        if ciwitness_dir.is_absolute()
        else (root / ciwitness_dir).resolve()
    )
    resolved_issues_path = (
        issues_path if issues_path is not None else root / ".premath" / "issues.jsonl"
    )
    if not resolved_issues_path.is_absolute():
        resolved_issues_path = (root / resolved_issues_path).resolve()

    with tempfile.TemporaryDirectory(prefix="premath-observe-build-") as tmp:
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
            str(witness_dir),
            "--issues-path",
            str(resolved_issues_path),
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
            stderr_lines = [line.strip() for line in completed.stderr.splitlines() if line.strip()]
            stdout_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
            message = (
                stderr_lines[-1]
                if stderr_lines
                else (stdout_lines[-1] if stdout_lines else "observe-build failed")
            )
            raise ValueError(f"observe-build failed: {message}")

        payload = json.loads(completed.stdout)
        if not isinstance(payload, dict):
            raise ValueError("observe-build returned non-object JSON payload")
        return payload


def build_events(surface: Dict[str, Any]) -> List[Dict[str, Any]]:
    events: List[Dict[str, Any]] = []
    latest = surface.get("latest") or {}
    delta = latest.get("delta")
    required = latest.get("required")
    decision = latest.get("decision")

    if isinstance(delta, dict):
        events.append({"kind": "ci.delta.v1.summary", "payload": delta})
    if isinstance(required, dict):
        events.append({"kind": REQUIRED_EVENT_KIND, "payload": required})
    if isinstance(decision, dict):
        events.append({"kind": REQUIRED_DECISION_EVENT_KIND, "payload": decision})

    instructions = surface.get("instructions")
    if isinstance(instructions, list):
        for row in instructions:
            if isinstance(row, dict):
                events.append({"kind": INSTRUCTION_EVENT_KIND, "payload": row})

    summary = surface.get("summary")
    if isinstance(summary, dict):
        events.append({"kind": "ci.observation.surface.v0.summary", "payload": summary})
        coherence = summary.get("coherence")
        if isinstance(coherence, dict):
            events.append({"kind": "ci.observation.surface.v0.coherence", "payload": coherence})

    return events


def write_surface(surface: Dict[str, Any], out_json: Path, out_jsonl: Optional[Path]) -> None:
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(surface, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if out_jsonl is None:
        return
    events = build_events(surface)
    out_jsonl.parent.mkdir(parents=True, exist_ok=True)
    with out_jsonl.open("w", encoding="utf-8") as handle:
        for event in events:
            handle.write(json.dumps(event, sort_keys=True, separators=(",", ":"), ensure_ascii=False))
            handle.write("\n")


def query_surface(
    surface: Dict[str, Any],
    mode: str,
    instruction_id: Optional[str] = None,
    projection_digest: Optional[str] = None,
    projection_match: str = "typed",
) -> Dict[str, Any]:
    latest = surface.get("latest") or {}
    summary = surface.get("summary") or {}

    if mode == "latest":
        return {"mode": "latest", "summary": summary, "latest": latest}

    if mode == "needs_attention":
        coherence = summary.get("coherence")
        if not isinstance(coherence, dict):
            coherence = None
        return {
            "mode": "needs_attention",
            "needsAttention": bool(summary.get("needsAttention")),
            "state": summary.get("state"),
            "topFailureClass": summary.get("topFailureClass"),
            "latestProjectionDigest": summary.get("latestProjectionDigest"),
            "latestInstructionId": summary.get("latestInstructionId"),
            "coherence": coherence,
        }

    if mode == "instruction":
        if not instruction_id:
            raise ValueError("--instruction-id is required for mode=instruction")
        instructions = surface.get("instructions")
        if not isinstance(instructions, list):
            raise ValueError("surface.instructions is not a list")
        for row in instructions:
            if isinstance(row, dict) and row.get("instructionId") == instruction_id:
                return {"mode": "instruction", "instruction": row}
        raise ValueError(f"instruction not found: {instruction_id}")

    if mode == "projection":
        if not projection_digest:
            raise ValueError("--projection-digest is required for mode=projection")
        if projection_match not in {"typed", "compatibility_alias"}:
            raise ValueError(
                "--projection-match must be one of: typed, compatibility_alias"
            )
        required = latest.get("required")
        delta = latest.get("delta")
        decision = latest.get("decision")

        def _matches_projection(row: Any) -> bool:
            if not isinstance(row, dict):
                return False
            typed = row.get("typedCoreProjectionDigest")
            alias = row.get("projectionDigest")
            if typed == projection_digest:
                return True
            if projection_match == "compatibility_alias" and alias == projection_digest:
                return True
            return False

        payload = {
            "mode": "projection",
            "projectionDigest": projection_digest,
            "projectionMatch": projection_match,
            "required": required if _matches_projection(required) else None,
            "delta": delta if _matches_projection(delta) else None,
            "decision": decision if _matches_projection(decision) else None,
        }
        if payload["required"] is None and payload["delta"] is None and payload["decision"] is None:
            raise ValueError(f"projection not found in latest surface: {projection_digest}")
        return payload

    raise ValueError(f"unsupported mode: {mode}")


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build or query deterministic CI observation surface.")
    subparsers = parser.add_subparsers(dest="cmd", required=True)

    build = subparsers.add_parser("build", help="Build observation surface artifacts.")
    build.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    build.add_argument(
        "--ciwitness-dir",
        type=Path,
        default=None,
        help="CI witness artifact directory (default: <repo-root>/artifacts/ciwitness).",
    )
    build.add_argument(
        "--out-json",
        type=Path,
        default=None,
        help="Surface JSON output path (default: <repo-root>/artifacts/observation/latest.json).",
    )
    build.add_argument(
        "--out-jsonl",
        type=Path,
        default=None,
        help="Optional JSONL event output path (default: <repo-root>/artifacts/observation/events.jsonl).",
    )
    build.add_argument(
        "--issues-path",
        type=Path,
        default=None,
        help="Issue memory path (default: <repo-root>/.premath/issues.jsonl).",
    )

    query = subparsers.add_parser("query", help="Query observation surface JSON.")
    query.add_argument(
        "--surface",
        type=Path,
        default=default_root / "artifacts" / "observation" / "latest.json",
        help=f"Observation surface JSON path (default: {default_root / 'artifacts' / 'observation' / 'latest.json'}).",
    )
    query.add_argument(
        "--mode",
        choices=["latest", "needs_attention", "instruction", "projection"],
        default="latest",
        help="Query mode (default: latest).",
    )
    query.add_argument(
        "--instruction-id",
        default=None,
        help="Instruction ID for mode=instruction.",
    )
    query.add_argument(
        "--projection-digest",
        default=None,
        help="Projection digest for mode=projection.",
    )
    query.add_argument(
        "--projection-match",
        choices=["typed", "compatibility_alias"],
        default="typed",
        help="Projection lookup mode for mode=projection (default: typed).",
    )

    return parser.parse_args()


def _resolve_path(root: Path, value: Optional[Path], default: Path) -> Path:
    path = default if value is None else value
    return path if path.is_absolute() else (root / path).resolve()


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)

    if args.cmd == "build":
        root = args.repo_root.resolve()
        ciwitness_dir = _resolve_path(root, args.ciwitness_dir, Path("artifacts/ciwitness"))
        out_json = _resolve_path(root, args.out_json, Path("artifacts/observation/latest.json"))
        out_jsonl = _resolve_path(root, args.out_jsonl, Path("artifacts/observation/events.jsonl"))
        issues_path = _resolve_path(root, args.issues_path, Path(".premath/issues.jsonl"))

        surface = build_surface(root, ciwitness_dir, issues_path=issues_path)
        write_surface(surface, out_json, out_jsonl)
        coherence = surface["summary"].get("coherence")
        attention_reasons = (
            coherence.get("attentionReasons", [])
            if isinstance(coherence, dict)
            else []
        )
        print(
            "[observation-surface] OK "
            f"(state={surface['summary']['state']}, needsAttention={surface['summary']['needsAttention']}, "
            f"attentionReasons={len(attention_reasons)}, out={out_json})"
        )
        return 0

    if args.cmd == "query":
        surface_path = args.surface.resolve()
        surface = _load_json(surface_path)
        if surface is None:
            print(f"[error] surface not found: {surface_path}", file=sys.stderr)
            return 2
        try:
            result = query_surface(
                surface,
                mode=args.mode,
                instruction_id=args.instruction_id,
                projection_digest=args.projection_digest,
                projection_match=args.projection_match,
            )
        except ValueError as exc:
            print(f"[error] {exc}", file=sys.stderr)
            return 2
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return 0

    print(f"[error] unsupported command: {args.cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
