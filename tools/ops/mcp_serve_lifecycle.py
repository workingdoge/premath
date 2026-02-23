#!/usr/bin/env python3
"""Lifecycle helper for repo-local premath mcp-serve processes."""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from typing import Iterable, List


@dataclass(frozen=True)
class ProcessInfo:
    pid: int
    command: str
    flavor: str


def _run_ps() -> str:
    proc = subprocess.run(
        ["ps", "-ax", "-o", "pid=,command="],
        check=True,
        capture_output=True,
        text=True,
    )
    return proc.stdout


def _flavor(command: str) -> str:
    if "--issues .premath/issues.jsonl" in command:
        return "canonical_premath"
    if "--issues .beads/issues.jsonl" in command:
        return "legacy_beads"
    if "mise run mcp-serve" in command:
        return "wrapper"
    return "unknown"


def _looks_like_mcp_serve(command: str) -> bool:
    if "mcp-serve" not in command:
        return False
    return (
        "target/debug/premath mcp-serve" in command
        or "premath mcp-serve" in command
        or "mise run mcp-serve" in command
    )


def _is_repo_local(command: str, repo_root: str) -> bool:
    return any(
        marker in command
        for marker in (
            repo_root,
            f"cd {repo_root}",
            "--issues .premath/issues.jsonl",
            "--issues .beads/issues.jsonl",
            "--repo-root .",
            "artifacts/observation/latest.json",
        )
    )


def list_repo_processes(repo_root: str) -> List[ProcessInfo]:
    rows: List[ProcessInfo] = []
    for line in _run_ps().splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        parts = stripped.split(None, 1)
        if len(parts) != 2:
            continue
        pid_raw, command = parts
        if not pid_raw.isdigit():
            continue
        pid = int(pid_raw)
        if pid == os.getpid():
            continue
        if not _looks_like_mcp_serve(command):
            continue
        if not _is_repo_local(command, repo_root):
            continue
        rows.append(ProcessInfo(pid=pid, command=command, flavor=_flavor(command)))
    rows.sort(key=lambda item: item.pid)
    return rows


def _pid_exists(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _wait_until_gone(pids: Iterable[int], timeout_seconds: float) -> List[int]:
    deadline = time.monotonic() + timeout_seconds
    pending = sorted(set(pids))
    while pending and time.monotonic() < deadline:
        pending = [pid for pid in pending if _pid_exists(pid)]
        if pending:
            time.sleep(0.1)
    return pending


def cmd_status(repo_root: str, as_json: bool) -> int:
    rows = list_repo_processes(repo_root)
    payload = {
        "action": "mcp-serve.status",
        "repoRoot": repo_root,
        "count": len(rows),
        "items": [asdict(row) for row in rows],
    }
    if as_json:
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0

    print("premath mcp-serve status")
    print(f"  repo root: {repo_root}")
    print(f"  running: {len(rows)}")
    for row in rows:
        print(f"  - pid={row.pid} flavor={row.flavor} cmd={row.command}")
    return 0


def cmd_stop(repo_root: str, timeout_seconds: float, as_json: bool) -> int:
    rows = list_repo_processes(repo_root)
    target_pids = [row.pid for row in rows]

    for pid in target_pids:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            continue

    still_running = _wait_until_gone(target_pids, timeout_seconds)

    for pid in still_running:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            continue

    still_running = _wait_until_gone(still_running, 0.5)
    stopped_count = len(target_pids) - len(still_running)

    payload = {
        "action": "mcp-serve.stop",
        "repoRoot": repo_root,
        "matched": len(target_pids),
        "stopped": stopped_count,
        "remainingPids": still_running,
    }
    if as_json:
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0 if not still_running else 1

    print("premath mcp-serve stop")
    print(f"  repo root: {repo_root}")
    print(f"  matched: {len(target_pids)}")
    print(f"  stopped: {stopped_count}")
    if still_running:
        print(f"  remaining pids: {still_running}")
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="premath mcp-serve lifecycle helper")
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root used to scope mcp-serve process matching (default: .)",
    )

    subparsers = parser.add_subparsers(dest="action", required=True)

    status_parser = subparsers.add_parser("status", help="List repo-local mcp-serve processes")
    status_parser.add_argument("--json", action="store_true", help="Emit JSON output")

    stop_parser = subparsers.add_parser("stop", help="Stop repo-local mcp-serve processes")
    stop_parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=2.0,
        help="Graceful termination wait time before SIGKILL (default: 2.0)",
    )
    stop_parser.add_argument("--json", action="store_true", help="Emit JSON output")

    args = parser.parse_args()
    repo_root = os.path.abspath(args.repo_root)

    if args.action == "status":
        return cmd_status(repo_root=repo_root, as_json=args.json)
    if args.action == "stop":
        return cmd_stop(
            repo_root=repo_root,
            timeout_seconds=args.timeout_seconds,
            as_json=args.json,
        )

    print(f"unknown action: {args.action}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
