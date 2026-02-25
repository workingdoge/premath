#!/usr/bin/env python3
"""Deterministic coordinator/worker loop for multi-worktree execution."""

from __future__ import annotations

import argparse
import functools
import hashlib
import json
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass, replace
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping, Sequence


REPO_ROOT = Path(__file__).resolve().parents[2]
CONTROL_PLANE_CONTRACT_PATH = REPO_ROOT / "specs/premath/draft/CONTROL-PLANE-CONTRACT.json"
MCP_ONLY_TRANSPORT_FAILURE_CLASS = "control_plane_host_action_mcp_transport_required"
LEASE_ACTION_TO_HOST_ACTION_ID = {
    "issue_lease_renew": "issue.lease_renew",
    "issue_lease_release": "issue.lease_release",
}
DEFAULT_MCP_ONLY_HOST_ACTIONS = frozenset(LEASE_ACTION_TO_HOST_ACTION_ID.values())


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run deterministic coordinator/worker loops over premath issue memory. "
            "Issue JSONL remains mutation authority; harness artifacts are projection-only."
        )
    )
    sub = parser.add_subparsers(dest="mode", required=True)

    worker = sub.add_parser("worker", help="Run one worker loop in one worktree")
    add_common_worker_args(worker)
    worker.add_argument(
        "--worker-id",
        default=os.environ.get("PREMATH_WORKER_ID", ""),
        help="Worker identity label (default: PREMATH_WORKER_ID)",
    )
    worker.add_argument(
        "--max-steps",
        type=int,
        default=1,
        help="Maximum claim/work iterations for this worker run (default: 1)",
    )
    worker.add_argument(
        "--continue-on-failure",
        action="store_true",
        help="Continue claiming after failed work/verify steps",
    )

    coordinator = sub.add_parser("coordinator", help="Run deterministic coordinator loop over N worktrees")
    add_common_worker_args(coordinator)
    coordinator.add_argument(
        "--worktree",
        action="append",
        default=[],
        help=(
            "Worker worktree path (repeatable). "
            "If omitted, PREMATH_WORKTREES may provide a comma-separated list."
        ),
    )
    coordinator.add_argument(
        "--rounds",
        type=int,
        default=1,
        help="Maximum coordinator rounds (default: 1)",
    )
    coordinator.add_argument(
        "--worker-prefix",
        default="worker",
        help="Worker id prefix used by coordinator (default: worker)",
    )
    coordinator.add_argument(
        "--max-steps-per-worker",
        type=int,
        default=1,
        help="Worker max-steps value for each coordinator dispatch (default: 1)",
    )
    coordinator.add_argument(
        "--continue-on-failure",
        action="store_true",
        help="Continue coordinator dispatch when one worker run fails",
    )

    return parser.parse_args()


def add_common_worker_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--repo-root",
        default=str(REPO_ROOT),
        help=f"Repository root (default: {REPO_ROOT})",
    )
    parser.add_argument(
        "--worktree-root",
        default=".",
        help="Working directory for work + verify commands (default: current directory)",
    )
    parser.add_argument(
        "--issues-path",
        default=".premath/issues.jsonl",
        help="Canonical issues JSONL path (default: .premath/issues.jsonl)",
    )
    parser.add_argument(
        "--session-path",
        default=".premath/harness_session.json",
        help="Harness session projection path (default: .premath/harness_session.json)",
    )
    parser.add_argument(
        "--feature-ledger-path",
        default=".premath/harness_feature_ledger.json",
        help="Harness feature ledger projection path (default: .premath/harness_feature_ledger.json)",
    )
    parser.add_argument(
        "--trajectory-path",
        default=".premath/harness_trajectory.jsonl",
        help="Harness trajectory projection path (default: .premath/harness_trajectory.jsonl)",
    )
    parser.add_argument(
        "--lease-ttl-seconds",
        type=int,
        default=3600,
        help="Lease TTL seconds passed to issue claim-next (default: 3600)",
    )
    parser.add_argument(
        "--work-cmd",
        default=os.environ.get("PREMATH_WORK_CMD", "true"),
        help="Shell command that performs bounded work (default: true)",
    )
    parser.add_argument(
        "--verify-cmd",
        default=os.environ.get("PREMATH_VERIFY_CMD", "mise run ci-check"),
        help="Shell command for deterministic verification (default: mise run ci-check)",
    )
    parser.add_argument(
        "--witness-ref-prefix",
        default=os.environ.get("PREMATH_WITNESS_REF_PREFIX", "worker://loop"),
        help="Prefix for synthetic witness refs in trajectory rows",
    )
    parser.add_argument(
        "--mutation-mode",
        default="",
        help=(
            "Worker mutation mode override. Empty means use control-plane default "
            "(workerLaneAuthority.mutationPolicy.defaultMode)."
        ),
    )
    parser.add_argument(
        "--override-reason",
        default="",
        help="Required when mutation mode is a non-default override (for example human-override).",
    )
    parser.add_argument(
        "--host-action-transport",
        choices=("mcp", "local-repl"),
        default=resolve_default_host_action_transport(),
        help=(
            "Host-action transport profile for failure-recovery actions "
            "(default: PREMATH_HOST_ACTION_TRANSPORT or mcp)."
        ),
    )


