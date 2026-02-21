"""
Toy witnessId implementation for Premath (draft/WITNESS-ID).

This is non-normative tooling, intended to generate and check toy vectors.
"""

from __future__ import annotations

import base64
import hashlib
import json
from typing import Any, Dict, Optional


def _jcs_dumps(obj: Any) -> bytes:
    """
    Minimal RFC 8785-compatible canonical JSON for the witness key domain we use:
    - dict keys sorted
    - no whitespace
    - integers only (no floats)
    - UTF-8
    """
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def _b32hex_lower_no_pad(data: bytes) -> str:
    """
    RFC 4648 base32hex encoding, lowercase and without padding.

    Python 3.10+ has base64.b32hexencode; older runtimes only provide base32.
    """
    if hasattr(base64, "b32hexencode"):
        encoded = base64.b32hexencode(data).decode("ascii")
    else:
        std = base64.b32encode(data).decode("ascii")
        # RFC 4648 alphabets:
        # base32:    A-Z 2-7
        # base32hex: 0-9 A-V
        trans = str.maketrans(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
            "0123456789ABCDEFGHIJKLMNOPQRSTUV",
        )
        encoded = std.translate(trans)
    return encoded.lower().rstrip("=")


def witness_id(
    *,
    cls: str,
    law_ref: str,
    token_path: Optional[str] = None,
    context: Optional[Dict[str, Any]] = None,
) -> str:
    key = {
        "schema": 1,
        "class": cls,
        "lawRef": law_ref,
        "tokenPath": token_path if token_path is not None else None,
        "context": context if context is not None else None,
    }
    key_bytes = _jcs_dumps(key)
    digest = hashlib.sha256(key_bytes).digest()
    b32 = _b32hex_lower_no_pad(digest)
    return "w1_" + b32
