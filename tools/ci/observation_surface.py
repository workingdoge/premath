#!/usr/bin/env python3
"""Build/query a deterministic CI observation surface."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

SCHEMA = 1
SURFACE_KIND = "ci.observation.surface.v0"


def _load_json(path: Path) -> Optional[Dict[str, Any]]:
    if not path.exists():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"expected object JSON: {path}")
    return data


def _string(value: Any) -> Optional[str]:
    if isinstance(value, str):
        trimmed = value.strip()
        if trimmed:
            return trimmed
    return None


def _string_list(value: Any) -> List[str]:
    if not isinstance(value, list):
        return []
    out = [item for item in value if isinstance(item, str) and item.strip()]
    return sorted(set(out))


def _normalize_delta(payload: Dict[str, Any], rel_path: str) -> Dict[str, Any]:
    changed_paths = _string_list(payload.get("changedPaths", []))
    return {
        "ref": rel_path,
        "projectionPolicy": _string(payload.get("projectionPolicy")),
        "projectionDigest": _string(payload.get("projectionDigest")),
        "deltaSource": _string(payload.get("deltaSource")),
        "fromRef": _string(payload.get("fromRef")),
        "toRef": _string(payload.get("toRef")),
        "changedPaths": changed_paths,
        "changedPathCount": len(changed_paths),
    }


def _normalize_required(payload: Dict[str, Any], rel_path: str) -> Dict[str, Any]:
    required_checks = _string_list(payload.get("requiredChecks", []))
    executed_checks = _string_list(payload.get("executedChecks", []))
    failure_classes = _string_list(payload.get("failureClasses", []))
    return {
        "ref": rel_path,
        "witnessKind": _string(payload.get("witnessKind")),
        "projectionPolicy": _string(payload.get("projectionPolicy")),
        "projectionDigest": _string(payload.get("projectionDigest")),
        "verdictClass": _string(payload.get("verdictClass")),
        "requiredChecks": required_checks,
        "executedChecks": executed_checks,
        "failureClasses": failure_classes,
    }


def _normalize_decision(payload: Dict[str, Any], rel_path: str) -> Dict[str, Any]:
    return {
        "ref": rel_path,
        "decisionKind": _string(payload.get("decisionKind")),
        "projectionDigest": _string(payload.get("projectionDigest")),
        "decision": _string(payload.get("decision")),
        "reasonClass": _string(payload.get("reasonClass")),
        "witnessPath": _string(payload.get("witnessPath")),
        "deltaSnapshotPath": _string(payload.get("deltaSnapshotPath")),
        "requiredChecks": _string_list(payload.get("requiredChecks", [])),
    }


def _normalize_instruction(payload: Dict[str, Any], rel_path: str) -> Dict[str, Any]:
    instruction_id = _string(payload.get("instructionId")) or Path(rel_path).stem
    return {
        "ref": rel_path,
        "witnessKind": _string(payload.get("witnessKind")),
        "instructionId": instruction_id,
        "instructionDigest": _string(payload.get("instructionDigest")),
        "instructionClassification": payload.get("instructionClassification"),
        "intent": _string(payload.get("intent")),
        "scope": payload.get("scope"),
        "policyDigest": _string(payload.get("policyDigest")),
        "verdictClass": _string(payload.get("verdictClass")),
        "requiredChecks": _string_list(payload.get("requiredChecks", [])),
        "executedChecks": _string_list(payload.get("executedChecks", [])),
        "failureClasses": _string_list(payload.get("failureClasses", [])),
    }


def _derive_state(
    required: Optional[Dict[str, Any]],
    decision: Optional[Dict[str, Any]],
    instructions: List[Dict[str, Any]],
) -> Dict[str, Any]:
    top_failure: Optional[str] = None

    if decision is not None:
        decision_value = decision.get("decision")
        if decision_value == "accept":
            state = "accepted"
        elif decision_value == "reject":
            state = "rejected"
        else:
            state = "error"
        top_failure = decision.get("reasonClass")
    elif required is not None:
        verdict = required.get("verdictClass")
        if verdict == "accepted":
            state = "running"
        elif verdict == "rejected":
            state = "rejected"
        else:
            state = "error"
        failures = required.get("failureClasses") or []
        if isinstance(failures, list) and failures:
            top_failure = failures[0]
    elif instructions:
        latest = instructions[-1]
        verdict = latest.get("verdictClass")
        if verdict == "accepted":
            state = "running"
        elif verdict == "rejected":
            state = "rejected"
        else:
            state = "error"
        failures = latest.get("failureClasses") or []
        if isinstance(failures, list) and failures:
            top_failure = failures[0]
    else:
        state = "empty"

    if top_failure is None and state == "rejected":
        top_failure = "rejected_without_reason"

    return {
        "state": state,
        "needsAttention": state in {"rejected", "error"},
        "topFailureClass": top_failure,
    }


def _relative(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def build_surface(repo_root: Path, ciwitness_dir: Path) -> Dict[str, Any]:
    required_path = ciwitness_dir / "latest-required.json"
    delta_path = ciwitness_dir / "latest-delta.json"
    decision_path = ciwitness_dir / "latest-decision.json"

    required_payload = _load_json(required_path)
    delta_payload = _load_json(delta_path)
    decision_payload = _load_json(decision_path)

    required = (
        _normalize_required(required_payload, _relative(required_path, repo_root))
        if required_payload is not None
        else None
    )
    delta = (
        _normalize_delta(delta_payload, _relative(delta_path, repo_root))
        if delta_payload is not None
        else None
    )
    decision = (
        _normalize_decision(decision_payload, _relative(decision_path, repo_root))
        if decision_payload is not None
        else None
    )

    instructions: List[Dict[str, Any]] = []
    if ciwitness_dir.exists():
        for path in sorted(ciwitness_dir.glob("*.json")):
            name = path.name
            if name.startswith("latest-") or name.startswith("proj1_"):
                continue
            payload = _load_json(path)
            if payload is None:
                continue
            if payload.get("witnessKind") != "ci.instruction.v1":
                continue
            instructions.append(_normalize_instruction(payload, _relative(path, repo_root)))
    instructions.sort(key=lambda row: row["instructionId"])

    latest_projection = None
    for candidate in (
        decision.get("projectionDigest") if decision else None,
        required.get("projectionDigest") if required else None,
        delta.get("projectionDigest") if delta else None,
    ):
        if isinstance(candidate, str) and candidate:
            latest_projection = candidate
            break

    latest_instruction_id = instructions[-1]["instructionId"] if instructions else None
    state = _derive_state(required, decision, instructions)
    summary = {
        **state,
        "latestProjectionDigest": latest_projection,
        "latestInstructionId": latest_instruction_id,
        "requiredCheckCount": len(required["requiredChecks"]) if required else 0,
        "executedCheckCount": len(required["executedChecks"]) if required else 0,
        "changedPathCount": delta["changedPathCount"] if delta else 0,
    }

    surface = {
        "schema": SCHEMA,
        "surfaceKind": SURFACE_KIND,
        "summary": summary,
        "latest": {
            "delta": delta,
            "required": required,
            "decision": decision,
        },
        "instructions": instructions,
    }
    return surface


def build_events(surface: Dict[str, Any]) -> List[Dict[str, Any]]:
    events: List[Dict[str, Any]] = []
    latest = surface.get("latest") or {}
    delta = latest.get("delta")
    required = latest.get("required")
    decision = latest.get("decision")

    if isinstance(delta, dict):
        events.append({"kind": "ci.delta.v1.summary", "payload": delta})
    if isinstance(required, dict):
        events.append({"kind": "ci.required.v1.summary", "payload": required})
    if isinstance(decision, dict):
        events.append({"kind": "ci.required.decision.v1.summary", "payload": decision})

    instructions = surface.get("instructions")
    if isinstance(instructions, list):
        for row in instructions:
            if isinstance(row, dict):
                events.append({"kind": "ci.instruction.v1.summary", "payload": row})

    summary = surface.get("summary")
    if isinstance(summary, dict):
        events.append({"kind": "ci.observation.surface.v0.summary", "payload": summary})

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
) -> Dict[str, Any]:
    latest = surface.get("latest") or {}
    summary = surface.get("summary") or {}

    if mode == "latest":
        return {"mode": "latest", "summary": summary, "latest": latest}

    if mode == "needs_attention":
        return {
            "mode": "needs_attention",
            "needsAttention": bool(summary.get("needsAttention")),
            "state": summary.get("state"),
            "topFailureClass": summary.get("topFailureClass"),
            "latestProjectionDigest": summary.get("latestProjectionDigest"),
            "latestInstructionId": summary.get("latestInstructionId"),
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
        required = latest.get("required")
        delta = latest.get("delta")
        decision = latest.get("decision")

        payload = {
            "mode": "projection",
            "projectionDigest": projection_digest,
            "required": required if isinstance(required, dict) and required.get("projectionDigest") == projection_digest else None,
            "delta": delta if isinstance(delta, dict) and delta.get("projectionDigest") == projection_digest else None,
            "decision": decision if isinstance(decision, dict) and decision.get("projectionDigest") == projection_digest else None,
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

        surface = build_surface(root, ciwitness_dir)
        write_surface(surface, out_json, out_jsonl)
        print(
            "[observation-surface] OK "
            f"(state={surface['summary']['state']}, needsAttention={surface['summary']['needsAttention']}, "
            f"out={out_json})"
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
