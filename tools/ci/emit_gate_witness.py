#!/usr/bin/env python3
"""Emit a deterministic GateWitnessEnvelope artifact for one gate check."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from gate_witness_envelope import make_gate_witness_envelope


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Emit deterministic gate witness artifact.")
    parser.add_argument("--check-id", required=True, help="Gate check identifier.")
    parser.add_argument("--exit-code", required=True, type=int, help="Executed check exit code.")
    parser.add_argument(
        "--projection-digest",
        required=True,
        help="Projection digest for ci.required run identity material.",
    )
    parser.add_argument(
        "--policy-digest",
        required=True,
        help="Policy digest for run identity material.",
    )
    parser.add_argument("--ctx-ref", required=True, help="Context ref for run identity.")
    parser.add_argument("--data-head-ref", required=True, help="Data head ref for run identity.")
    parser.add_argument(
        "--source",
        choices=["native", "fallback"],
        default="fallback",
        help="Provenance source label for emitted witness.",
    )
    parser.add_argument("--out", required=True, type=Path, help="Output artifact path.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    envelope = make_gate_witness_envelope(
        check_id=args.check_id,
        exit_code=args.exit_code,
        projection_digest=args.projection_digest,
        policy_digest=args.policy_digest,
        ctx_ref=args.ctx_ref,
        data_head_ref=args.data_head_ref,
    )
    envelope["witnessSource"] = args.source

    out_path = args.out
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as f:
        json.dump(envelope, f, indent=2, ensure_ascii=False)
        f.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
