#!/usr/bin/env python3
"""Provider-neutral instruction pipeline entrypoint."""

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
from control_plane_contract import (
    INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT,
    INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT,
)
from governance_gate import governance_failure_classes
from kcir_mapping_gate import (
    MappingGateReport,
    evaluate_instruction_mapping,
    render_mapping_summary_lines,
)
from provider_env import map_github_to_premath_env


def parse_args(default_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run provider-neutral instruction pipeline.")
    parser.add_argument(
        "--instruction",
        type=Path,
        required=True,
        help="Instruction envelope path.",
    )
    parser.add_argument(
        "--allow-failure",
        action="store_true",
        help="Allow failing checks but still emit witness.",
    )
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


def _resolve_instruction(root: Path, instruction: Path) -> Path:
    if instruction.is_absolute():
        resolved = instruction
    else:
        resolved = (root / instruction).resolve()
    if not resolved.exists():
        raise FileNotFoundError(f"instruction file not found: {resolved}")
    return resolved


def _instruction_id(path: Path) -> str:
    if path.suffix != ".json":
        raise ValueError("instruction filename must end with .json")
    return path.stem


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
    instruction_id: str,
    *,
    retry_history: Sequence[RetryDecision] = (),
    retry_policy_digest: str | None = None,
    retry_policy_id: str | None = None,
    escalation: EscalationResult | None = None,
    mapping_report: MappingGateReport | None = None,
) -> str:
    witness_path = repo_root / "artifacts/ciwitness" / f"{instruction_id}.json"
    witness_sha_path = repo_root / "artifacts/ciwitness" / f"{instruction_id}.sha256"

    lines = ["### CI Instruction Witness"]
    if not witness_path.exists():
        lines.append("")
        lines.append(f"- witness: missing (`{witness_path.relative_to(repo_root)}`)")
        return "\n".join(lines) + "\n"

    payload = json.loads(witness_path.read_text(encoding="utf-8"))
    digest = _write_sha(witness_path, witness_sha_path)
    required = payload.get("requiredChecks", [])
    required_line = ", ".join(required) if isinstance(required, list) else "(invalid)"
    executed = payload.get("executedChecks", [])
    executed_line = ", ".join(executed) if isinstance(executed, list) else "(invalid)"
    lines.extend(
        [
            "",
            f"- instruction id: `{payload.get('instructionId', '(missing)')}`",
            f"- instruction digest: `{payload.get('instructionDigest', '(missing)')}`",
            f"- normalizer id: `{payload.get('normalizerId', '(missing)')}`",
            f"- verdict: `{payload.get('verdictClass', '(missing)')}`",
            f"- required checks: `{required_line}`",
            f"- executed checks: `{executed_line}`",
            f"- witness sha256: `{digest}`",
        ]
    )
    proposal_ingest = payload.get("proposalIngest")
    if isinstance(proposal_ingest, dict):
        proposal_digest = proposal_ingest.get("proposalDigest", "(missing)")
        proposal_kcir_ref = proposal_ingest.get("proposalKcirRef", "(missing)")
        proposal_kind = proposal_ingest.get("kind", "(missing)")
        obligations = proposal_ingest.get("obligations", [])
        obligation_count = len(obligations) if isinstance(obligations, list) else 0
        discharge = proposal_ingest.get("discharge")
        discharge_outcome = "(missing)"
        discharge_failures = "(none)"
        if isinstance(discharge, dict):
            discharge_outcome = str(discharge.get("outcome", "(missing)"))
            failures = discharge.get("failureClasses", [])
            if isinstance(failures, list) and failures:
                discharge_failures = ", ".join(
                    sorted({str(item) for item in failures if isinstance(item, str) and item})
                )
        lines.extend(
            [
                f"- proposal kind: `{proposal_kind}`",
                f"- proposal digest: `{proposal_digest}`",
                f"- proposal kcir ref: `{proposal_kcir_ref}`",
                f"- proposal obligations: `{obligation_count}`",
                f"- proposal discharge: `{discharge_outcome}`",
                f"- proposal discharge failures: `{discharge_failures}`",
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


def run_instruction_once(
    root: Path,
    instruction_path: Path,
    allow_failure: bool,
) -> tuple[int, tuple[str, ...]]:
    validate_cmd = list(INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT) + [
        str(instruction_path)
    ]
    validate = _run_with_streamed_output(
        validate_cmd,
        cwd=root,
    )
    validate_failure_classes = failure_classes_from_completed_process(validate)
    if validate.returncode != 0:
        reject_cmd = list(INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT) + [
            str(instruction_path)
        ]
        reject_run = _run_with_streamed_output(
            reject_cmd,
            cwd=root,
        )
        reject_failure_classes = failure_classes_from_completed_process(reject_run)
        return (
            int(reject_run.returncode or validate.returncode),
            combine_failure_class_sources(reject_failure_classes, validate_failure_classes),
        )

    run_cmd = list(INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT) + [str(instruction_path)]
    if allow_failure:
        run_cmd.append("--allow-failure")
    run = _run_with_streamed_output(run_cmd, cwd=root)
    return int(run.returncode), failure_classes_from_completed_process(run)


def run_instruction_with_retry(
    root: Path,
    instruction_path: Path,
    instruction_id: str,
    policy: Dict[str, object],
    *,
    allow_failure: bool,
) -> tuple[int, tuple[RetryDecision, ...], EscalationResult | None]:
    retry_history: list[RetryDecision] = []
    witness_path = root / "artifacts/ciwitness" / f"{instruction_id}.json"
    attempt = 1
    while True:
        exit_code, process_failure_classes = run_instruction_once(root, instruction_path, allow_failure)
        governance_classes = governance_failure_classes(root)
        if governance_classes:
            process_failure_classes = combine_failure_class_sources(
                process_failure_classes,
                governance_classes,
            )
            if exit_code == 0:
                joined = ", ".join(governance_classes)
                print(
                    "[pipeline-instruction] governance promotion gate rejected successful run "
                    f"(failureClasses={joined})"
                )
                exit_code = 1
        mapping_report = evaluate_instruction_mapping(
            root,
            instruction_path=instruction_path,
            instruction_id=instruction_id,
            strict=(exit_code == 0),
        )
        if mapping_report.failure_classes:
            process_failure_classes = combine_failure_class_sources(
                process_failure_classes,
                mapping_report.failure_classes,
            )
            if exit_code == 0:
                joined = ", ".join(mapping_report.failure_classes)
                print(
                    "[pipeline-instruction] kcir mapping gate rejected successful run "
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
                    "[pipeline-instruction] retry "
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
                scope="instruction",
                decision=decision,
                policy=policy,
                witness_path=witness_path,
            )
        except EscalationError as exc:
            print(
                (
                    "[pipeline-instruction] escalation error "
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
                "[pipeline-instruction] escalation "
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
        print(f"[pipeline-instruction] mapped provider refs: {mapped_str}")

    try:
        retry_policy = load_retry_policy(root, args.retry_policy)
    except RetryPolicyError as exc:
        print(f"[pipeline-instruction] retry policy error: {exc.failure_class}: {exc.reason}")
        return 2

    instruction_path = _resolve_instruction(root, args.instruction)
    instruction_id = _instruction_id(instruction_path)

    allow_failure_env = os.environ.get("ALLOW_FAILURE", "").strip().lower()
    allow_failure = args.allow_failure or allow_failure_env in {"1", "true", "yes", "on"}

    exit_code, retry_history, escalation = run_instruction_with_retry(
        root,
        instruction_path,
        instruction_id,
        retry_policy,
        allow_failure=allow_failure,
    )
    summary = render_summary(
        root,
        instruction_id,
        retry_history=retry_history,
        retry_policy_digest=str(retry_policy.get("policyDigest")),
        retry_policy_id=str(retry_policy.get("policyId")),
        escalation=escalation,
        mapping_report=evaluate_instruction_mapping(
            root,
            instruction_path=instruction_path,
            instruction_id=instruction_id,
            strict=False,
        ),
    )
    write_summary(summary, args.summary_out)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