@dataclass(frozen=True)
class RunResult:
    code: int
    stdout: str
    stderr: str


@dataclass(frozen=True)
class WorkerPolicy:
    active_epoch: str
    default_mode: str
    allowed_modes: set[str]
    support_until_by_mode: dict[str, str]


@dataclass(frozen=True)
class IssueLeaseSnapshot:
    issue_id: str
    status: str
    assignee: str
    lease_id: str | None
    lease_owner: str | None
    lease_expires_at: str | None
    lease_state: str


@dataclass(frozen=True)
class StopHandoff:
    lease_state: str
    lease_action: str
    result_class: str
    summary: str
    next_step: str
    lease_ref: str


def now_rfc3339() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def resolve_path(repo_root: Path, raw: str) -> Path:
    path = Path(raw)
    if path.is_absolute():
        return path
    return (repo_root / path).resolve()


def resolve_worktree(raw: str) -> Path:
    return Path(raw).resolve()


def premath_base_cmd() -> list[str]:
    override = os.environ.get("PREMATH_BIN", "").strip()
    if override:
        return shlex.split(override)
    return ["cargo", "run", "--package", "premath-cli", "--"]


def stable_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def parse_rfc3339(value: str) -> datetime | None:
    raw = value.strip()
    if not raw:
        return None
    if raw.endswith("Z"):
        raw = f"{raw[:-1]}+00:00"
    try:
        parsed = datetime.fromisoformat(raw)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def resolve_default_host_action_transport() -> str:
    raw = os.environ.get("PREMATH_HOST_ACTION_TRANSPORT", "").strip().lower()
    if raw == "local":
        return "local-repl"
    if raw in {"mcp", "local-repl"}:
        return raw
    return "mcp"


