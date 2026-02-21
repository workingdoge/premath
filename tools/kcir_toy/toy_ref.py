"""Toy reference binding for KCIR/NF fixtures.

This is *tooling only* and is NOT a normative commitment backend.

The Premath kernel is commitment-backend agnostic (see `specs/premath/draft/REF-BINDING.md`):
a backend provides `project_ref` / `verify_ref` for `(domain, payload_bytes)`.

For the toy fixture suite we choose a single deterministic binder to make golden/adversarial
vectors reproducible across implementations:

- ObjNF: digest = SHA256("ObjNF" || envSig || uid || obj_nf_bytes)
- MorNF: digest = SHA256("MorNF" || envSig || uid || mor_nf_bytes)
- KCIR nodes: digest = SHA256("KCIRNode" || node_bytes)

We then encode references as raw 32-byte digests using the legacy fixed32 wire format.

If/when you swap to a Merkle, lattice (R_q), or other backend, ONLY this fixture-binding
module changes; the kernel specs and Gate obligations do not.
"""


from __future__ import annotations

import hashlib


def sha256(b: bytes) -> bytes:
    return hashlib.sha256(b).digest()


def h_obj(env_sig: bytes, uid: bytes, obj_bytes: bytes) -> bytes:
    if len(env_sig) != 32 or len(uid) != 32:
        raise ValueError('envSig/uid must be 32 bytes')
    return sha256(b"ObjNF" + env_sig + uid + obj_bytes)


def h_mor(env_sig: bytes, uid: bytes, mor_bytes: bytes) -> bytes:
    if len(env_sig) != 32 or len(uid) != 32:
        raise ValueError('envSig/uid must be 32 bytes')
    return sha256(b"MorNF" + env_sig + uid + mor_bytes)


def h_node(node_bytes: bytes) -> bytes:
    return sha256(b"KCIRNode" + node_bytes)
