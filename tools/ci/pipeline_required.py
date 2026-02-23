#!/usr/bin/env python3
"""Provider-neutral required-gate pipeline entrypoint."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
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
    classify_failure_classes,
    combine_failure_class_sources,
    failure_classes_from_completed_process,
    failure_classes_from_witness_path,
    load_retry_policy,
    resolve_retry_decision,
)
from control_plane_contract import REQUIRED_DECISION_CANONICAL_ENTRYPOINT
from governance_gate import governance_failure_classes
from kcir_mapping_gate import (
    MappingGateReport,
    evaluate_required_mapping,
    render_mapping_summary_lines,
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


def _run_with_streamed_output(cmd: Sequence[str], *, cwd: Path) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=True,
        text=True,
    )
    if completed.stdout:
        print(completed.stdout, end="")
    if completed.stderr:
        print(completed.stderr, end="", file=sys.stderr)
    return completed


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
    mapping_report: MappingGateReport | None = None,
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
        typed_projection = payload.get("typedCoreProjectionDigest")
        authority_alias = payload.get("authorityPayloadDigest")
        lines.extend(
            [
                "",
                f"- typed authority digest: `{typed_projection or '(missing)'}`",
                f"- compatibility alias digest: `{authority_alias or '(missing)'}`",
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
        typed_projection = decision.get("typedCoreProjectionDigest")
        authority_alias = decision.get("authorityPayloadDigest")
        lines.extend(
            [
                f"- decision: `{decision.get('decision', '(missing)')}`",
                f"- decision reason: `{decision.get('reasonClass', '(missing)')}`",
                f"- decision typed authority: `{typed_projection or '(missing)'}`",
                f"- decision compatibility alias: `{authority_alias or '(missing)'}`",
                f"- decision witness sha256: `{decision.get('witnessSha256', '(missing)')}`",
                f"- decision delta sha256: `{decision.get('deltaSha256', '(missing)')}`",
                f"- decision sha256: `{decision_digest}`",
            ]
        )

    if retry_policy_digest:
        policy_label = retry_policy_id or "(unknown)"
        lines.append(f"- retry policy: `{policy_label}` (`{retry_policy_digest}`)")
    if mapping_report is not None:
        lines.extend(render_mapping_summary_lines(mapping_report))
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
        run = _run_with_streamed_output(
            list(REQUIRED_DECISION_CANONICAL_ENTRYPOINT),
            cwd=root,
        )
        exit_code = int(run.returncode)
        process_failure_classes = failure_classes_from_completed_process(run)
        governance_classes = governance_failure_classes(root)
        if governance_classes:
            process_failure_classes = combine_failure_class_sources(
                process_failure_classes,
                governance_classes,
            )
            if exit_code == 0:
                joined = ", ".join(governance_classes)
                print(
                    "[pipeline-required] governance promotion gate rejected successful run "
                    f"(failureClasses={joined})"
                )
                exit_code = 1
        mapping_report = evaluate_required_mapping(root, strict=(exit_code == 0))
        if mapping_report.failure_classes:
            process_failure_classes = combine_failure_class_sources(
                process_failure_classes,
                mapping_report.failure_classes,
            )
            if exit_code == 0:
                joined = ", ".join(mapping_report.failure_classes)
                print(
                    "[pipeline-required] kcir mapping gate rejected successful run "
                    f"(failureClasses={joined})"
                )
                exit_code = 1
        if exit_code == 0:
            return 0, tuple(retry_history), None

        witness_failure_classes = failure_classes_from_witness_path(witness_path)
        failure_classes = classify_failure_classes(
            witness_failure_classes,
            process_failure_classes,
        )
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
        mapping_report=evaluate_required_mapping(root, strict=False),
    )
    write_summary(summary, args.summary_out)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