@functools.lru_cache(maxsize=1)
def load_mcp_only_host_actions() -> frozenset[str]:
    try:
        payload = json.loads(CONTROL_PLANE_CONTRACT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return DEFAULT_MCP_ONLY_HOST_ACTIONS
    if not isinstance(payload, dict):
        return DEFAULT_MCP_ONLY_HOST_ACTIONS
    host_action_surface = payload.get("hostActionSurface")
    if not isinstance(host_action_surface, dict):
        return DEFAULT_MCP_ONLY_HOST_ACTIONS
    rows = host_action_surface.get("mcpOnlyHostActions")
    if not isinstance(rows, list):
        return DEFAULT_MCP_ONLY_HOST_ACTIONS
    actions = []
    for item in rows:
        if isinstance(item, str) and item.strip():
            actions.append(item.strip())
    if not actions:
        return DEFAULT_MCP_ONLY_HOST_ACTIONS
    return frozenset(actions)


def enforce_mcp_only_transport(
    handoff: StopHandoff, *, host_action_transport: str
) -> StopHandoff:
    if host_action_transport == "mcp":
        return handoff
    host_action_id = LEASE_ACTION_TO_HOST_ACTION_ID.get(handoff.lease_action)
    if host_action_id is None:
        return handoff
    mcp_only_actions = load_mcp_only_host_actions()
    if host_action_id not in mcp_only_actions:
        return handoff
    return StopHandoff(
        lease_state=handoff.lease_state,
        lease_action=handoff.lease_action,
        result_class=MCP_ONLY_TRANSPORT_FAILURE_CLASS,
        summary=(
            "mcp-only lease action cannot execute on local-repl transport "
            f"(action={handoff.lease_action})"
        ),
        next_step=(
            "switch to MCP transport for lease recovery "
            f"(required host action: {host_action_id})"
        ),
        lease_ref=handoff.lease_ref,
    )


def read_issue_lease_snapshot(issues_path: Path, issue_id: str) -> IssueLeaseSnapshot:
    if not issues_path.exists():
        raise RuntimeError(f"issues path does not exist for stop/handoff invariant check: {issues_path}")

    try:
        lines = issues_path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise RuntimeError(f"failed reading issues path for stop/handoff invariant check: {exc}") from exc

    latest: Mapping[str, Any] | None = None
    for raw in lines:
        line = raw.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(payload, dict):
            continue
        if str(payload.get("id", "")).strip() != issue_id:
            continue
        latest = payload

    if latest is None:
        return IssueLeaseSnapshot(
            issue_id=issue_id,
            status="",
            assignee="",
            lease_id=None,
            lease_owner=None,
            lease_expires_at=None,
            lease_state="missing_issue",
        )

    status = str(latest.get("status", "")).strip()
    assignee = str(latest.get("assignee", "")).strip()
    lease_obj = latest.get("lease")
    if status == "closed":
        if isinstance(lease_obj, dict):
            return IssueLeaseSnapshot(
                issue_id=issue_id,
                status=status,
                assignee=assignee,
                lease_id=str(lease_obj.get("lease_id", "")).strip() or None,
                lease_owner=str(lease_obj.get("owner", "")).strip() or None,
                lease_expires_at=str(lease_obj.get("expires_at", "")).strip() or None,
                lease_state="closed_with_lease",
            )
        return IssueLeaseSnapshot(
            issue_id=issue_id,
            status=status,
            assignee=assignee,
            lease_id=None,
            lease_owner=None,
            lease_expires_at=None,
            lease_state="released",
        )

    if not isinstance(lease_obj, dict):
        return IssueLeaseSnapshot(
            issue_id=issue_id,
            status=status,
            assignee=assignee,
            lease_id=None,
            lease_owner=None,
            lease_expires_at=None,
            lease_state="unleased",
        )

    lease_id = str(lease_obj.get("lease_id", "")).strip() or None
    lease_owner = str(lease_obj.get("owner", "")).strip() or None
    lease_expires_at = str(lease_obj.get("expires_at", "")).strip() or None
    expires = parse_rfc3339(lease_expires_at or "")
    if expires is None:
        lease_state = "invalid_expires_at"
    else:
        now = datetime.now(timezone.utc)
        if expires <= now:
            lease_state = "stale"
        elif status != "in_progress" or (lease_owner and assignee != lease_owner):
            lease_state = "contended"
        else:
            lease_state = "active"

    return IssueLeaseSnapshot(
        issue_id=issue_id,
        status=status,
        assignee=assignee,
        lease_id=lease_id,
        lease_owner=lease_owner,
        lease_expires_at=lease_expires_at,
        lease_state=lease_state,
    )


def build_lease_ref(
    *,
    issue_id: str,
    worker_id: str,
    lease_id: str | None,
    lease_owner: str | None,
    lease_state: str,
    lease_action: str,
    status: str,
    assignee: str,
) -> str:
    digest = stable_hash(
        {
            "issueId": issue_id,
            "workerId": worker_id,
            "leaseId": lease_id or "",
            "leaseOwner": lease_owner or "",
            "leaseState": lease_state,
            "leaseAction": lease_action,
            "status": status,
            "assignee": assignee,
        }
    )
    return f"lease://handoff/{issue_id}/{lease_state}/{digest}"


def classify_failed_stop_handoff(
    *,
    snapshot: IssueLeaseSnapshot,
    worker_id: str,
    claimed_lease_id: str | None,
    host_action_transport: str = "mcp",
) -> StopHandoff:
    if snapshot.lease_state == "active":
        lease_id = snapshot.lease_id or claimed_lease_id
        owner = snapshot.lease_owner or worker_id
        handoff = StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="issue_lease_renew",
            result_class="retry_needed_lease_active",
            summary=f"lease active for owner={owner} lease_id={lease_id or 'unknown'}",
            next_step=(
                "recover lease via transport-dispatch issue.lease_renew "
                f"(id={snapshot.issue_id}, assignee={owner}, lease_id={lease_id or 'MISSING'})"
            ),
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=lease_id,
                lease_owner=owner,
                lease_state=snapshot.lease_state,
                lease_action="issue_lease_renew",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )
        return enforce_mcp_only_transport(
            handoff, host_action_transport=host_action_transport
        )

    if snapshot.lease_state == "stale":
        return StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="issue_claim_next_reclaim",
            result_class="retry_needed_lease_stale",
            summary="lease stale; reclaim required before retry",
            next_step=(
                "lease stale; reclaim via issue claim-next (or explicit issue_lease_release reconciliation) before retry"
            ),
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=snapshot.lease_id,
                lease_owner=snapshot.lease_owner,
                lease_state=snapshot.lease_state,
                lease_action="issue_claim_next_reclaim",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )

    if snapshot.lease_state == "contended":
        handoff = StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="issue_lease_release",
            result_class="retry_needed_lease_contended",
            summary="lease contention detected; release/rebind required before retry",
            next_step=(
                "lease contended; reconcile via transport-dispatch issue.lease_release then reclaim/renew before retry"
            ),
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=snapshot.lease_id,
                lease_owner=snapshot.lease_owner,
                lease_state=snapshot.lease_state,
                lease_action="issue_lease_release",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )
        return enforce_mcp_only_transport(
            handoff, host_action_transport=host_action_transport
        )

    if snapshot.lease_state == "released" and snapshot.status == "closed":
        return StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="claim_next",
            result_class="failed_issue_closed",
            summary="issue already closed; lease released",
            next_step="issue closed during failure recovery; claim next ready issue",
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=None,
                lease_owner=None,
                lease_state=snapshot.lease_state,
                lease_action="claim_next",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )

    if snapshot.lease_state == "unleased":
        return StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="reconcile",
            result_class="failed_lease_missing",
            summary="issue is in progress without lease binding",
            next_step="lease missing; reconcile assignee/lease binding via issue_lease_release or re-claim",
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=None,
                lease_owner=None,
                lease_state=snapshot.lease_state,
                lease_action="reconcile",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )

    if snapshot.lease_state == "missing_issue":
        return StopHandoff(
            lease_state=snapshot.lease_state,
            lease_action="stop",
            result_class="failed_issue_missing",
            summary="issue missing from issue-memory projection",
            next_step="issue missing from issues.jsonl; run issue list/check before retry",
            lease_ref=build_lease_ref(
                issue_id=snapshot.issue_id,
                worker_id=worker_id,
                lease_id=None,
                lease_owner=None,
                lease_state=snapshot.lease_state,
                lease_action="stop",
                status=snapshot.status,
                assignee=snapshot.assignee,
            ),
        )

    return StopHandoff(
        lease_state=snapshot.lease_state,
        lease_action="stop",
        result_class="failed_lease_invariant",
        summary=f"lease invariant mismatch ({snapshot.lease_state})",
        next_step="lease invariant mismatch; inspect issue_lease_projection and reconcile manually",
        lease_ref=build_lease_ref(
            issue_id=snapshot.issue_id,
            worker_id=worker_id,
            lease_id=snapshot.lease_id,
            lease_owner=snapshot.lease_owner,
            lease_state=snapshot.lease_state,
            lease_action="stop",
            status=snapshot.status,
            assignee=snapshot.assignee,
        ),
    )


