#!/usr/bin/env python3
"""Deterministic retry-escalation bridge into `premath issue` mutations."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Mapping, Sequence

from core_cli_client import resolve_premath_cli
from harness_retry_policy import RetryDecision


ACTIVE_ISSUE_ENV_KEYS = ("PREMATH_ACTIVE_ISSUE_ID", "PREMATH_ISSUE_ID")
DEFAULT_ISSUES_REL_PATH = Path(".premath/issues.jsonl")
HARNESS_SESSION_ENV_KEY = "PREMATH_HARNESS_SESSION_PATH"
DEFAULT_HARNESS_SESSION_REL_PATH = Path(".premath/harness_session.json")
HARNESS_SESSION_KIND = "premath.harness.session.v1"


class EscalationError(ValueError):
    """Escalation execution failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        self.reason = message
        super().__init__(f"{failure_class}: {message}")


@dataclass(frozen=True)
class EscalationResult:
    """Result of terminal retry escalation handling."""

    action: str
    outcome: str
    issue_id: str | None
    created_issue_id: str | None
    note_digest: str | None
    witness_ref: str
    details: str | None = None


def _canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _stable_hash(value: Any) -> str:
    return hashlib.sha256(_canonical_json(value).encode("utf-8")).hexdigest()


def resolve_active_issue_id(env: Mapping[str, str] | None = None) -> str | None:
    source = env if env is not None else os.environ
    for key in ACTIVE_ISSUE_ENV_KEYS:
        value = source.get(key)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return None


def _resolve_active_issue_from_env(
    env: Mapping[str, str],
) -> tuple[str | None, str | None]:
    values: list[tuple[str, str]] = []
    for key in ACTIVE_ISSUE_ENV_KEYS:
        raw = env.get(key)
        if isinstance(raw, str) and raw.strip():
            values.append((key, raw.strip()))
    if not values:
        return None, None

    unique_values = {value for _, value in values}
    if len(unique_values) > 1:
        rendered = ", ".join(f"{key}={value}" for key, value in values)
        raise EscalationError(
            "escalation_issue_context_ambiguous",
            f"conflicting active issue env values: {rendered}",
        )
    key, value = values[0]
    return value, key


def resolve_harness_session_path(
    repo_root: Path,
    env: Mapping[str, str] | None = None,
) -> Path:
    source = env if env is not None else os.environ
    raw = source.get(HARNESS_SESSION_ENV_KEY)
    path = Path(raw) if isinstance(raw, str) and raw.strip() else DEFAULT_HARNESS_SESSION_REL_PATH
    if not path.is_absolute():
        path = (repo_root / path).resolve()
    return path


