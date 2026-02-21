"""
Toy Gate checker for the toy semantic vector suite.

This is non-normative tooling. It implements a tiny subset of Gate semantics:
- stability (contravariant functor law)
- locality (restrictions exist)
- descent existence + contractible uniqueness (set-level)
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple

from toy_worlds import get_world
from witness_id import witness_id


def _mask_is_subset(a: int, b: int) -> bool:
    return (a & ~b) == 0


def _overlaps(legs: List[int]) -> List[Tuple[int, int, int]]:
    out = []
    for i in range(len(legs)):
        for j in range(i + 1, len(legs)):
            out.append((i, j, legs[i] & legs[j]))
    return out


def _triple_overlaps(legs: List[int]) -> List[Tuple[int, int, int, int]]:
    """Return list of (i,j,k, mask) for triple overlaps."""
    out = []
    for i in range(len(legs)):
        for j in range(i + 1, len(legs)):
            for k in range(j + 1, len(legs)):
                out.append((i, j, k, legs[i] & legs[j] & legs[k]))
    return out


def _gate_reject(cls: str, law_ref: str, *, token_path: Optional[str], context: Optional[Dict[str, Any]], msg: str) -> Dict[str, Any]:
    wid = witness_id(cls=cls, law_ref=law_ref, token_path=token_path, context=context)
    return {
        "witnessId": wid,
        "class": cls,
        "lawRef": law_ref,
        "message": msg,
        "tokenPath": token_path,
        "context": context,
    }


def run_case(case: Dict[str, Any]) -> Dict[str, Any]:
    assert case.get("schema") == 1, "unsupported case schema"
    world = get_world(case["world"])
    check = case["check"]
    kind = check["kind"]
    token_path = check.get("tokenPath", None)

    failures: List[Dict[str, Any]] = []

    if kind == "locality":
        gamma = int(check["gammaMask"])
        a = check["a"]
        legs = [int(x) for x in check["legs"]]
        # validate legs
        if any(not _mask_is_subset(l, gamma) for l in legs):
            raise ValueError("leg not subset of gamma")
        if not world.validate(gamma, a):
            failures.append(_gate_reject(
                "locality_failure",
                "GATE-3.2",
                token_path=token_path,
                context={"mask": gamma},
                msg="claimed definable is not in Def(gamma) for this world",
            ))
        else:
            for l in legs:
                r = world.restrict(l, gamma, a)
                if r is None or not world.validate(l, r):
                    failures.append(_gate_reject(
                        "locality_failure",
                        "GATE-3.2",
                        token_path=token_path,
                        context={"mask": l},
                        msg="restriction along a cover leg is undefined or ill-typed",
                    ))
                    break

    elif kind == "stability":
        gamma = int(check["gammaMask"])
        a = check["a"]
        f = check["f"]  # src->tgt
        g = check["g"]
        f_src, f_tgt = int(f["src"]), int(f["tgt"])
        g_src, g_tgt = int(g["src"]), int(g["tgt"])

        # Type checks: f: Gamma'->Gamma, g: Gamma''->Gamma'
        if f_tgt != gamma:
            raise ValueError("f.tgt must equal gammaMask")
        if g_tgt != f_src:
            raise ValueError("g.tgt must equal f.src")
        if not _mask_is_subset(f_src, f_tgt) or not _mask_is_subset(g_src, g_tgt):
            raise ValueError("maps must be inclusions")

        if not world.validate(gamma, a):
            failures.append(_gate_reject(
                "stability_failure",
                "GATE-3.1",
                token_path=token_path,
                context={"mask": gamma},
                msg="claimed definable is not in Def(gamma) for this world",
            ))
        else:
            # (f o g)* a == g*(f* a)
            fg = world.restrict(g_src, gamma, a)  # restrict directly to g_src (composite)
            fa = world.restrict(f_src, gamma, a)
            if fa is None:
                failures.append(_gate_reject(
                    "stability_failure",
                    "GATE-3.1",
                    token_path=token_path,
                    context={"mask": f_src},
                    msg="restriction f* is undefined",
                ))
            else:
                gfa = world.restrict(g_src, f_src, fa)
                if fg is None or gfa is None or not world.equal(fg, gfa):
                    failures.append(_gate_reject(
                        "stability_failure",
                        "GATE-3.1",
                        token_path=token_path,
                        context={"mask": g_src},
                        msg="composition law failed: (f o g)* != g*(f*)",
                    ))

    elif kind == "descent":
        base = int(check["baseMask"])
        legs = [int(x) for x in check["legs"]]
        locals_ = check["locals"]
        glue_witness = check.get("glue", None)
        overlap_certified = bool(check.get("overlapCertified", False))
        cocycle_certified = bool(check.get("cocycleCertified", False))
        contractible_certified = bool(check.get("contractibleCertified", False))

        if any(not _mask_is_subset(l, base) for l in legs):
            raise ValueError("leg not subset of base")
        # locality: locals well-typed
        for (l, ai) in zip(legs, locals_):
            if not world.validate(l, ai):
                failures.append(_gate_reject(
                    "locality_failure",
                    "GATE-3.2",
                    token_path=token_path,
                    context={"mask": l},
                    msg="local definable ill-typed for leg context",
                ))
                break

        if not failures:
            # overlap compatibility (pairwise)
            if not overlap_certified:
                for i, j, ov in _overlaps(legs):
                    ri = world.restrict(ov, legs[i], locals_[i])
                    rj = world.restrict(ov, legs[j], locals_[j])
                    if ri is None or rj is None or not world.equal(ri, rj):
                        failures.append(_gate_reject(
                            "descent_failure",
                            "GATE-3.3",
                            token_path=token_path,
                            context={"mask": ov},
                            msg="overlap compatibility failed",
                        ))
                        break

        if not failures:
            # cocycle coherence (triple overlaps)
            if not cocycle_certified:
                for i, j, k, ov3 in _triple_overlaps(legs):
                    r1 = world.restrict(ov3, legs[i], locals_[i])
                    r2 = world.restrict(ov3, legs[j], locals_[j])
                    r3 = world.restrict(ov3, legs[k], locals_[k])
                    if r1 is None or r2 is None or r3 is None:
                        failures.append(_gate_reject(
                            "descent_failure",
                            "GATE-3.3",
                            token_path=token_path,
                            context={"mask": ov3},
                            msg="cocycle coherence failed: restriction undefined on triple overlap",
                        ))
                        break
                    if not (world.equal(r1, r2) and world.equal(r2, r3)):
                        failures.append(_gate_reject(
                            "descent_failure",
                            "GATE-3.3",
                            token_path=token_path,
                            context={"mask": ov3},
                            msg="cocycle coherence failed on triple overlap",
                        ))
                        break

        if not failures:
            if contractible_certified:
                # Proof-carrying path: contractibility is certified upstream.
                # We do not re-enumerate candidates here.
                if "glue" not in check:
                    failures.append(_gate_reject(
                        "descent_failure",
                        "GATE-3.3",
                        token_path=token_path,
                        context={"mask": base},
                        msg="contractibleCertified is set but no glue witness was provided",
                    ))
                elif glue_witness is None or not world.validate(base, glue_witness):
                    failures.append(_gate_reject(
                        "descent_failure",
                        "GATE-3.3",
                        token_path=token_path,
                        context={"mask": base},
                        msg="provided glue witness is ill-typed for the base context",
                    ))
                else:
                    # Existence sanity: glue witness must restrict to the claimed locals.
                    for (l, ai) in zip(legs, locals_):
                        r = world.restrict(l, base, glue_witness)
                        if r is None or not world.equal(r, ai):
                            failures.append(_gate_reject(
                                "descent_failure",
                                "GATE-3.3",
                                token_path=token_path,
                                context={"mask": l},
                                msg="provided glue witness does not restrict to the local data",
                            ))
                            break
            else:
                # Non-certified path: enumerate global candidates.
                cands = []
                for a in world.enumerate(base):
                    ok = True
                    for (l, ai) in zip(legs, locals_):
                        r = world.restrict(l, base, a)
                        if r is None or not world.equal(r, ai):
                            ok = False
                            break
                    if ok:
                        cands.append(a)

                if len(cands) == 0:
                    failures.append(_gate_reject(
                        "descent_failure",
                        "GATE-3.3",
                        token_path=token_path,
                        context={"mask": base},
                        msg="no global glue exists for compatible local data",
                    ))
                elif len(cands) > 1:
                    failures.append(_gate_reject(
                        "glue_non_contractible",
                        "GATE-3.4",
                        token_path=token_path,
                        context={"mask": base},
                        msg="multiple global glues exist for the same descent datum",
                    ))
                else:
                    # Contractible uniqueness case.
                    # If the caller provided a glue witness, sanity-check it.
                    if "glue" in check and not world.equal(glue_witness, cands[0]):
                        failures.append(_gate_reject(
                            "descent_failure",
                            "GATE-3.3",
                            token_path=token_path,
                            context={"mask": base},
                            msg="provided glue witness does not match the unique glued value",
                        ))

    else:
        raise ValueError(f"unknown check kind: {kind}")

    # Deterministic ordering: class, lawRef, tokenPath, context.mask, witnessId
    def _sort_key(f: Dict[str, Any]):
        ctx = f.get("context") or {}
        return (f.get("class",""), f.get("lawRef",""), f.get("tokenPath") or "", str(ctx.get("mask","")), f.get("witnessId",""))
    failures.sort(key=_sort_key)

    if failures:
        return {
            "witnessSchema": 1,
            "profile": "toy",
            "result": "rejected",
            "failures": failures,
        }
    return {
        "witnessSchema": 1,
        "profile": "toy",
        "result": "accepted",
        "failures": [],
    }


def main() -> None:
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("case_json", help="path to case.json")
    ap.add_argument("--out", default=None, help="write output json")
    args = ap.parse_args()

    case = json.load(open(args.case_json, "r", encoding="utf-8"))
    out = run_case(case)
    if args.out:
        json.dump(out, open(args.out, "w", encoding="utf-8"), indent=2, sort_keys=True)
    else:
        print(json.dumps(out, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
