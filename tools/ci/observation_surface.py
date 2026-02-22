#!/usr/bin/env python3
"""Build/query a deterministic CI observation surface."""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
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
BLOCKING_DEP_TYPES = {"blocks", "parent-child", "conditional-blocks", "waits-for"}


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


def _parse_rfc3339(value: Any) -> Optional[datetime]:
    text = _string(value)
    if text is None:
        return None
    normalized = text[:-1] + "+00:00" if text.endswith("Z") else text
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def _rfc3339(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _canonical_dep_type(value: Any) -> Optional[str]:
    text = _string(value)
    if text is None:
        return None
    return text.lower().replace("_", "-")


def _is_blocking_dep(dep: Dict[str, Any]) -> bool:
    dep_type = _canonical_dep_type(dep.get("type"))
    if dep_type is None:
        dep_type = _canonical_dep_type(dep.get("dep_type"))
    return dep_type in BLOCKING_DEP_TYPES


def _issue_depends_on_id(dep: Dict[str, Any]) -> Optional[str]:
    value = _string(dep.get("depends_on_id"))
    if value is not None:
        return value
    return _string(dep.get("dependsOnId"))


def _lease_expires_at(lease: Dict[str, Any]) -> Optional[datetime]:
    expires = _parse_rfc3339(lease.get("expires_at"))
    if expires is not None:
        return expires
    return _parse_rfc3339(lease.get("expiresAt"))


def _load_issue_rows(issues_path: Path) -> List[Dict[str, Any]]:
    if not issues_path.exists():
        return []
    if not issues_path.is_file():
        raise ValueError(f"issues path is not a file: {issues_path}")

    by_id: Dict[str, Dict[str, Any]] = {}
    with issues_path.open("r", encoding="utf-8") as f:
        for line_no, raw_line in enumerate(f, start=1):
            line = raw_line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"invalid jsonl at {issues_path}:{line_no}: {exc}") from exc
            if not isinstance(data, dict):
                raise ValueError(f"jsonl row must be object at {issues_path}:{line_no}")
            issue_id = _string(data.get("id"))
            if issue_id is None:
                raise ValueError(f"missing issue id at {issues_path}:{line_no}")
            by_id[issue_id] = data
    return [by_id[key] for key in sorted(by_id)]


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
    proposal_ingest = payload.get("proposalIngest")
    if not isinstance(proposal_ingest, dict):
        proposal_ingest = None
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
        "runStartedAt": _string(payload.get("runStartedAt")),
        "runFinishedAt": _string(payload.get("runFinishedAt")),
        "proposalIngest": proposal_ingest,
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


def _coherence_policy_drift(
    delta: Optional[Dict[str, Any]],
    required: Optional[Dict[str, Any]],
    decision: Optional[Dict[str, Any]],
    instructions: List[Dict[str, Any]],
) -> Dict[str, Any]:
    projection_policies = sorted(
        {
            policy
            for policy in (
                _string(delta.get("projectionPolicy")) if delta else None,
                _string(required.get("projectionPolicy")) if required else None,
            )
            if policy is not None
        }
    )
    projection_digests = sorted(
        {
            digest
            for digest in (
                _string(delta.get("projectionDigest")) if delta else None,
                _string(required.get("projectionDigest")) if required else None,
                _string(decision.get("projectionDigest")) if decision else None,
            )
            if digest is not None
        }
    )
    instruction_policy_digests = sorted(
        {
            digest
            for digest in (_string(row.get("policyDigest")) for row in instructions)
            if digest is not None
        }
    )
    missing_instruction_policy_ids = sorted(
        row["instructionId"]
        for row in instructions
        if _string(row.get("policyDigest")) is None
    )

    drift_classes: List[str] = []
    if len(projection_policies) > 1:
        drift_classes.append("projection_policy_drift")
    if len(projection_digests) > 1:
        drift_classes.append("projection_digest_drift")
    if len(instruction_policy_digests) > 1:
        drift_classes.append("instruction_policy_drift")

    return {
        "projectionPolicies": projection_policies,
        "projectionDigests": projection_digests,
        "instructionPolicyDigests": instruction_policy_digests,
        "missingInstructionPolicyIds": missing_instruction_policy_ids,
        "driftClasses": drift_classes,
        "driftDetected": bool(drift_classes),
    }


