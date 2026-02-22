#!/usr/bin/env python3
"""Shared client for core `premath proposal-check` execution."""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


class ProposalCheckError(ValueError):
    """Proposal-check failure with deterministic failure class."""

    def __init__(self, failure_class: str, message: str) -> None:
        self.failure_class = failure_class
        super().__init__(f"{failure_class}: {message}")


def resolve_premath_cli(root: Path) -> List[str]:
    premath_bin = root / "target" / "debug" / "premath"
    if premath_bin.exists() and os.access(premath_bin, os.X_OK):
        return [str(premath_bin)]
    return ["cargo", "run", "--package", "premath-cli", "--"]


def _extract_failure_message(completed: subprocess.CompletedProcess[str]) -> str:
    stderr_lines = [line.strip() for line in completed.stderr.splitlines() if line.strip()]
    stdout_lines = [line.strip() for line in completed.stdout.splitlines() if line.strip()]

    for line in reversed(stderr_lines):
        if re.match(r"^[a-z0-9_]+:\s+.+$", line):
            return line
    if stderr_lines:
        return stderr_lines[-1]
    if stdout_lines:
        return stdout_lines[-1]
    return "proposal_invalid_shape: proposal-check failed"


def _validate_payload(payload: Any) -> Dict[str, Any]:
    if not isinstance(payload, dict):
        raise ProposalCheckError("proposal_invalid_shape", "proposal-check payload must be an object")

    canonical = payload.get("canonical")
    digest = payload.get("digest")
    kcir_ref = payload.get("kcirRef")
    obligations = payload.get("obligations", [])
    discharge = payload.get("discharge")

    if not isinstance(canonical, dict):
        raise ProposalCheckError(
            "proposal_invalid_shape",
            "proposal-check canonical payload must be an object",
        )
    if not isinstance(digest, str) or not digest:
        raise ProposalCheckError("proposal_nondeterministic", "proposal-check digest is missing")
    if not isinstance(kcir_ref, str) or not kcir_ref:
        raise ProposalCheckError("proposal_kcir_ref_mismatch", "proposal-check kcirRef is missing")
    if not isinstance(obligations, list):
        raise ProposalCheckError("proposal_invalid_step", "proposal-check obligations must be a list")
    if not isinstance(discharge, dict):
        raise ProposalCheckError(
            "proposal_invalid_step",
            "proposal-check discharge payload must be an object",
        )

    return {
        "canonical": canonical,
        "digest": digest,
        "kcirRef": kcir_ref,
        "obligations": obligations,
        "discharge": discharge,
    }


def run_proposal_check(root: Path, proposal: Dict[str, Any]) -> Dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="premath-proposal-check-") as tmp:
        proposal_path = Path(tmp) / "proposal.json"
        proposal_path.write_text(
            json.dumps(proposal, ensure_ascii=False),
            encoding="utf-8",
        )
        cmd = [
            *resolve_premath_cli(root),
            "proposal-check",
            "--proposal",
            str(proposal_path),
            "--json",
        ]
        completed = subprocess.run(
            cmd,
            cwd=root,
            capture_output=True,
            text=True,
        )

    if completed.returncode != 0:
        message = _extract_failure_message(completed)
        match = re.match(r"^(?P<class>[a-z0-9_]+):\s*(?P<reason>.*)$", message)
        if match:
            failure_class = match.group("class")
            reason = match.group("reason").strip() or message
            raise ProposalCheckError(failure_class, reason)
        raise ProposalCheckError("proposal_invalid_shape", message)

    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise ProposalCheckError(
            "proposal_invalid_shape",
            "proposal-check returned invalid JSON",
        ) from exc

    return _validate_payload(payload)
