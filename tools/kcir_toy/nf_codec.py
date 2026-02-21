"""Minimal ObjNF codec for the KCIR toy suite.

This is *tooling*.

We implement only the ObjNF fragments exercised by the KCIR toy suite:

- Prim (tag 0x02)
- Glue (tag 0x06)

The full grammar is in:
  specs/premath/draft/NF.md
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List, Tuple

from kcir_codec import enc_varint, dec_varint


@dataclass(frozen=True)
class ObjNF:
    tag: int
    payload: bytes


def parse_obj_nf(obj_bytes: bytes) -> ObjNF:
    if not obj_bytes:
        raise ValueError('empty ObjNF')
    tag = obj_bytes[0]
    payload = obj_bytes[1:]
    return ObjNF(tag=tag, payload=payload)


def build_obj_prim(prim_id: bytes) -> bytes:
    if len(prim_id) != 32:
        raise ValueError('primId must be 32 bytes')
    return bytes([0x02]) + prim_id


def prim_id_of(obj_bytes: bytes) -> bytes:
    n = parse_obj_nf(obj_bytes)
    if n.tag != 0x02 or len(n.payload) != 32:
        raise ValueError('not a Prim ObjNF')
    return n.payload


def _enc_list_b32(xs: List[bytes]) -> bytes:
    for x in xs:
        if len(x) != 32:
            raise ValueError('expected bytes32 list element')
    return enc_varint(len(xs)) + b''.join(xs)


def _dec_list_b32(buf: bytes, off: int) -> Tuple[List[bytes], int]:
    n, off2 = dec_varint(buf, off)
    need = n * 32
    if off2 + need > len(buf):
        raise ValueError('truncated bytes32 list')
    xs = [buf[off2 + 32*i: off2 + 32*(i+1)] for i in range(n)]
    return xs, off2 + need


def build_obj_glue(w_sig: bytes, locals_: List[bytes]) -> bytes:
    """ObjNF Glue(wSig, locals).

    Encoding (hash-profile toy):
      0x06 || wSig:bytes32 || encListB32(locals)
    """
    if len(w_sig) != 32:
        raise ValueError('wSig must be 32 bytes')
    return bytes([0x06]) + w_sig + _enc_list_b32(locals_)


def glue_fields(obj_bytes: bytes) -> Tuple[bytes, List[bytes]]:
    """Return (wSig, localsRefs) for an ObjNF Glue."""
    n = parse_obj_nf(obj_bytes)
    if n.tag != 0x06:
        raise ValueError('not a Glue ObjNF')
    if len(n.payload) < 32:
        raise ValueError('truncated Glue payload')
    w_sig = n.payload[:32]
    locals_, off = _dec_list_b32(n.payload, 32)
    if off != len(n.payload):
        raise ValueError('trailing bytes in Glue payload')
    return w_sig, locals_
