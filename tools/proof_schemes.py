"""Proof scheme identifiers used by non-normative tooling.

This file is intentionally *not* part of the normative kernel. It exists to:

1) give the toy fixture generator a stable, deterministic scheme id to embed in
   witness nodes, and
2) let the toy verifier select a proof-checking algorithm.

The kernel-facing contract is simply: "schemeId is an opaque Bytes32 label".
Nothing here commits Premath to any cryptographic backend or proof system.
"""

from __future__ import annotations

import hashlib


def scheme_id(name: str) -> bytes:
    """Derive a stable Bytes32 scheme identifier from an ASCII/UTF-8 name.

    Tooling-only convention.
    """

    return hashlib.sha256(name.encode("utf-8")).digest()


# The toy suite's default contractibility proof scheme.
#
# Semantics: "the verifier may enumerate globals and check uniqueness".
# Proof bytes are required to be empty for determinism.
SCHEME_TOY_ENUMERATE_V1: bytes = scheme_id("toy.enumerate.v1")