def execute_transport_recovery_action(
    *,
    base_cmd: Sequence[str],
    cwd: Path,
    issues_path: Path,
    snapshot: IssueLeaseSnapshot,
    worker_id: str,
    claimed_lease_id: str | None,
    lease_ttl_seconds: int,
    handoff: StopHandoff,
    host_action_transport: str,
) -> tuple[StopHandoff, list[str]]:
    if host_action_transport != "mcp":
        return handoff, []
    if handoff.lease_action not in {"issue_lease_renew", "issue_lease_release"}:
        return handoff, []

    if handoff.lease_action == "issue_lease_renew":
        lease_id = snapshot.lease_id or claimed_lease_id
        assignee = snapshot.lease_owner or worker_id
        if not lease_id:
            updated = replace(
                handoff,
                result_class=f"{handoff.result_class}_transport_input_missing",
                summary=f"{handoff.summary}; transport dispatch skipped (missing lease_id)",
                next_step=(
                    f"{handoff.next_step}; lease_id missing so issue.lease_renew was not dispatched"
                ),
            )
            return updated, []
        dispatch_action = "issue.lease_renew"
        dispatch_payload: dict[str, Any] = {
            "id": snapshot.issue_id,
            "assignee": assignee,
            "leaseId": lease_id,
            "leaseTtlSeconds": lease_ttl_seconds,
            "issuesPath": str(issues_path),
        }
    else:
        dispatch_action = "issue.lease_release"
        dispatch_payload = {
            "id": snapshot.issue_id,
            "issuesPath": str(issues_path),
        }
        if snapshot.lease_owner:
            dispatch_payload["assignee"] = snapshot.lease_owner
        if snapshot.lease_id:
            dispatch_payload["leaseId"] = snapshot.lease_id

    try:
        response = run_transport_dispatch(
            base_cmd,
            cwd,
            action=dispatch_action,
            payload=dispatch_payload,
        )
    except RuntimeError as exc:
        updated = replace(
            handoff,
            result_class=f"{handoff.result_class}_transport_error",
            summary=f"{handoff.summary}; transport dispatch error: {exc}",
            next_step=(
                f"{handoff.next_step}; transport dispatch failed to execute ({dispatch_action})"
            ),
        )
        return updated, []

    refs: list[str] = []
    transport_ref = transport_dispatch_ref(dispatch_action, response)
    if transport_ref:
        refs.append(transport_ref)

    result = str(response.get("result", "")).strip()
    if result == "accepted":
        updated = replace(
            handoff,
            result_class=f"{handoff.result_class}_transport_dispatched",
            summary=f"{handoff.summary}; transport dispatch accepted ({dispatch_action})",
            next_step=(
                f"{handoff.next_step}; recovery action dispatched via transport ({dispatch_action})"
            ),
        )
        return updated, refs

    failure_classes_raw = response.get("failureClasses")
    failure_classes: list[str]
    if isinstance(failure_classes_raw, list):
        failure_classes = [
            str(item).strip() for item in failure_classes_raw if str(item).strip()
        ]
    else:
        failure_classes = []
    failure_suffix = ",".join(failure_classes) if failure_classes else "unknown"
    updated = replace(
        handoff,
        result_class=f"{handoff.result_class}_transport_rejected",
        summary=(
            f"{handoff.summary}; transport dispatch rejected "
            f"({dispatch_action}; failures={failure_suffix})"
        ),
        next_step=(
            f"{handoff.next_step}; inspect transport dispatch rejection and reconcile manually"
        ),
    )
    return updated, refs


def assert_success_stop_handoff(
    *,
    snapshot: IssueLeaseSnapshot,
    worker_id: str,
    claimed_lease_id: str | None,
) -> StopHandoff:
    if snapshot.status != "closed":
        raise RuntimeError(
            f"stop/handoff invariant violated for {snapshot.issue_id}: expected status=closed, got {snapshot.status!r}"
        )
    if snapshot.lease_state != "released":
        raise RuntimeError(
            "stop/handoff invariant violated for "
            f"{snapshot.issue_id}: expected lease_state=released, got {snapshot.lease_state!r}"
        )
    return StopHandoff(
        lease_state=snapshot.lease_state,
        lease_action="release_closed",
        result_class="completed",
        summary="closed transition released lease",
        next_step="claim next ready issue",
        lease_ref=build_lease_ref(
            issue_id=snapshot.issue_id,
            worker_id=worker_id,
            lease_id=claimed_lease_id,
            lease_owner=worker_id,
            lease_state=snapshot.lease_state,
            lease_action="release_closed",
            status=snapshot.status,
            assignee=snapshot.assignee,
        ),
    )


def run_command(cmd: Sequence[str], cwd: Path) -> RunResult:
    proc = subprocess.run(
        list(cmd),
        cwd=str(cwd),
        text=True,
        capture_output=True,
    )
    return RunResult(code=proc.returncode, stdout=proc.stdout, stderr=proc.stderr)


