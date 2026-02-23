#!/usr/bin/env python3
"""Deterministic throughput KPI benchmark over harness trajectory projections."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
SUCCESS_CLASSES = {"accepted", "verified_accept", "completed", "success", "ok", "passed"}


@dataclass(frozen=True)
class TrajectoryRow:
    step_id: str
    issue_id: str
    action: str
    result_class: str
    finished_at: datetime


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compute canonical multithread throughput KPI: "
            "(completed issues/day per active worker) * gate pass-rate."
        )
    )
    parser.add_argument(
        "--trajectory-path",
        default=".premath/harness_trajectory.jsonl",
        help="Harness trajectory path (default: .premath/harness_trajectory.jsonl)",
    )
    parser.add_argument(
        "--session-path",
        default=".premath/harness_session.json",
        help="Harness session projection reference path (default: .premath/harness_session.json)",
    )
    parser.add_argument(
        "--issues-path",
        default=".premath/issues.jsonl",
        help="Issue-memory authority path (default: .premath/issues.jsonl)",
    )
    parser.add_argument(
        "--window-hours",
        type=int,
        default=24,
        help="Benchmark lookback window in hours (default: 24)",
    )
    parser.add_argument(
        "--target-kpi",
        type=float,
        default=0.8,
        help="Target KPI threshold (default: 0.8)",
    )
    parser.add_argument(
        "--rollback-kpi",
        type=float,
        default=0.4,
        help="Rollback KPI threshold (default: 0.4)",
    )
    parser.add_argument(
        "--min-sample-rows",
        type=int,
        default=3,
        help="Minimum window rows before threshold decisions (default: 3)",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON payload")
    return parser.parse_args()


def resolve_path(repo_root: Path, raw: str) -> Path:
    path = Path(raw)
    if path.is_absolute():
        return path
    return (repo_root / path).resolve()


def parse_rfc3339(raw: str, *, label: str) -> datetime:
    text = raw.strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    try:
        value = datetime.fromisoformat(text)
    except ValueError as exc:
        raise RuntimeError(f"{label} must be RFC3339: {raw!r}") from exc
    if value.tzinfo is None:
        value = value.replace(tzinfo=timezone.utc)
    return value.astimezone(timezone.utc)


def normalize_result_class(raw: str) -> str:
    return raw.strip().lower().replace("-", "_")


def is_success(raw: str) -> bool:
    return normalize_result_class(raw) in SUCCESS_CLASSES


def parse_worker_id(step_id: str) -> str:
    cleaned = step_id.strip()
    if not cleaned:
        return "unknown"
    return cleaned.split(".", 1)[0] or "unknown"


def load_trajectory_rows(path: Path) -> list[TrajectoryRow]:
    if not path.exists():
        return []
    if not path.is_file():
        raise RuntimeError(f"trajectory path is not a file: {path}")

    rows: list[TrajectoryRow] = []
    for index, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw_line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as exc:
            raise RuntimeError(f"invalid JSON in {path}:{index}: {exc}") from exc
        if not isinstance(payload, dict):
            raise RuntimeError(f"trajectory row must be object ({path}:{index})")

        step_id = str(payload.get("stepId", "")).strip()
        action = str(payload.get("action", "")).strip()
        result_class = str(payload.get("resultClass", "")).strip()
        finished_at_raw = str(payload.get("finishedAt", "")).strip()
        issue_id = str(payload.get("issueId", "")).strip()
        if not step_id or not action or not result_class or not finished_at_raw:
            raise RuntimeError(
                "trajectory row missing required fields "
                f"(stepId/action/resultClass/finishedAt) at {path}:{index}"
            )
        finished_at = parse_rfc3339(finished_at_raw, label=f"{path}:{index}:finishedAt")
        rows.append(
            TrajectoryRow(
                step_id=step_id,
                issue_id=issue_id,
                action=action,
                result_class=result_class,
                finished_at=finished_at,
            )
        )
    rows.sort(key=lambda row: (row.finished_at, row.step_id, row.action), reverse=True)
    return rows


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    if args.window_hours <= 0:
        raise RuntimeError("--window-hours must be > 0")
    if args.min_sample_rows < 1:
        raise RuntimeError("--min-sample-rows must be >= 1")
    if args.target_kpi <= 0:
        raise RuntimeError("--target-kpi must be > 0")
    if args.rollback_kpi <= 0:
        raise RuntimeError("--rollback-kpi must be > 0")
    if args.target_kpi <= args.rollback_kpi:
        raise RuntimeError("--target-kpi must be greater than --rollback-kpi")

    repo_root = REPO_ROOT
    trajectory_path = resolve_path(repo_root, args.trajectory_path)
    session_path = resolve_path(repo_root, args.session_path)
    issues_path = resolve_path(repo_root, args.issues_path)

    rows = load_trajectory_rows(trajectory_path)
    if rows:
        window_end = rows[0].finished_at
    else:
        window_end = datetime(1970, 1, 1, tzinfo=timezone.utc)
    window_start = window_end - timedelta(hours=args.window_hours)

    window_rows = [row for row in rows if window_start <= row.finished_at <= window_end]
    row_count = len(window_rows)
    completed_rows = [row for row in window_rows if row.issue_id and is_success(row.result_class)]
    completed_count = len(completed_rows)
    failed_count = row_count - completed_count

    worker_ids = sorted({parse_worker_id(row.step_id) for row in window_rows if row.issue_id})
    active_workers = len(worker_ids)
    pass_rate = (completed_count / row_count) if row_count > 0 else 0.0
    completed_per_day = completed_count * (24.0 / float(args.window_hours))
    throughput_per_worker_per_day = completed_per_day / float(max(active_workers, 1))
    kpi = throughput_per_worker_per_day * pass_rate

    if row_count < args.min_sample_rows:
        decision = "insufficient_data"
        reason = "window row count below minimum sample threshold"
    elif kpi < args.rollback_kpi:
        decision = "rollback"
        reason = "kpi below rollback threshold"
    elif kpi < args.target_kpi:
        decision = "watch"
        reason = "kpi below target threshold"
    else:
        decision = "pass"
        reason = "kpi at or above target threshold"

    def r6(value: float) -> float:
        return round(value, 6)

    return {
        "schema": 1,
        "kpiKind": "premath.multithread.throughput.v1",
        "windowHours": args.window_hours,
        "windowStart": window_start.isoformat().replace("+00:00", "Z"),
        "windowEnd": window_end.isoformat().replace("+00:00", "Z"),
        "counts": {
            "windowRows": row_count,
            "completedRows": completed_count,
            "failedRows": failed_count,
            "activeWorkers": active_workers,
            "workerIds": worker_ids,
        },
        "ratios": {
            "gatePassRate": r6(pass_rate),
            "throughputPerWorkerPerDay": r6(throughput_per_worker_per_day),
            "kpi": r6(kpi),
        },
        "thresholds": {
            "targetKpi": r6(args.target_kpi),
            "rollbackKpi": r6(args.rollback_kpi),
            "minSampleRows": args.min_sample_rows,
        },
        "decision": {
            "state": decision,
            "reason": reason,
        },
        "refs": {
            "trajectoryPath": str(trajectory_path),
            "sessionPath": str(session_path),
            "issuesPath": str(issues_path),
        },
    }


def print_text(report: dict[str, Any]) -> None:
    ratios = report["ratios"]
    counts = report["counts"]
    decision = report["decision"]
    thresholds = report["thresholds"]

    print("[harness-kpi] deterministic throughput benchmark")
    print(f"  window: {report['windowStart']} -> {report['windowEnd']} ({report['windowHours']}h)")
    print(
        "  counts: "
        f"rows={counts['windowRows']} completed={counts['completedRows']} "
        f"failed={counts['failedRows']} workers={counts['activeWorkers']}"
    )
    print(
        "  ratios: "
        f"pass_rate={ratios['gatePassRate']} "
        f"throughput_per_worker_day={ratios['throughputPerWorkerPerDay']} "
        f"kpi={ratios['kpi']}"
    )
    print(
        "  thresholds: "
        f"target={thresholds['targetKpi']} rollback={thresholds['rollbackKpi']} "
        f"min_rows={thresholds['minSampleRows']}"
    )
    print(f"  decision: {decision['state']} ({decision['reason']})")


def main() -> int:
    args = parse_args()
    try:
        report = build_report(args)
    except RuntimeError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True))
    else:
        print_text(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
