#!/usr/bin/env python3
"""Provider-neutral required-gate pipeline entrypoint."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Dict

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


def render_summary(repo_root: Path) -> str:
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


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    root = args.repo_root.resolve()

    mapped = apply_provider_env()
    if mapped:
        mapped_str = ", ".join(f"{k}={v}" for k, v in sorted(mapped.items()))
        print(f"[pipeline-required] mapped provider refs: {mapped_str}")

    run = subprocess.run(["mise", "run", "ci-required-attested"], cwd=root)
    summary = render_summary(root)
    write_summary(summary, args.summary_out)
    return int(run.returncode)


if __name__ == "__main__":
    raise SystemExit(main())