def _resolve_active_issue_from_harness_session(
    session_path: Path,
) -> str | None:
    if not session_path.exists():
        return None
    try:
        payload = json.loads(session_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise EscalationError(
            "escalation_session_invalid",
            f"invalid harness-session JSON at {session_path}: {exc}",
        ) from exc
    except OSError as exc:
        raise EscalationError(
            "escalation_session_read_failed",
            f"failed reading harness-session artifact {session_path}: {exc}",
        ) from exc

    if not isinstance(payload, dict):
        raise EscalationError(
            "escalation_session_invalid",
            f"harness-session payload at {session_path} must be an object",
        )
    session_kind = payload.get("sessionKind")
    if not isinstance(session_kind, str) or session_kind != HARNESS_SESSION_KIND:
        raise EscalationError(
            "escalation_session_invalid",
            (
                "invalid harness-session kind at "
                f"{session_path} (expected {HARNESS_SESSION_KIND})"
            ),
        )
    issue_id = payload.get("issueId")
    if isinstance(issue_id, str) and issue_id.strip():
        return issue_id.strip()
    return None


def _resolve_active_issue_from_ready_frontier(
    repo_root: Path,
    issues_path: Path,
    *,
    run_process: Any = subprocess.run,
) -> str | None:
    payload = _run_issue_json(
        repo_root,
        ["ready", "--issues", str(issues_path)],
        run_process=run_process,
    )
    items = payload.get("items")
    if not isinstance(items, list):
        raise EscalationError(
            "escalation_issue_output_invalid",
            "issue ready payload must include array field `items`",
        )

    issue_ids: list[str] = []
    for item in items:
        if not isinstance(item, dict):
            raise EscalationError(
                "escalation_issue_output_invalid",
                "issue ready row must be an object",
            )
        issue_id = item.get("id")
        if not isinstance(issue_id, str) or not issue_id.strip():
            raise EscalationError(
                "escalation_issue_output_invalid",
                "issue ready row missing non-empty `id`",
            )
        issue_ids.append(issue_id.strip())

    if not issue_ids:
        return None
    if len(issue_ids) > 1:
        rendered = ", ".join(issue_ids)
        raise EscalationError(
            "escalation_issue_context_ambiguous",
            f"ready frontier contains multiple issues: {rendered}",
        )
    return issue_ids[0]


def resolve_active_issue_context(
    repo_root: Path,
    env: Mapping[str, str] | None = None,
    *,
    issues_path: Path | None = None,
    run_process: Any = subprocess.run,
) -> tuple[str | None, str]:
    source = env if env is not None else os.environ
    env_issue_id, env_key = _resolve_active_issue_from_env(source)
    if env_issue_id and env_key:
        return env_issue_id, f"env:{env_key}"

    session_path = resolve_harness_session_path(repo_root, source)
    issue_from_session = _resolve_active_issue_from_harness_session(session_path)
    if issue_from_session:
        return issue_from_session, f"session:{session_path}"

    resolved_issues_path = resolve_issues_path(repo_root, source, issues_path)
    issue_from_ready = _resolve_active_issue_from_ready_frontier(
        repo_root,
        resolved_issues_path,
        run_process=run_process,
    )
    if issue_from_ready:
        return issue_from_ready, f"ready:{resolved_issues_path}"

    raise EscalationError(
        "escalation_issue_context_unbound",
        (
            "active issue context missing across env/session/ready fallback; "
            f"set one of: {', '.join(ACTIVE_ISSUE_ENV_KEYS)}, "
            f"or provide harness session at {HARNESS_SESSION_ENV_KEY}, "
            f"or ensure exactly one ready issue in {resolved_issues_path}"
        ),
    )


def resolve_issues_path(
    repo_root: Path,
    env: Mapping[str, str] | None = None,
    issues_path: Path | None = None,
) -> Path:
    if issues_path is not None:
        path = issues_path
    else:
        source = env if env is not None else os.environ
        env_path = source.get("PREMATH_ISSUES_PATH")
        path = Path(env_path) if isinstance(env_path, str) and env_path.strip() else DEFAULT_ISSUES_REL_PATH
    if not path.is_absolute():
        path = (repo_root / path).resolve()
    return path


def _extract_failure_message(completed: subprocess.CompletedProcess[str]) -> str:
    stderr_lines = [line.strip() for line in completed.stderr.splitlines() if line.strip()]
    stdout_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]
    if stderr_lines:
        return stderr_lines[-1]
    if stdout_lines:
        return stdout_lines[-1]
    return "issue command failed"


def _run_issue_json(
    repo_root: Path,
    command_args: Sequence[str],
    *,
    run_process: Any = subprocess.run,
) -> Dict[str, Any]:
    def run_once(cli_prefix: Sequence[str]) -> subprocess.CompletedProcess[str]:
        cmd = [*cli_prefix, "issue", *command_args, "--json"]
        return run_process(
            cmd,
            cwd=repo_root,
            capture_output=True,
            text=True,
        )

    cli_prefix = resolve_premath_cli(repo_root)
    completed = run_once(cli_prefix)
    if completed.returncode != 0 and cli_prefix and Path(str(cli_prefix[0])).name == "premath":
        stderr = completed.stderr + "\n" + completed.stdout
        if "unrecognized subcommand 'issue'" in stderr:
            completed = run_once(["cargo", "run", "--package", "premath-cli", "--"])

    if completed.returncode != 0:
        message = _extract_failure_message(completed)
        raise EscalationError("escalation_issue_command_failed", message)

    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise EscalationError(
            "escalation_issue_output_invalid",
            "issue command returned invalid JSON",
        ) from exc
    if not isinstance(payload, dict):
        raise EscalationError(
            "escalation_issue_output_invalid",
            "issue command payload must be an object",
        )
    return payload


