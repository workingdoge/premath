"""
Run toy semantic vectors.

This is non-normative tooling.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Dict

from toy_gate_check import run_case


def _load(path: str) -> Any:
    return json.load(open(path, "r", encoding="utf-8"))


def _subset_ok(expect: Dict[str, Any], got: Dict[str, Any]) -> bool:
    # Compare only stable fields: result + failures[class,lawRef,witnessId]
    if expect.get("result") != got.get("result"):
        return False
    ef = expect.get("failures", [])
    gf = got.get("failures", [])
    if len(ef) != len(gf):
        return False
    for e, g in zip(ef, gf):
        for k in ("class", "lawRef", "witnessId"):
            if e.get(k) != g.get(k):
                return False
    return True


def main() -> None:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixtures", default="tests/toy/fixtures", help="fixtures dir")
    args = ap.parse_args()

    base = args.fixtures
    case_ids = sorted([d for d in os.listdir(base) if os.path.isdir(os.path.join(base, d))])

    ok = 0
    bad = 0
    for cid in case_ids:
        case_path = os.path.join(base, cid, "case.json")
        exp_path = os.path.join(base, cid, "expect.json")
        case = _load(case_path)
        expect = _load(exp_path)
        got = run_case(case)
        if _subset_ok(expect, got):
            ok += 1
        else:
            bad += 1
            print(f"[FAIL] {cid}")
            print("expected:", json.dumps(expect, indent=2, sort_keys=True))
            print("got:", json.dumps(got, indent=2, sort_keys=True))
    print(f"[toy vectors] ok={ok} bad={bad}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
