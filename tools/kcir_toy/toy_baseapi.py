"""ToyViews Base API (non-normative reference implementation).

This implements the algorithms specified by:
  specs/premath/raw/BASEAPI-TOY-VIEWS.md

All functions are deterministic.

Encoding conventions:
- mapId: 32 bytes. bytes[0:4]=srcMask u32le, bytes[4:8]=tgtMask u32le, bytes[8:32]=0.
- coverSig: 32-byte SHA256("ToyCover"||coverPayloadBytes)

Cover payload bytes:
- baseMask:u32le || nLegs:u32le || legMask[0]:u32le || ... || legMask[n-1]:u32le

"""

from __future__ import annotations

import hashlib
import struct
from dataclasses import dataclass
from typing import Dict, List, Tuple


def u32le(x: int) -> bytes:
    return struct.pack('<I', x & 0xFFFFFFFF)


def decode_u32le(b: bytes) -> int:
    if len(b) != 4:
        raise ValueError('expected 4 bytes')
    return struct.unpack('<I', b)[0]


# --------------------
# Maps
# --------------------

def encode_map_id(src_mask: int, tgt_mask: int) -> bytes:
    if src_mask & ~tgt_mask:
        raise ValueError('mapId encode: src not subset of tgt')
    return u32le(src_mask) + u32le(tgt_mask) + b"\x00" * 24


def decode_map_id(map_id: bytes) -> Tuple[int, int]:
    if len(map_id) != 32:
        raise ValueError('mapId must be 32 bytes')
    if any(map_id[i] != 0 for i in range(8, 32)):
        raise ValueError('non-canonical mapId: trailing bytes must be zero')
    src = decode_u32le(map_id[0:4])
    tgt = decode_u32le(map_id[4:8])
    if src & ~tgt:
        raise ValueError('invalid mapId: src not subset of tgt')
    return src, tgt


def is_id_map(map_id: bytes) -> bool:
    src, tgt = decode_map_id(map_id)
    return src == tgt


def compose_maps(outer: bytes, inner: bytes) -> bytes:
    # outer ∘ inner
    a, b1 = decode_map_id(inner)
    b2, c = decode_map_id(outer)
    if b1 != b2:
        raise ValueError('composeMaps: tgt(inner) != src(outer)')
    return encode_map_id(a, c)


# --------------------
# Covers
# --------------------

@dataclass(frozen=True)
class CoverData:
    baseMask: int
    legs: List[int]  # canonical: sorted, nonzero, subset, covers base


def normalize_cover(base_mask: int, legs: List[int]) -> CoverData:
    # Enforce canonicalization rules from spec.
    legs2 = [int(x) for x in legs]
    if any(x == 0 for x in legs2):
        raise ValueError('cover leg must be non-zero')
    if any(x & ~base_mask for x in legs2):
        raise ValueError('cover leg must be subset of baseMask')
    legs2_sorted = sorted(legs2)
    if len(set(legs2_sorted)) != len(legs2_sorted):
        raise ValueError('duplicate cover legs')
    # Valid cover iff OR == base
    acc = 0
    for x in legs2_sorted:
        acc |= x
    if acc != base_mask:
        raise ValueError('cover legs do not cover baseMask')
    return CoverData(baseMask=base_mask, legs=legs2_sorted)


def cover_payload_bytes(cd: CoverData) -> bytes:
    b = bytearray()
    b += u32le(cd.baseMask)
    b += u32le(len(cd.legs))
    for lm in cd.legs:
        b += u32le(lm)
    return bytes(b)


def cover_sig(cd: CoverData) -> bytes:
    payload = cover_payload_bytes(cd)
    return hashlib.sha256(b"ToyCover" + payload).digest()


def validate_cover(sig: bytes, cd: CoverData) -> bool:
    try:
        cd2 = normalize_cover(cd.baseMask, cd.legs)
    except Exception:
        return False
    return sig == cover_sig(cd2)


def cover_len(cd: CoverData) -> int:
    return len(cd.legs)


# --------------------
# pullCover
# --------------------

def pull_cover(p_id: bytes, u_sig: bytes, cover_store: Dict[bytes, CoverData]) -> Tuple[bytes, List[int], List[bytes]]:
    """Compute pullCover(pId, uSig) per the ToyViews spec.

    Returns:
      - wSig
      - mapWtoU: list[int] mapping W-leg index -> U-leg index
      - projIds: list[bytes32] inclusion W_k -> U_i
    """
    w_mask, u_mask = decode_map_id(p_id)
    if u_sig not in cover_store:
        raise ValueError('unknown coverSig')
    u_cd = cover_store[u_sig]
    if u_cd.baseMask != u_mask:
        raise ValueError('cover baseMask does not match map tgt')

    pulled_legs: List[int] = []
    map_w_to_u: List[int] = []
    proj_ids: List[bytes] = []

    for i, u_leg in enumerate(u_cd.legs):
        w_leg = w_mask & u_leg
        if w_leg == 0:
            continue
        pulled_legs.append(w_leg)
        map_w_to_u.append(i)
        proj_ids.append(encode_map_id(w_leg, u_leg))

    w_cd = normalize_cover(w_mask, pulled_legs) if w_mask != 0 else CoverData(baseMask=0, legs=[])
    # Special case: empty base context has the empty cover.
    if w_mask == 0:
        # empty cover is valid: OR of empty list is 0
        w_cd = CoverData(baseMask=0, legs=[])

    w_sig = cover_sig(w_cd)
    cover_store[w_sig] = w_cd
    return w_sig, map_w_to_u, proj_ids


# --------------------
# Beck–Chevalley square
# --------------------

def bc_allowed(p_id: bytes, f_id: bytes) -> bool:
    try:
        _, p_tgt = decode_map_id(p_id)
        _, f_tgt = decode_map_id(f_id)
        return p_tgt == f_tgt
    except Exception:
        return False


def bc_square(push_id: bytes, pull_id: bytes) -> Tuple[bytes, bytes]:
    a, b1 = decode_map_id(push_id)
    c, b2 = decode_map_id(pull_id)
    if b1 != b2:
        raise ValueError('bcSquare: tgt mismatch')
    d = a & c
    f_prime = encode_map_id(d, c)  # D -> C
    p_prime = encode_map_id(d, a)  # D -> A
    return f_prime, p_prime
