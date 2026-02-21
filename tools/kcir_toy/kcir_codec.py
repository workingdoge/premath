"""KCIR legacy fixed32 wire codec (tooling).

Matches specs:
- specs/premath/draft/KCIR-CORE.md (legacy fixed32 transitional)
- specs/premath/draft/WIRE-FORMATS.md (kcir.wire.legacy-fixed32.v1)

We treat refs as raw 32-byte digests (Bytes32).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List, Tuple


def enc_varint(n: int) -> bytes:
    """Unsigned LEB128."""
    if n < 0:
        raise ValueError('varint must be non-negative')
    out = bytearray()
    x = n
    while True:
        b = x & 0x7F
        x >>= 7
        if x:
            out.append(b | 0x80)
        else:
            out.append(b)
            break
    return bytes(out)


def dec_varint(buf: bytes, off: int) -> Tuple[int, int]:
    """Return (value, new_off)."""
    x = 0
    shift = 0
    i = off
    while True:
        if i >= len(buf):
            raise ValueError('truncated varint')
        b = buf[i]
        i += 1
        x |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            return x, i
        shift += 7
        if shift > 63:
            raise ValueError('varint too large')


@dataclass
class KCIRNode:
    env_sig: bytes
    uid: bytes
    sort: int
    opcode: int
    out: bytes  # Bytes32 digest
    args: bytes
    deps: List[bytes]  # list[Bytes32]


def encode_node(n: KCIRNode) -> bytes:
    if len(n.env_sig) != 32 or len(n.uid) != 32:
        raise ValueError('envSig and uid must be 32 bytes')
    if len(n.out) != 32:
        raise ValueError('out must be 32 bytes')
    if not (0 <= n.sort <= 255 and 0 <= n.opcode <= 255):
        raise ValueError('sort/opcode must be bytes')
    for d in n.deps:
        if len(d) != 32:
            raise ValueError('dep must be 32 bytes')

    b = bytearray()
    b += n.env_sig
    b += n.uid
    b.append(n.sort)
    b.append(n.opcode)
    b += n.out
    b += enc_varint(len(n.args))
    b += n.args
    b += enc_varint(len(n.deps))
    for d in n.deps:
        b += d
    return bytes(b)


def decode_node(buf: bytes) -> KCIRNode:
    if len(buf) < 32 + 32 + 1 + 1 + 32:
        raise ValueError('truncated node')
    off = 0
    env_sig = buf[off:off+32]; off += 32
    uid = buf[off:off+32]; off += 32
    sort = buf[off]; off += 1
    opcode = buf[off]; off += 1
    out = buf[off:off+32]; off += 32
    args_len, off = dec_varint(buf, off)
    if off + args_len > len(buf):
        raise ValueError('truncated args')
    args = buf[off:off+args_len]; off += args_len
    deps_count, off = dec_varint(buf, off)
    deps: List[bytes] = []
    if off + deps_count * 32 != len(buf):
        raise ValueError('deps length mismatch or trailing bytes')
    for _ in range(deps_count):
        deps.append(buf[off:off+32])
        off += 32
    return KCIRNode(env_sig=env_sig, uid=uid, sort=sort, opcode=opcode, out=out, args=args, deps=deps)