def _read_issue_notes(issues_path: Path, issue_id: str) -> str:
    if not issues_path.exists():
        return ""

    latest_notes = ""
    found = False
    try:
        lines = issues_path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise EscalationError(
            "escalation_issue_read_failed",
            f"failed reading issues path {issues_path}: {exc}",
        ) from exc

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
        found = True
        notes = payload.get("notes")
        if isinstance(notes, str):
            latest_notes = notes
        else:
            latest_notes = ""

    if not found:
        raise EscalationError(
            "escalation_issue_not_found",
            f"active issue id not present in issues path: {issue_id}",
        )
    return latest_notes


def build_escalation_note(
    scope: str,
    decision: RetryDecision,
    policy: Mapping[str, Any],
    witness_ref: str,
) -> str:
    failures = ", ".join(decision.failure_classes) if decision.failure_classes else "(none)"
    return "\n".join(
        [
            "[harness-escalation]",
            f"scope: {scope}",
            f"action: {decision.escalation_action}",
            f"policyId: {policy.get('policyId', '(missing)')}",
            f"policyDigest: {policy.get('policyDigest', '(missing)')}",
            f"ruleId: {decision.rule_id}",
            f"matchedFailureClass: {decision.matched_failure_class}",
            f"attempt: {decision.attempt}/{decision.max_attempts}",
            f"failureClasses: {failures}",
            f"witnessRef: {witness_ref}",
        ]
    )


def merge_issue_notes(existing: str, addition: str) -> str:
    base = existing.strip()
    add = addition.strip()
    if not base:
        return add
    if add in base:
        return base
    return f"{base}\n\n{add}"


def apply_terminal_escalation(
    repo_root: Path,
    *,
    scope: str,
    decision: RetryDecision,
    policy: Mapping[str, Any],
    witness_path: Path,
    env: Mapping[str, str] | None = None,
    run_process: Any = subprocess.run,
    issues_path: Path | None = None,
) -> EscalationResult:
    action = decision.escalation_action
    witness_ref = (
        str(witness_path.relative_to(repo_root))
        if witness_path.is_relative_to(repo_root)
        else str(witness_path)
    )
    if action == "stop":
        return EscalationResult(
            action=action,
            outcome="stop",
            issue_id=None,
            created_issue_id=None,
            note_digest=None,
            witness_ref=witness_ref,
            details="terminal stop with no mutation",
        )

    resolved_issues_path = resolve_issues_path(repo_root, env, issues_path)
    active_issue_id, issue_source = resolve_active_issue_context(
        repo_root,
        env,
        issues_path=resolved_issues_path,
        run_process=run_process,
    )
    note = build_escalation_note(scope, decision, policy, witness_ref)
    note_digest = "note1_" + _stable_hash({"scope": scope, "note": note})

    if action == "issue_discover":
        title = (
            f"[HarnessEscalation] {scope} retry exhausted "
            f"({decision.matched_failure_class})"
        )
        payload = _run_issue_json(
            repo_root,
            [
                "discover",
                active_issue_id,
                title,
                "--description",
                note,
                "--issues",
                str(resolved_issues_path),
            ],
            run_process=run_process,
        )
        issue = payload.get("issue")
        created_issue_id = None
        if isinstance(issue, dict):
            created = issue.get("id")
            if isinstance(created, str) and created.strip():
                created_issue_id = created.strip()
        return EscalationResult(
            action=action,
            outcome="applied",
            issue_id=active_issue_id,
            created_issue_id=created_issue_id,
            note_digest=note_digest,
            witness_ref=witness_ref,
            details=f"issuesPath={resolved_issues_path}; issueSource={issue_source}",
        )

    if action == "mark_blocked":
        existing_notes = _read_issue_notes(resolved_issues_path, active_issue_id)
        merged_notes = merge_issue_notes(existing_notes, note)
        _run_issue_json(
            repo_root,
            [
                "update",
                active_issue_id,
                "--status",
                "blocked",
                "--notes",
                merged_notes,
                "--issues",
                str(resolved_issues_path),
            ],
            run_process=run_process,
        )
        return EscalationResult(
            action=action,
            outcome="applied",
            issue_id=active_issue_id,
            created_issue_id=None,
            note_digest=note_digest,
            witness_ref=witness_ref,
            details=f"issuesPath={resolved_issues_path}; issueSource={issue_source}",
        )

    raise EscalationError("escalation_unknown_action", f"unsupported escalation action: {action}")
