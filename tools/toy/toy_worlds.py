"""
Toy Def-assignments ("worlds") for semantic Gate vectors.

Each world defines:
- validate(mask, a): bool
- restrict(src_mask, tgt_mask, a): Optional[a']  where (src -> tgt) is inclusion
- equal(a, b): bool
- enumerate(mask): list[a]  (only used for tiny masks)

KCIR toy fixtures may additionally carry proof-carrying Gate witnesses.
Those witnesses are verified via a *scheme id* plus an opaque byte payload.
The default toy scheme still uses enumeration internally, but the interface is
deliberately shaped so alternative backends (e.g. lattice/linear proofs) can be
plugged in without changing the kernel.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple

# Tooling-only: make the repo root importable so we can share scheme ids.
import os
import sys

_THIS_DIR = os.path.dirname(__file__)
_REPO_ROOT = os.path.abspath(os.path.join(_THIS_DIR, '..', '..'))
if _REPO_ROOT not in sys.path:
    sys.path.append(_REPO_ROOT)

from tools.proof_schemes import SCHEME_TOY_ENUMERATE_V1  # type: ignore


def _bits(mask: int) -> List[int]:
    return [i for i in range(32) if (mask >> i) & 1]


@dataclass(frozen=True)
class World:
    name: str

    def validate(self, mask: int, a: Any) -> bool:
        raise NotImplementedError

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        """
        Restrict along inclusion src -> tgt, where src ⊆ tgt.
        Returns None if restriction is undefined in this world (locality failure).
        """
        raise NotImplementedError

    def equal(self, a: Any, b: Any) -> bool:
        return a == b

    def enumerate(self, mask: int) -> List[Any]:
        raise NotImplementedError

    def verify_contractible(
        self,
        scheme_id: bytes,
        proof: bytes,
        *,
        base_mask: int,
        legs: List[int],
        locals_vals: List[Any],
    ) -> bool:
        """Verify a contractible-gluing proof for a descent datum.

        Tooling contract:
        - `scheme_id` is a Bytes32 label.
        - `proof` is opaque bytes interpreted by that scheme.
        - `base_mask`, `legs`, and `locals_vals` are the public inputs.

        The default toy scheme (`SCHEME_TOY_ENUMERATE_V1`) requires `proof` to
        be empty and checks contractibility by enumerating candidates.
        """

        if scheme_id != SCHEME_TOY_ENUMERATE_V1:
            return False
        # Determinism rule for the toy scheme: proof bytes must be empty.
        if proof != b"":
            return False

        found = None
        for cand in self.enumerate(base_mask):
            ok = True
            for (leg_mask, lv) in zip(legs, locals_vals):
                r = self.restrict(leg_mask, base_mask, cand)
                if r is None or not self.equal(r, lv):
                    ok = False
                    break
            if ok:
                if found is None:
                    found = cand
                else:
                    return False
        return found is not None


class SheafBits(World):
    """
    Def(mask) = all total functions bits(mask)->{0,1}.
    Representation: dict[int,str/int]? We use dict[str,int] in JSON, and convert to dict[int,int].
    """

    def __init__(self):
        super().__init__("sheaf_bits")

    def validate(self, mask: int, a: Any) -> bool:
        if not isinstance(a, dict):
            return False
        bs = _bits(mask)
        # a must have exactly keys for all bits in mask
        try:
            keys = sorted(int(k) for k in a.keys())
        except Exception:
            return False
        if keys != bs:
            return False
        # values must be 0/1
        return all(a[str(i)] in (0, 1) for i in bs)

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        if not self.validate(tgt_mask, a):
            return None
        bs = _bits(src_mask)
        return {str(i): int(a[str(i)]) for i in bs}

    def enumerate(self, mask: int) -> List[Any]:
        bs = _bits(mask)
        out: List[Any] = []
        n = len(bs)
        for x in range(1 << n):
            d = {}
            for j, bit in enumerate(bs):
                d[str(bit)] = (x >> j) & 1
            out.append(d)
        return out


class BadConstant(World):
    """
    Def(mask!=0) = {0,1}, Def(0) = {"*"}.
    Restriction:
      - to empty: both 0 and 1 map to None
      - to non-empty: identity
    """

    def __init__(self):
        super().__init__("bad_constant")

    def validate(self, mask: int, a: Any) -> bool:
        if mask == 0:
            return a == "*"
        return a in (0, 1)

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        if not self.validate(tgt_mask, a):
            return None
        if src_mask == 0:
            return "*"
        # src non-empty
        if tgt_mask == 0:
            # should not happen: cannot include non-empty into empty
            return None
        return a

    def enumerate(self, mask: int) -> List[Any]:
        if mask == 0:
            return ["*"]
        return [0, 1]


class NonSeparated(World):
    """
    Def(mask!=0) = {0,1}, Def(0) = {"*"}.
    Restriction:
      - to empty: -> None
      - to non-empty: constant map to 0
    This violates uniqueness (non-contractible glue space).
    """

    def __init__(self):
        super().__init__("non_separated")

    def validate(self, mask: int, a: Any) -> bool:
        if mask == 0:
            return a == "*"
        return a in (0, 1)

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        if not self.validate(tgt_mask, a):
            return None
        if src_mask == 0:
            return "*"
        return 0

    def enumerate(self, mask: int) -> List[Any]:
        if mask == 0:
            return ["*"]
        return [0, 1]


class BadStability(World):
    """
    Like BadConstant, but with a non-functorial override:
    there exists a specific tgt->src restriction that disagrees with composition.
    We implement this by making restriction to src_mask==1 from tgt_mask==7 always return 0.
    (Even if restricting via an intermediate yields 1.)
    """

    def __init__(self):
        super().__init__("bad_stability")

    def validate(self, mask: int, a: Any) -> bool:
        if mask == 0:
            return a is None
        return a in (0, 1)

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        if not self.validate(tgt_mask, a):
            return None
        if src_mask == 0:
            return None
        # non-functorial special case
        if tgt_mask == 7 and src_mask == 1:
            return 0
        # otherwise behaves like BadConstant identity-on-nonempty
        return a

    def enumerate(self, mask: int) -> List[Any]:
        if mask == 0:
            return [None]
        return [0, 1]


class PartialRestrict(World):
    """
    Like BadConstant, but restriction to any singleton context is undefined.
    This targets locality_failure.
    """

    def __init__(self):
        super().__init__("partial_restrict")

    def validate(self, mask: int, a: Any) -> bool:
        if mask == 0:
            return a is None
        return a in (0, 1)

    def restrict(self, src_mask: int, tgt_mask: int, a: Any) -> Optional[Any]:
        if not self.validate(tgt_mask, a):
            return None
        if src_mask == 0:
            return None
        # undefined if src has exactly one bit set
        if src_mask != 0 and (src_mask & (src_mask - 1)) == 0:
            return None
        return a

    def enumerate(self, mask: int) -> List[Any]:
        if mask == 0:
            return [None]
        return [0, 1]


def get_world(name: str) -> World:
    if name == "sheaf_bits":
        return SheafBits()
    if name == "bad_constant":
        return BadConstant()
    if name == "non_separated":
        return NonSeparated()
    if name == "bad_stability":
        return BadStability()
    if name == "partial_restrict":
        return PartialRestrict()
    raise ValueError(f"unknown world: {name}")
