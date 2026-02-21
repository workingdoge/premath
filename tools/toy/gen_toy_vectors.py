"""
Generate toy semantic vectors under tests/toy/fixtures.

This is non-normative tooling.
"""

from __future__ import annotations

import json
import os
from typing import Any, Dict, List

from toy_gate_check import run_case


def _write_case(base_dir: str, case_id: str, case: Dict[str, Any]) -> None:
    d = os.path.join(base_dir, case_id)
    os.makedirs(d, exist_ok=True)
    case_path = os.path.join(d, "case.json")
    expect_path = os.path.join(d, "expect.json")

    with open(case_path, "w", encoding="utf-8") as f:
        json.dump(case, f, indent=2, sort_keys=True)

    expect = run_case(case)
    with open(expect_path, "w", encoding="utf-8") as f:
        json.dump(expect, f, indent=2, sort_keys=True)


def generate(base_dir: str) -> None:
    # 1) Golden descent: SheafBits gluing
    case1 = {
        "schema": 1,
        "world": "sheaf_bits",
        "check": {
            "kind": "descent",
            "baseMask": 0b111,          # {0,1,2}
            "legs": [0b011, 0b110],     # {0,1}, {1,2}
            "locals": [
                {"0": 1, "1": 0},       # on {0,1}
                {"1": 0, "2": 1},       # on {1,2} agrees on overlap {1}
            ],
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "golden_descent_sheaf_bits", case1)

    # 1b) Golden descent: SheafBits gluing with 3 legs (exercises triple-overlap coherence)
    case1b = {
        "schema": 1,
        "world": "sheaf_bits",
        "check": {
            "kind": "descent",
            "baseMask": 0b1111,           # {0,1,2,3}
            "legs": [0b0111, 0b1011, 0b1101],  # {0,1,2}, {0,1,3}, {0,2,3}
            "locals": [
                {"0": 1, "1": 0, "2": 1},
                {"0": 1, "1": 0, "3": 0},
                {"0": 1, "2": 1, "3": 0},
            ],
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "golden_descent_sheaf_bits_cocycle", case1b)

    # 2) Golden stability: SheafBits restriction composition
    case2 = {
        "schema": 1,
        "world": "sheaf_bits",
        "check": {
            "kind": "stability",
            "gammaMask": 0b111,  # {0,1,2}
            "a": {"0": 1, "1": 0, "2": 1},
            "f": {"src": 0b011, "tgt": 0b111},  # {0,1}->{0,1,2}
            "g": {"src": 0b001, "tgt": 0b011},  # {0}->{0,1}
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "golden_stability_sheaf_bits", case2)

    # 3) Adversarial: descent failure (bad_constant on disjoint cover)
    case3 = {
        "schema": 1,
        "world": "bad_constant",
        "check": {
            "kind": "descent",
            "baseMask": 0b011,      # {0,1}
            "legs": [0b001, 0b010], # {0}, {1} disjoint overlap empty
            "locals": [0, 1],       # compatible on empty (both -> None), but no global
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "adversarial_descent_failure_bad_constant", case3)

    # 4) Adversarial: non-contractible glue (non_separated)
    case4 = {
        "schema": 1,
        "world": "non_separated",
        "check": {
            "kind": "descent",
            "baseMask": 0b011,
            "legs": [0b001, 0b010],
            "locals": [0, 0],  # both legs see 0, but globals 0 and 1 both restrict to 0
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "adversarial_glue_non_contractible_non_separated", case4)

    # 5) Adversarial: stability failure (bad_stability)
    case5 = {
        "schema": 1,
        "world": "bad_stability",
        "check": {
            "kind": "stability",
            "gammaMask": 0b111,     # {0,1,2}
            "a": 1,                 # global value 1
            "f": {"src": 0b011, "tgt": 0b111},  # {0,1}->{0,1,2}
            "g": {"src": 0b001, "tgt": 0b011},  # {0}->{0,1}
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "adversarial_stability_failure_bad_stability", case5)

    # 6) Adversarial: locality failure (partial_restrict can't restrict to singleton)
    case6 = {
        "schema": 1,
        "world": "partial_restrict",
        "check": {
            "kind": "locality",
            "gammaMask": 0b011,      # {0,1}
            "a": 1,
            "legs": [0b001, 0b010],  # singletons
            "tokenPath": None,
        },
    }
    _write_case(base_dir, "adversarial_locality_failure_partial_restrict", case6)


def main() -> None:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="tests/toy/fixtures", help="output fixtures dir")
    args = ap.parse_args()
    generate(args.out)


if __name__ == "__main__":
    main()