def run_shell(command: str, cwd: Path) -> int:
    proc = subprocess.run(["sh", "-lc", command], cwd=str(cwd))
    return int(proc.returncode)


def run_premath_json(base_cmd: Sequence[str], args: Sequence[str], cwd: Path) -> dict[str, Any]:
    result = run_command([*base_cmd, *args, "--json"], cwd)
    if result.code != 0:
        raise RuntimeError(
            "premath command failed "
            f"(cwd={cwd}, code={result.code}): {' '.join([*base_cmd, *args])}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            "premath command returned non-JSON output "
            f"(cwd={cwd}): {' '.join([*base_cmd, *args])}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        ) from exc
    if not isinstance(payload, dict):
        raise RuntimeError(
            f"premath command returned non-object payload (cwd={cwd}): {' '.join([*base_cmd, *args])}"
        )
    return payload


def run_transport_dispatch(
    base_cmd: Sequence[str],
    cwd: Path,
    *,
    action: str,
    payload: Mapping[str, Any],
) -> dict[str, Any]:
    payload_arg = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return run_premath_json(
        base_cmd,
        [
            "transport-dispatch",
            "--action",
            action,
            "--payload",
            payload_arg,
        ],
        cwd,
    )


def transport_dispatch_ref(action: str, response: Mapping[str, Any]) -> str | None:
    semantic_digest = str(response.get("semanticDigest", "")).strip()
    if not semantic_digest:
        return None
    safe_action = action.strip().replace("/", "_")
    return f"transport://dispatch/{safe_action}/{semantic_digest}"


def load_worker_policy(repo_root: Path) -> WorkerPolicy:
    contract_path = repo_root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
    payload = json.loads(contract_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise RuntimeError(f"control-plane contract must be object: {contract_path}")

    active_epoch = str(payload.get("schemaLifecycle", {}).get("activeEpoch", "")).strip()
    worker_lane = payload.get("workerLaneAuthority", {})
    if not isinstance(worker_lane, dict):
        raise RuntimeError("workerLaneAuthority must be an object")
    policy = worker_lane.get("mutationPolicy", {})
    if not isinstance(policy, dict):
        raise RuntimeError("workerLaneAuthority.mutationPolicy must be an object")
    default_mode = str(policy.get("defaultMode", "")).strip()
    allowed_modes_raw = policy.get("allowedModes", [])
    if not isinstance(allowed_modes_raw, list):
        raise RuntimeError("workerLaneAuthority.mutationPolicy.allowedModes must be a list")
    allowed_modes = {str(item).strip() for item in allowed_modes_raw if str(item).strip()}

    support_until_by_mode: dict[str, str] = {}
    overrides = policy.get("compatibilityOverrides", [])
    if isinstance(overrides, list):
        for row in overrides:
            if not isinstance(row, dict):
                continue
            mode = str(row.get("mode", "")).strip()
            support_until = str(row.get("supportUntilEpoch", "")).strip()
            if mode and support_until:
                support_until_by_mode[mode] = support_until

    if not default_mode:
        raise RuntimeError("workerLaneAuthority.mutationPolicy.defaultMode is required")
    if default_mode not in allowed_modes:
        raise RuntimeError("workerLaneAuthority.mutationPolicy.defaultMode must be listed in allowedModes")

    return WorkerPolicy(
        active_epoch=active_epoch,
        default_mode=default_mode,
        allowed_modes=allowed_modes,
        support_until_by_mode=support_until_by_mode,
    )


def parse_epoch(value: str) -> tuple[int, int]:
    raw = value.strip()
    chunks = raw.split("-", 1)
    if len(chunks) != 2:
        raise RuntimeError(f"epoch must be YYYY-MM: {value!r}")
    year = int(chunks[0])
    month = int(chunks[1])
    if month < 1 or month > 12:
        raise RuntimeError(f"epoch month must be 1..12: {value!r}")
    return year, month


def epoch_leq(left: str, right: str) -> bool:
    return parse_epoch(left) <= parse_epoch(right)


def resolve_mutation_mode(args: argparse.Namespace, policy: WorkerPolicy) -> tuple[str, str, str]:
    mode = args.mutation_mode.strip() or policy.default_mode
    reason = args.override_reason.strip()
    if mode not in policy.allowed_modes:
        raise RuntimeError(
            f"mutation mode `{mode}` is not allowed by control-plane contract: {sorted(policy.allowed_modes)}"
        )

    if mode == policy.default_mode:
        raise RuntimeError(
            "default mutation mode is `instruction-linked`; this loop uses direct CLI mutation paths. "
            "Use explicit `--mutation-mode human-override --override-reason <reason>` "
            "or execute via MCP instruction-linked routes."
        )

    if not reason:
        raise RuntimeError("override mutation mode requires --override-reason")

    support_until = policy.support_until_by_mode.get(mode, "")
    if support_until and policy.active_epoch and not epoch_leq(policy.active_epoch, support_until):
        raise RuntimeError(
            f"override mode `{mode}` expired at epoch {support_until} (active epoch: {policy.active_epoch})"
        )
    return mode, reason, support_until


def policy_audit_ref(
    *,
    mode: str,
    reason: str,
    active_epoch: str,
    support_until: str,
    worker_id: str,
    issue_id: str,
    step_index: int,
) -> str:
    digest = stable_hash(
        {
            "mode": mode,
            "reason": reason,
            "activeEpoch": active_epoch,
            "supportUntil": support_until,
            "workerId": worker_id,
            "issueId": issue_id,
            "stepIndex": step_index,
        }
    )
    return f"policy://worker-lane/{mode}/{digest}"


def build_site_lineage_refs(
    *,
    repo_root: Path,
    issues_path: Path,
    worktree: Path,
    worker_id: str,
    issue_id: str,
    mutation_mode: str,
    active_epoch: str,
    support_until: str,
) -> list[str]:
    ctx_digest = stable_hash(
        {
            "kind": "ctx",
            "issueId": issue_id,
            "repoRoot": str(repo_root),
            "issuesPath": str(issues_path),
        }
    )
    ctx_ref = f"ctx://issue/{issue_id}/{ctx_digest}"

    cover_digest = stable_hash(
        {
            "kind": "cover",
            "ctxRef": ctx_ref,
            "workerPool": "harness.multithread.v1",
        }
    )
    cover_ref = f"cover://worker-loop/{issue_id}/{cover_digest}"

    refinement_digest = stable_hash(
        {
            "kind": "refinement",
            "coverRef": cover_ref,
            "workerId": worker_id,
            "worktree": str(worktree),
            "mutationMode": mutation_mode,
            "activeEpoch": active_epoch,
            "supportUntil": support_until,
        }
    )
    refinement_ref = f"refinement://worker-loop/{issue_id}/{worker_id}/{refinement_digest}"
    return sorted({ctx_ref, cover_ref, refinement_ref})


def issue_ready_count(base_cmd: Sequence[str], cwd: Path, issues_path: Path) -> int:
    payload = run_premath_json(
        base_cmd,
        ["issue", "ready", "--issues", str(issues_path)],
        cwd,
    )
    raw = payload.get("count", 0)
    if isinstance(raw, int):
        return raw
    raise RuntimeError(f"issue.ready count is not an integer: {raw!r}")


def write_session_projection(
    base_cmd: Sequence[str],
    cwd: Path,
    *,
    session_path: Path,
    issues_path: Path,
    state: str,
    issue_id: str,
    summary: str,
    next_step: str,
    witness_refs: Sequence[str] = (),
    lineage_refs: Sequence[str] = (),
) -> None:
    args = [
        "harness-session",
        "write",
        "--path",
        str(session_path),
        "--state",
        state,
        "--issue-id",
        issue_id,
        "--summary",
        summary,
        "--next-step",
        next_step,
        "--issues",
        str(issues_path),
    ]
    for witness_ref in witness_refs:
        if witness_ref:
            args.extend(["--witness-ref", witness_ref])
    for lineage_ref in lineage_refs:
        if lineage_ref:
            args.extend(["--lineage-ref", lineage_ref])
    run_premath_json(base_cmd, args, cwd)


def write_feature_projection(
    base_cmd: Sequence[str],
    cwd: Path,
    *,
    feature_ledger_path: Path,
    session_path: Path,
    issue_id: str,
    status: str,
    summary: str,
    verification_refs: Sequence[str] = (),
) -> None:
    args = [
        "harness-feature",
        "write",
        "--path",
        str(feature_ledger_path),
        "--feature-id",
        issue_id,
        "--status",
        status,
        "--issue-id",
        issue_id,
        "--summary",
        summary,
        "--session-ref",
        str(session_path),
    ]
    for verification_ref in verification_refs:
        if verification_ref:
            args.extend(["--verification-ref", verification_ref])
    run_premath_json(base_cmd, args, cwd)


def append_trajectory_projection(
    base_cmd: Sequence[str],
    cwd: Path,
    *,
    trajectory_path: Path,
    step_id: str,
    issue_id: str,
    result_class: str,
    witness_refs: Sequence[str],
    lineage_refs: Sequence[str],
) -> None:
    args = [
        "harness-trajectory",
        "append",
        "--path",
        str(trajectory_path),
        "--step-id",
        step_id,
        "--issue-id",
        issue_id,
        "--action",
        "worker.loop",
        "--result-class",
        result_class,
    ]
    for ref in witness_refs:
        args.extend(["--witness-ref", ref])
    for ref in lineage_refs:
        if ref:
            args.extend(["--lineage-ref", ref])
    args.extend(["--finished-at", now_rfc3339()])
    run_premath_json(base_cmd, args, cwd)


def close_issue(base_cmd: Sequence[str], cwd: Path, issues_path: Path, issue_id: str) -> None:
    run_premath_json(
        base_cmd,
        [
            "issue",
            "update",
            issue_id,
            "--status",
            "closed",
            "--issues",
            str(issues_path),
        ],
        cwd,
    )


def run_worker(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    worktree = resolve_worktree(args.worktree_root)
    issues_path = resolve_path(repo_root, args.issues_path)
    session_path = resolve_path(repo_root, args.session_path)
    feature_ledger_path = resolve_path(repo_root, args.feature_ledger_path)
    trajectory_path = resolve_path(repo_root, args.trajectory_path)
    worker_id = args.worker_id.strip()
    if not worker_id:
        print("error: --worker-id is required (or set PREMATH_WORKER_ID)", file=sys.stderr)
        return 2
    if args.max_steps < 1:
        print("error: --max-steps must be >= 1", file=sys.stderr)
        return 2

    base_cmd = premath_base_cmd()
    policy = load_worker_policy(repo_root)
    try:
        mutation_mode, override_reason, support_until = resolve_mutation_mode(args, policy)
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    policy_summary = (
        f"mode={mutation_mode};reason={override_reason};"
        f"active_epoch={policy.active_epoch};support_until={support_until or 'none'}"
    )
    print(f"[worker:{worker_id}] override policy {policy_summary}")

    for step_index in range(1, args.max_steps + 1):
        claim = run_premath_json(
            base_cmd,
            [
                "issue",
                "claim-next",
                "--assignee",
                worker_id,
                "--lease-ttl-seconds",
                str(args.lease_ttl_seconds),
                "--issues",
                str(issues_path),
            ],
            worktree,
        )
        if not bool(claim.get("claimed", False)):
            print(f"[worker:{worker_id}] no ready issue to claim (step={step_index})")
            return 0

        issue_obj = claim.get("issue")
        if not isinstance(issue_obj, dict):
            raise RuntimeError("issue.claim-next payload missing issue object when claimed=true")
        issue_id = str(issue_obj.get("id", "")).strip()
        if not issue_id:
            raise RuntimeError("issue.claim-next payload returned empty issue id")
        lease_obj = issue_obj.get("lease")
        claimed_lease_id = None
        if isinstance(lease_obj, dict):
            raw_lease_id = lease_obj.get("leaseId")
            if isinstance(raw_lease_id, str) and raw_lease_id.strip():
                claimed_lease_id = raw_lease_id.strip()

        summary = f"worker={worker_id} claimed={issue_id} worktree={worktree} {policy_summary}"
        lineage_refs = build_site_lineage_refs(
            repo_root=repo_root,
            issues_path=issues_path,
            worktree=worktree,
            worker_id=worker_id,
            issue_id=issue_id,
            mutation_mode=mutation_mode,
            active_epoch=policy.active_epoch,
            support_until=support_until,
        )
        write_session_projection(
            base_cmd,
            worktree,
            session_path=session_path,
            issues_path=issues_path,
            state="active",
            issue_id=issue_id,
            summary=summary,
            next_step="work -> verify -> close_or_recover",
            witness_refs=[],
            lineage_refs=lineage_refs,
        )
        write_feature_projection(
            base_cmd,
            worktree,
            feature_ledger_path=feature_ledger_path,
            session_path=session_path,
            issue_id=issue_id,
            status="in_progress",
            summary=summary,
            verification_refs=[],
        )

        work_rc = run_shell(args.work_cmd, worktree)
        verify_rc = run_shell(args.verify_cmd, worktree)
        success = work_rc == 0 and verify_rc == 0

        step_id = f"{worker_id}.{issue_id}.{step_index}"
        execution_result = "completed" if success else "failed"
        witness_ref = f"{args.witness_ref_prefix.rstrip('/')}/{worker_id}/{issue_id}/{execution_result}"
        policy_ref = policy_audit_ref(
            mode=mutation_mode,
            reason=override_reason,
            active_epoch=policy.active_epoch,
            support_until=support_until,
            worker_id=worker_id,
            issue_id=issue_id,
            step_index=step_index,
        )

        if success:
            close_issue(base_cmd, worktree, issues_path, issue_id)
            snapshot = read_issue_lease_snapshot(issues_path, issue_id)
            handoff = assert_success_stop_handoff(
                snapshot=snapshot,
                worker_id=worker_id,
                claimed_lease_id=claimed_lease_id,
            )
            append_trajectory_projection(
                base_cmd,
                worktree,
                trajectory_path=trajectory_path,
                step_id=step_id,
                issue_id=issue_id,
                result_class=handoff.result_class,
                witness_refs=[witness_ref, policy_ref, handoff.lease_ref],
                lineage_refs=lineage_refs,
            )
            write_feature_projection(
                base_cmd,
                worktree,
                feature_ledger_path=feature_ledger_path,
                session_path=session_path,
                issue_id=issue_id,
                status="completed",
                summary=(
                    f"worker={worker_id} verified and closed issue={issue_id}; "
                    f"lease_state={handoff.lease_state}; action={handoff.lease_action}"
                ),
                verification_refs=[witness_ref, handoff.lease_ref],
            )
            write_session_projection(
                base_cmd,
                worktree,
                session_path=session_path,
                issues_path=issues_path,
                state="stopped",
                issue_id=issue_id,
                summary=(
                    f"worker={worker_id} closed issue={issue_id}; "
                    f"lease_state={handoff.lease_state}; action={handoff.lease_action}"
                ),
                next_step=handoff.next_step,
                witness_refs=[policy_ref, handoff.lease_ref],
                lineage_refs=lineage_refs,
            )
            print(f"[worker:{worker_id}] closed issue {issue_id}")
            continue

        snapshot = read_issue_lease_snapshot(issues_path, issue_id)
        handoff = classify_failed_stop_handoff(
            snapshot=snapshot,
            worker_id=worker_id,
            claimed_lease_id=claimed_lease_id,
            host_action_transport=args.host_action_transport,
        )
        handoff, transport_refs = execute_transport_recovery_action(
            base_cmd=base_cmd,
            cwd=worktree,
            issues_path=issues_path,
            snapshot=snapshot,
            worker_id=worker_id,
            claimed_lease_id=claimed_lease_id,
            lease_ttl_seconds=args.lease_ttl_seconds,
            handoff=handoff,
            host_action_transport=args.host_action_transport,
        )
        failure_witness_refs = [witness_ref, policy_ref, handoff.lease_ref, *transport_refs]
        append_trajectory_projection(
            base_cmd,
            worktree,
            trajectory_path=trajectory_path,
            step_id=step_id,
            issue_id=issue_id,
            result_class=handoff.result_class,
            witness_refs=failure_witness_refs,
            lineage_refs=lineage_refs,
        )
        write_feature_projection(
            base_cmd,
            worktree,
            feature_ledger_path=feature_ledger_path,
            session_path=session_path,
            issue_id=issue_id,
            status="blocked",
            summary=(
                f"worker={worker_id} failed issue={issue_id}; "
                f"lease_state={handoff.lease_state}; action={handoff.lease_action}; "
                f"{handoff.summary}"
            ),
            verification_refs=[witness_ref, handoff.lease_ref, *transport_refs],
        )
        write_session_projection(
            base_cmd,
            worktree,
            session_path=session_path,
            issues_path=issues_path,
            state="stopped",
            issue_id=issue_id,
            summary=(
                f"worker={worker_id} failed issue={issue_id}; "
                f"lease_state={handoff.lease_state}; action={handoff.lease_action}"
            ),
            next_step=handoff.next_step,
            witness_refs=[policy_ref, handoff.lease_ref, *transport_refs],
            lineage_refs=lineage_refs,
        )
        print(
            f"[worker:{worker_id}] work/verify failed for {issue_id} "
            f"(work_rc={work_rc}, verify_rc={verify_rc}, "
            f"lease_state={handoff.lease_state}, action={handoff.lease_action})"
        )
        if not args.continue_on_failure:
            return 1

    return 0


def resolve_worktrees(raw_rows: Sequence[str]) -> list[Path]:
    rows: list[str] = [item for item in raw_rows if item.strip()]
    if not rows:
        env_rows = os.environ.get("PREMATH_WORKTREES", "").strip()
        if env_rows:
            rows = [item.strip() for item in env_rows.split(",") if item.strip()]
    return sorted({resolve_worktree(row) for row in rows}, key=lambda item: str(item))


def run_coordinator(args: argparse.Namespace) -> int:
    repo_root = Path(args.repo_root).resolve()
    issues_path = resolve_path(repo_root, args.issues_path)
    worktrees = resolve_worktrees(args.worktree)
    if not worktrees:
        print("error: provide at least one --worktree (or PREMATH_WORKTREES)", file=sys.stderr)
        return 2
    if args.rounds < 1:
        print("error: --rounds must be >= 1", file=sys.stderr)
        return 2
    if args.max_steps_per_worker < 1:
        print("error: --max-steps-per-worker must be >= 1", file=sys.stderr)
        return 2

    base_cmd = premath_base_cmd()
    script_path = Path(__file__).resolve()

    for round_index in range(1, args.rounds + 1):
        ready = issue_ready_count(base_cmd, repo_root, issues_path)
        print(f"[coordinator] round={round_index} ready={ready}")
        if ready == 0:
            return 0

        for worker_index, worktree in enumerate(worktrees, start=1):
            ready = issue_ready_count(base_cmd, repo_root, issues_path)
            if ready == 0:
                return 0

            worker_id = f"{args.worker_prefix}.{worker_index}"
            cmd = [
                sys.executable,
                str(script_path),
                "worker",
                "--repo-root",
                str(repo_root),
                "--worktree-root",
                str(worktree),
                "--issues-path",
                str(issues_path),
                "--session-path",
                args.session_path,
                "--feature-ledger-path",
                args.feature_ledger_path,
                "--trajectory-path",
                args.trajectory_path,
                "--worker-id",
                worker_id,
                "--max-steps",
                str(args.max_steps_per_worker),
                "--lease-ttl-seconds",
                str(args.lease_ttl_seconds),
                "--work-cmd",
                args.work_cmd,
                "--verify-cmd",
                args.verify_cmd,
                "--witness-ref-prefix",
                args.witness_ref_prefix,
                "--host-action-transport",
                args.host_action_transport,
            ]
            if args.continue_on_failure:
                cmd.append("--continue-on-failure")
            if args.mutation_mode:
                cmd.extend(["--mutation-mode", args.mutation_mode])
            if args.override_reason:
                cmd.extend(["--override-reason", args.override_reason])

            result = run_command(cmd, repo_root)
            if result.stdout.strip():
                print(result.stdout.strip())
            if result.stderr.strip():
                print(result.stderr.strip(), file=sys.stderr)
            if result.code != 0 and not args.continue_on_failure:
                return result.code

    return 0


def main() -> int:
    args = parse_args()
    if args.mode == "worker":
        return run_worker(args)
    if args.mode == "coordinator":
        return run_coordinator(args)
    print(f"error: unsupported mode: {args.mode}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
