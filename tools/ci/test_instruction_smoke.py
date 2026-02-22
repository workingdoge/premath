#!/usr/bin/env python3
"""Smoke test for deterministic instruction witness core fields."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Dict, List


def parse_args(default_instruction: Path, default_repo_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run deterministic instruction witness smoke test.")
    parser.add_argument(
        "--instruction",
        type=Path,
        default=default_instruction,
        help=f"Instruction envelope path (default: {default_instruction})",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=default_repo_root,
        help=f"Repository root (default: {default_repo_root})",
    )
    return parser.parse_args()


def run_instruction(repo_root: Path, instruction: Path, out_dir: Path) -> Path:
    cmd = [
        "python3",
        str(repo_root / "tools/ci/run_instruction.py"),
        str(instruction),
        "--out-dir",
        str(out_dir),
    ]
    subprocess.run(cmd, cwd=repo_root, check=True)
    witness_path = out_dir / f"{instruction.stem}.json"
    if not witness_path.exists():
        raise FileNotFoundError(f"expected witness not found: {witness_path}")
    return witness_path


def normalize_results(rows: Any) -> List[Dict[str, Any]]:
    if not isinstance(rows, list):
        return []
    normalized: List[Dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        normalized.append(
            {
                "checkId": row.get("checkId"),
                "status": row.get("status"),
                "exitCode": row.get("exitCode"),
            }
        )
    return sorted(normalized, key=lambda item: str(item.get("checkId", "")))


def normalized_core(payload: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "ciSchema": payload.get("ciSchema"),
        "witnessKind": payload.get("witnessKind"),
        "instructionDigest": payload.get("instructionDigest"),
        "instructionClassification": payload.get("instructionClassification"),
        "typingPolicy": payload.get("typingPolicy"),
        "proposalIngest": payload.get("proposalIngest"),
        "intent": payload.get("intent"),
        "scope": payload.get("scope"),
        "normalizerId": payload.get("normalizerId"),
        "policyDigest": payload.get("policyDigest"),
        "capabilityClaims": payload.get("capabilityClaims"),
        "requiredChecks": payload.get("requiredChecks"),
        "executedChecks": payload.get("executedChecks"),
        "verdictClass": payload.get("verdictClass"),
        "operationalFailureClasses": payload.get("operationalFailureClasses"),
        "semanticFailureClasses": payload.get("semanticFailureClasses"),
        "failureClasses": payload.get("failureClasses"),
        "squeakSiteProfile": payload.get("squeakSiteProfile"),
        "results": normalize_results(payload.get("results")),
    }


def main() -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(
        repo_root / "tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json",
        repo_root,
    )
    root = args.repo_root.resolve()

    instruction = args.instruction
    if not instruction.is_absolute():
        instruction = (root / instruction).resolve()
    if not instruction.exists():
        raise FileNotFoundError(f"instruction fixture not found: {instruction}")

    with tempfile.TemporaryDirectory(prefix="premath-instr-smoke-") as tmp:
        tmp_root = Path(tmp)
        run_a = tmp_root / "run-a"
        run_b = tmp_root / "run-b"
        run_a.mkdir(parents=True, exist_ok=True)
        run_b.mkdir(parents=True, exist_ok=True)

        witness_a_path = run_instruction(root, instruction, run_a)
        witness_b_path = run_instruction(root, instruction, run_b)

        witness_a = json.loads(witness_a_path.read_text(encoding="utf-8"))
        witness_b = json.loads(witness_b_path.read_text(encoding="utf-8"))

    core_a = normalized_core(witness_a)
    core_b = normalized_core(witness_b)

    if core_a != core_b:
        print("[instruction-smoke] FAIL (non-deterministic core fields)")
        print("[instruction-smoke] core_a=")
        print(json.dumps(core_a, indent=2, ensure_ascii=False))
        print("[instruction-smoke] core_b=")
        print(json.dumps(core_b, indent=2, ensure_ascii=False))
        return 1

    print("[instruction-smoke] OK (deterministic witness core fields)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