def _coherence_instruction_typing(instructions: List[Dict[str, Any]]) -> Dict[str, Any]:
    unknown_instruction_ids: List[str] = []
    unknown_rejected_ids: List[str] = []
    typed_instruction_ids: List[str] = []

    for row in instructions:
        instruction_id = _string(row.get("instructionId")) or "(unknown)"
        classification = row.get("instructionClassification")
        state = classification.get("state") if isinstance(classification, dict) else None
        verdict = _string(row.get("verdictClass"))

        if state == "unknown":
            unknown_instruction_ids.append(instruction_id)
            if verdict == "rejected":
                unknown_rejected_ids.append(instruction_id)
        elif state == "typed":
            typed_instruction_ids.append(instruction_id)

    instruction_count = len(instructions)
    unknown_count = len(unknown_instruction_ids)
    unknown_rate = (unknown_count / instruction_count) if instruction_count else 0.0
    unknown_rate_percent = unknown_rate * 100.0

    return {
        "instructionCount": instruction_count,
        "typedCount": len(typed_instruction_ids),
        "unknownCount": unknown_count,
        "unknownRejectedCount": len(unknown_rejected_ids),
        "unknownRate": round(unknown_rate, 6),
        "unknownRatePercent": round(unknown_rate_percent, 2),
        "unknownInstructionIds": sorted(set(unknown_instruction_ids)),
    }


def _coherence_proposal_reject_classes(instructions: List[Dict[str, Any]]) -> Dict[str, Any]:
    counts: Dict[str, int] = {}
    instruction_ids: List[str] = []

    for row in instructions:
        instruction_id = _string(row.get("instructionId")) or "(unknown)"
        failures = row.get("failureClasses")
        if not isinstance(failures, list):
            continue

        proposal_failures = sorted(
            {
                item
                for item in failures
                if isinstance(item, str) and item.startswith("proposal_")
            }
        )
        if not proposal_failures:
            continue

        instruction_ids.append(instruction_id)
        for failure in proposal_failures:
            counts[failure] = counts.get(failure, 0) + 1

    sorted_counts = {key: counts[key] for key in sorted(counts)}
    ranked = sorted(sorted_counts.items(), key=lambda item: (-item[1], item[0]))

    return {
        "totalRejectCount": sum(sorted_counts.values()),
        "classCounts": sorted_counts,
        "topClasses": [name for name, _ in ranked[:5]],
        "instructionIds": sorted(set(instruction_ids)),
    }


def _coherence_issue_partition(issue_rows: List[Dict[str, Any]]) -> Dict[str, Any]:
    by_id: Dict[str, Dict[str, Any]] = {}
    for row in issue_rows:
        issue_id = _string(row.get("id"))
        if issue_id is not None:
            by_id[issue_id] = row

    open_ids = sorted(
        issue_id
        for issue_id, row in by_id.items()
        if _string(row.get("status")) == "open"
    )

    ready_ids: List[str] = []
    blocked_ids: List[str] = []
    for issue_id in open_ids:
        row = by_id[issue_id]
        dependencies = row.get("dependencies")
        blocked = False
        if isinstance(dependencies, list):
            for dep in dependencies:
                if not isinstance(dep, dict):
                    continue
                if not _is_blocking_dep(dep):
                    continue
                depends_on_id = _issue_depends_on_id(dep)
                if depends_on_id is None:
                    blocked = True
                    break
                blocker = by_id.get(depends_on_id)
                blocker_status = _string(blocker.get("status")) if blocker else None
                if blocker_status == "closed":
                    continue
                blocked = True
                break
        if blocked:
            blocked_ids.append(issue_id)
        else:
            ready_ids.append(issue_id)

    overlap_ids = sorted(set(ready_ids).intersection(blocked_ids))
    open_partition_gap_ids = sorted(set(open_ids).difference(set(ready_ids).union(blocked_ids)))

    return {
        "openIssueCount": len(open_ids),
        "readyCount": len(ready_ids),
        "blockedCount": len(blocked_ids),
        "readyIssueIds": ready_ids,
        "blockedIssueIds": blocked_ids,
        "overlapIssueIds": overlap_ids,
        "openPartitionGapIssueIds": open_partition_gap_ids,
        "isCoherent": not overlap_ids and not open_partition_gap_ids,
    }


def _derive_reference_time(
    instructions: List[Dict[str, Any]],
    issue_rows: List[Dict[str, Any]],
) -> Optional[datetime]:
    candidates: List[datetime] = []

    for row in instructions:
        finished_at = _parse_rfc3339(row.get("runFinishedAt"))
        if finished_at is not None:
            candidates.append(finished_at)
            continue
        started_at = _parse_rfc3339(row.get("runStartedAt"))
        if started_at is not None:
            candidates.append(started_at)

    for row in issue_rows:
        updated_at = _parse_rfc3339(row.get("updated_at"))
        if updated_at is not None:
            candidates.append(updated_at)
            continue
        updated_at = _parse_rfc3339(row.get("updatedAt"))
        if updated_at is not None:
            candidates.append(updated_at)

    if not candidates:
        return None
    return max(candidates)


