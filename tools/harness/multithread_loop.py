#!/usr/bin/env python3
"""Deterministic coordinator/worker loop for multi-worktree execution."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shlex
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Sequence


REPO_ROOT = Path(__file__).resolve().parents[2]


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
    witness_ref: str | None,
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
    if witness_ref:
        args.extend(["--witness-ref", witness_ref])
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
    verification_ref: str | None,
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

        summary = f"worker={worker_id} claimed={issue_id} worktree={worktree} {policy_summary}"
        write_session_projection(
            base_cmd,
            worktree,
            session_path=session_path,
            issues_path=issues_path,
            state="active",
            issue_id=issue_id,
            summary=summary,
            next_step="work -> verify -> close_or_recover",
            witness_ref=None,
        )
        write_feature_projection(
            base_cmd,
            worktree,
            feature_ledger_path=feature_ledger_path,
            session_path=session_path,
            issue_id=issue_id,
            status="in_progress",
            summary=summary,
            verification_ref=None,
        )

        work_rc = run_shell(args.work_cmd, worktree)
        verify_rc = run_shell(args.verify_cmd, worktree)
        success = work_rc == 0 and verify_rc == 0

        step_id = f"{worker_id}.{issue_id}.{step_index}"
        result_class = "completed" if success else "failed"
        witness_ref = f"{args.witness_ref_prefix.rstrip('/')}/{worker_id}/{issue_id}/{result_class}"
        policy_ref = policy_audit_ref(
            mode=mutation_mode,
            reason=override_reason,
            active_epoch=policy.active_epoch,
            support_until=support_until,
            worker_id=worker_id,
            issue_id=issue_id,
            step_index=step_index,
        )
        append_trajectory_projection(
            base_cmd,
            worktree,
            trajectory_path=trajectory_path,
            step_id=step_id,
            issue_id=issue_id,
            result_class=result_class,
            witness_refs=[witness_ref, policy_ref],
        )

        if success:
            close_issue(base_cmd, worktree, issues_path, issue_id)
            write_feature_projection(
                base_cmd,
                worktree,
                feature_ledger_path=feature_ledger_path,
                session_path=session_path,
                issue_id=issue_id,
                status="completed",
                summary=f"worker={worker_id} verified and closed issue={issue_id}",
                verification_ref=witness_ref,
            )
            write_session_projection(
                base_cmd,
                worktree,
                session_path=session_path,
                issues_path=issues_path,
                state="stopped",
                issue_id=issue_id,
                summary=f"worker={worker_id} closed issue={issue_id}",
                next_step="claim next ready issue",
                witness_ref=policy_ref,
            )
            print(f"[worker:{worker_id}] closed issue {issue_id}")
            continue

        write_feature_projection(
            base_cmd,
            worktree,
            feature_ledger_path=feature_ledger_path,
            session_path=session_path,
            issue_id=issue_id,
            status="blocked",
            summary=(
                f"worker={worker_id} failed issue={issue_id}; "
                "recover via issue_lease_renew or issue_lease_release (MCP surface)"
            ),
            verification_ref=witness_ref,
        )
        write_session_projection(
            base_cmd,
            worktree,
            session_path=session_path,
            issues_path=issues_path,
                state="stopped",
                issue_id=issue_id,
                summary=f"worker={worker_id} failed issue={issue_id}",
                next_step="recover lease via MCP issue_lease_renew/issue_lease_release; retry",
                witness_ref=policy_ref,
            )
        print(
            f"[worker:{worker_id}] work/verify failed for {issue_id} "
            f"(work_rc={work_rc}, verify_rc={verify_rc})"
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
