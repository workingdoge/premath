#!/usr/bin/env python3
"""Provider-neutral required-gate pipeline entrypoint."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Dict, Sequence

from harness_escalation import (
    EscalationError,
    EscalationResult,
    apply_terminal_escalation,
)
from harness_retry_policy import (
    RetryDecision,
    RetryPolicyError,
    failure_classes_from_witness_path,
    load_retry_policy,
    resolve_retry_decision,
)
from provider_env import map_github_to_premath_env


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run the provider-neutral required-gate pipeline.")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_root,
        help=f"Repository root (default: {default_root})",
    )
    parser.add_argument(
        "--summary-out",
        type=Path,
        default=None,
        help="Optional summary markdown output path (defaults to GITHUB_STEP_SUMMARY when set).",
    )
    parser.add_argument(
        "--retry-policy",
        type=Path,
        default=None,
        help="Optional retry-policy artifact path (defaults to policies/control/harness-retry-policy-v1.json).",
    )
    return parser.parse_args()


def apply_provider_env() -> Dict[str, str]:
    mapped = map_github_to_premath_env(os.environ)
    for key, value in mapped.items():
        os.environ[key] = value
    return mapped


def _write_sha(path: Path, out_path: Path) -> str:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    out_path.write_text(digest + "\n", encoding="utf-8")
    return digest


def _render_retry_lines(retry_history: Sequence[RetryDecision]) -> list[str]:
    if not retry_history:
        return []
    lines = [
        "- retry history:",
    ]
    for decision in retry_history:
        failure_line = ", ".join(decision.failure_classes) if decision.failure_classes else "(none)"
        action = "retry" if decision.retry else decision.escalation_action
        lines.append(
            (
                f"  - attempt {decision.attempt}/{decision.max_attempts}: "
                f"rule={decision.rule_id} matched={decision.matched_failure_class} "
                f"backoff={decision.backoff_class} action={action} "
                f"failureClasses={failure_line}"
            )
        )
    return lines


def _render_escalation_lines(escalation: EscalationResult | None) -> list[str]:
    if escalation is None:
        return []
    lines = [
        (
            f"- escalation: action=`{escalation.action}` "
            f"outcome=`{escalation.outcome}`"
        )
    ]
    if escalation.issue_id:
        lines.append(f"- escalation issue id: `{escalation.issue_id}`")
    if escalation.created_issue_id:
        lines.append(f"- escalation created issue id: `{escalation.created_issue_id}`")
    if escalation.note_digest:
        lines.append(f"- escalation note digest: `{escalation.note_digest}`")
    lines.append(f"- escalation witness ref: `{escalation.witness_ref}`")
    if escalation.details:
        lines.append(f"- escalation details: `{escalation.details}`")
    return lines


def render_summary(
    repo_root: Path,
    *,
    retry_history: Sequence[RetryDecision] = (),
    retry_policy_digest: str | None = None,
    retry_policy_id: str | None = None,
    escalation: EscalationResult | None = None,
) -> str:
    witness_path = repo_root / "artifacts/ciwitness/latest-required.json"
    digest_path = repo_root / "artifacts/ciwitness/latest-required.sha256"
    delta_path = repo_root / "artifacts/ciwitness/latest-delta.json"
    delta_digest_path = repo_root / "artifacts/ciwitness/latest-delta.sha256"
    decision_path = repo_root / "artifacts/ciwitness/latest-decision.json"
    decision_digest_path = repo_root / "artifacts/ciwitness/latest-decision.sha256"

    lines = ["### CI Required Attestation"]

    if not witness_path.exists():
        lines.append("")
        lines.append("- witness: missing (`artifacts/ciwitness/latest-required.json`)")
    else:
        payload = json.loads(witness_path.read_text(encoding="utf-8"))
        raw_digest = _write_sha(witness_path, digest_path)
        checks = payload.get("requiredChecks", [])
        checks_line = ", ".join(checks) if isinstance(checks, list) and checks else "(none)"
        lines.extend(
            [
                "",
                f"- projection digest: `{payload.get('projectionDigest', '(missing)')}`",
                f"- witness verdict: `{payload.get('verdictClass', '(missing)')}`",
                f"- required checks: `{checks_line}`",
                f"- witness sha256: `{raw_digest}`",
            ]
        )

    if not delta_path.exists():
        lines.append("- delta snapshot: missing (`artifacts/ciwitness/latest-delta.json`)")
    else:
        delta = json.loads(delta_path.read_text(encoding="utf-8"))
        delta_digest = _write_sha(delta_path, delta_digest_path)
        changed = delta.get("changedPaths", [])
        changed_count = len(changed) if isinstance(changed, list) else "(invalid)"
        lines.extend(
            [
                f"- delta source: `{delta.get('deltaSource', '(missing)')}`",
                f"- delta changed paths: `{changed_count}`",
                f"- delta sha256: `{delta_digest}`",
            ]
        )

    if not decision_path.exists():
        lines.append("- decision: missing (`artifacts/ciwitness/latest-decision.json`)")
    else:
        decision = json.loads(decision_path.read_text(encoding="utf-8"))
        decision_digest = _write_sha(decision_path, decision_digest_path)
        lines.extend(
            [
                f"- decision: `{decision.get('decision', '(missing)')}`",
                f"- decision reason: `{decision.get('reasonClass', '(missing)')}`",
                f"- decision witness sha256: `{decision.get('witnessSha256', '(missing)')}`",
                f"- decision delta sha256: `{decision.get('deltaSha256', '(missing)')}`",
                f"- decision sha256: `{decision_digest}`",
            ]
        )

    if retry_policy_digest:
        policy_label = retry_policy_id or "(unknown)"
        lines.append(f"- retry policy: `{policy_label}` (`{retry_policy_digest}`)")
    lines.extend(_render_retry_lines(retry_history))
    lines.extend(_render_escalation_lines(escalation))

    return "\n".join(lines) + "\n"


def write_summary(summary_text: str, summary_out: Path | None) -> None:
    target = summary_out
    if target is None:
        github_summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if github_summary:
            target = Path(github_summary)

    if target is not None:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(summary_text, encoding="utf-8")
    print(summary_text, end="")


def run_required_with_retry(
    root: Path,
    policy: Dict[str, object],
) -> tuple[int, tuple[RetryDecision, ...], EscalationResult | None]:
    retry_history: list[RetryDecision] = []
    attempt = 1
    witness_path = root / "artifacts/ciwitness/latest-required.json"
    while True:
        run = subprocess.run(["mise", "run", "ci-required-attested"], cwd=root)
        exit_code = int(run.returncode)
        if exit_code == 0:
            return 0, tuple(retry_history), None

        failure_classes = failure_classes_from_witness_path(witness_path)
        decision = resolve_retry_decision(policy, failure_classes, attempt=attempt)
        retry_history.append(decision)
        if decision.retry:
            print(
                (
                    "[pipeline-required] retry "
                    f"{attempt + 1}/{decision.max_attempts} "
                    f"(rule={decision.rule_id}, matched={decision.matched_failure_class}, "
                    f"backoff={decision.backoff_class})"
                )
            )
            attempt += 1
            continue

        try:
            escalation = apply_terminal_escalation(
                root,
                scope="required",
                decision=decision,
                policy=policy,
                witness_path=witness_path,
            )
        except EscalationError as exc:
            print(
                (
                    "[pipeline-required] escalation error "
                    f"{exc.failure_class}: {exc.reason}"
                )
            )
            escalation = EscalationResult(
                action=decision.escalation_action,
                outcome=f"error:{exc.failure_class}",
                issue_id=None,
                created_issue_id=None,
                note_digest=None,
                witness_ref=str(witness_path.relative_to(root)),
                details=exc.reason,
            )
            return 2, tuple(retry_history), escalation

        print(
            (
                "[pipeline-required] escalation "
                f"action={decision.escalation_action} outcome={escalation.outcome} "
                f"(rule={decision.rule_id}, matched={decision.matched_failure_class})"
            )
        )
        return exit_code, tuple(retry_history), escalation


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()

    mapped = apply_provider_env()
    if mapped:
        mapped_str = ", ".join(f"{k}={v}" for k, v in sorted(mapped.items()))
        print(f"[pipeline-required] mapped provider refs: {mapped_str}")

    try:
        retry_policy = load_retry_policy(root, args.retry_policy)
    except RetryPolicyError as exc:
        print(f"[pipeline-required] retry policy error: {exc.failure_class}: {exc.reason}")
        return 2

    exit_code, retry_history, escalation = run_required_with_retry(root, retry_policy)
    summary = render_summary(
        root,
        retry_history=retry_history,
        retry_policy_digest=str(retry_policy.get("policyDigest")),
        retry_policy_id=str(retry_policy.get("policyId")),
        escalation=escalation,
    )
    write_summary(summary, args.summary_out)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