def _coherence_lease_health(
    issue_rows: List[Dict[str, Any]],
    reference_time: Optional[datetime],
) -> Dict[str, Any]:
    stale_issue_ids: List[str] = []
    contended_issue_ids: List[str] = []
    unknown_evaluation_issue_ids: List[str] = []
    active_lease_count = 0
    lease_issue_count = 0

    for row in issue_rows:
        issue_id = _string(row.get("id")) or "(unknown)"
        lease = row.get("lease")
        if not isinstance(lease, dict):
            continue
        lease_issue_count += 1
        if reference_time is None:
            unknown_evaluation_issue_ids.append(issue_id)
            continue

        expires_at = _lease_expires_at(lease)
        if expires_at is None:
            unknown_evaluation_issue_ids.append(issue_id)
            continue

        if expires_at <= reference_time:
            stale_issue_ids.append(issue_id)
            continue

        active_lease_count += 1
        owner = _string(lease.get("owner")) or ""
        status = _string(row.get("status")) or ""
        assignee = _string(row.get("assignee")) or ""
        if status != "in_progress" or assignee != owner:
            contended_issue_ids.append(issue_id)

    stale_issue_ids = sorted(set(stale_issue_ids))
    contended_issue_ids = sorted(set(contended_issue_ids))
    unknown_evaluation_issue_ids = sorted(set(unknown_evaluation_issue_ids))

    return {
        "referenceTime": _rfc3339(reference_time) if reference_time is not None else None,
        "leaseIssueCount": lease_issue_count,
        "activeLeaseCount": active_lease_count,
        "staleCount": len(stale_issue_ids),
        "staleIssueIds": stale_issue_ids,
        "contendedCount": len(contended_issue_ids),
        "contendedIssueIds": contended_issue_ids,
        "unknownEvaluationCount": len(unknown_evaluation_issue_ids),
        "unknownEvaluationIssueIds": unknown_evaluation_issue_ids,
    }


def _build_coherence_summary(
    delta: Optional[Dict[str, Any]],
    required: Optional[Dict[str, Any]],
    decision: Optional[Dict[str, Any]],
    instructions: List[Dict[str, Any]],
    issue_rows: List[Dict[str, Any]],
) -> Dict[str, Any]:
    policy_drift = _coherence_policy_drift(delta, required, decision, instructions)
    instruction_typing = _coherence_instruction_typing(instructions)
    proposal_reject_classes = _coherence_proposal_reject_classes(instructions)
    issue_partition = _coherence_issue_partition(issue_rows)
    reference_time = _derive_reference_time(instructions, issue_rows)
    lease_health = _coherence_lease_health(issue_rows, reference_time)

    attention_reasons: List[str] = []
    if policy_drift["driftDetected"]:
        attention_reasons.append("policy_drift")
    if instruction_typing["unknownCount"] > 0:
        attention_reasons.append("instruction_unknown_classification")
    if proposal_reject_classes["totalRejectCount"] > 0:
        attention_reasons.append("proposal_reject_classes_present")
    if not issue_partition["isCoherent"]:
        attention_reasons.append("issue_partition_incoherent")
    if lease_health["staleCount"] > 0:
        attention_reasons.append("stale_claims")
    if lease_health["contendedCount"] > 0:
        attention_reasons.append("contended_claims")

    return {
        "policyDrift": policy_drift,
        "instructionTyping": instruction_typing,
        "proposalRejectClasses": proposal_reject_classes,
        "issuePartition": issue_partition,
        "leaseHealth": lease_health,
        "needsAttention": bool(attention_reasons),
        "attentionReasons": attention_reasons,
    }


def _relative(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def build_surface(
    repo_root: Path,
    ciwitness_dir: Path,
    issues_path: Optional[Path] = None,
) -> Dict[str, Any]:
    required_path = ciwitness_dir / "latest-required.json"
    delta_path = ciwitness_dir / "latest-delta.json"
    decision_path = ciwitness_dir / "latest-decision.json"
    resolved_issues_path = (
        issues_path
        if issues_path is not None
        else (repo_root / ".premath" / "issues.jsonl").resolve()
    )

    required_payload = _load_json(required_path)
    delta_payload = _load_json(delta_path)
    decision_payload = _load_json(decision_path)
    issue_rows = _load_issue_rows(resolved_issues_path)

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
            if payload.get("witnessKind") != INSTRUCTION_WITNESS_KIND:
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
    coherence = _build_coherence_summary(delta, required, decision, instructions, issue_rows)
    needs_attention = bool(state["needsAttention"]) or bool(coherence["needsAttention"])
    top_failure_class = state["topFailureClass"]
    if coherence["attentionReasons"] and state["state"] in {"accepted", "running", "empty"}:
        top_failure_class = coherence["attentionReasons"][0]
    elif top_failure_class is None and coherence["attentionReasons"]:
        top_failure_class = coherence["attentionReasons"][0]
    summary = {
        "state": state["state"],
        "needsAttention": needs_attention,
        "topFailureClass": top_failure_class,
        "latestProjectionDigest": latest_projection,
        "latestInstructionId": latest_instruction_id,
        "requiredCheckCount": len(required["requiredChecks"]) if required else 0,
        "executedCheckCount": len(required["executedChecks"]) if required else 0,
        "changedPathCount": delta["changedPathCount"] if delta else 0,
        "coherence": coherence,
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
