"""Fixture store loader for the KCIR toy suite."""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from typing import Dict, Optional


def _read_bin(path: str) -> bytes:
    return open(path, 'rb').read()


def _read_json(path: str):
    return json.load(open(path, 'r', encoding='utf-8'))


def _hex_to_bytes32(h: str) -> bytes:
    b = bytes.fromhex(h)
    if len(b) != 32:
        raise ValueError('expected 32-byte hex')
    return b


def _bytes32_to_hex(b: bytes) -> str:
    if len(b) != 32:
        raise ValueError('expected 32 bytes')
    return b.hex()


@dataclass
class FixtureStore:
    root: Optional[bytes]
    certs: Dict[bytes, bytes]
    obj: Dict[bytes, bytes]
    covers: Dict[bytes, dict]
    prims: Dict[bytes, dict]

    @staticmethod
    def load(fixture_dir: str) -> 'FixtureStore':
        # root
        root_path = os.path.join(fixture_dir, 'root.txt')
        root = None
        if os.path.exists(root_path):
            root_hex = open(root_path, 'r', encoding='utf-8').read().strip()
            if root_hex:
                root = _hex_to_bytes32(root_hex)

        certs: Dict[bytes, bytes] = {}
        cert_dir = os.path.join(fixture_dir, 'certs')
        if os.path.isdir(cert_dir):
            for fn in os.listdir(cert_dir):
                if not fn.endswith('.bin'):
                    continue
                h = fn[:-4]
                certs[_hex_to_bytes32(h)] = _read_bin(os.path.join(cert_dir, fn))

        obj: Dict[bytes, bytes] = {}
        obj_dir = os.path.join(fixture_dir, 'obj')
        if os.path.isdir(obj_dir):
            for fn in os.listdir(obj_dir):
                if not fn.endswith('.bin'):
                    continue
                h = fn[:-4]
                obj[_hex_to_bytes32(h)] = _read_bin(os.path.join(obj_dir, fn))

        covers: Dict[bytes, dict] = {}
        cov_dir = os.path.join(fixture_dir, 'covers')
        if os.path.isdir(cov_dir):
            for fn in os.listdir(cov_dir):
                if not fn.endswith('.json'):
                    continue
                h = fn[:-5]
                covers[_hex_to_bytes32(h)] = _read_json(os.path.join(cov_dir, fn))

        prims: Dict[bytes, dict] = {}
        prim_dir = os.path.join(fixture_dir, 'prims')
        if os.path.isdir(prim_dir):
            for fn in os.listdir(prim_dir):
                if not fn.endswith('.json'):
                    continue
                h = fn[:-5]
                prims[_hex_to_bytes32(h)] = _read_json(os.path.join(prim_dir, fn))

        return FixtureStore(root=root, certs=certs, obj=obj, covers=covers, prims=prims)
