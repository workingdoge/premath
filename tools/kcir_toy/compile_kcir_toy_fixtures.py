"""Compile semantic toy Gate fixtures to KCIR-shaped toy fixtures.

Input fixtures (semantic):
  tests/toy/fixtures/<caseId>/{case.json,expect.json}

Output fixtures (KCIR-shaped):
  tests/kcir_toy/fixtures/<caseId>/
    root.txt
    case.json          (copied)
    expect.json        (copied)
    certs/*.bin        (KCIR nodes)
    obj/*.bin          (ObjNF bytes)
    prims/*.json       (primId -> {mask,value})
    covers/*.json      (coverSig -> {baseMask,legs})
    gate/check.json    (compiled gate check referencing KCIR objects)

Non-normative tooling.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
from dataclasses import dataclass
from typing import Any, Dict, List, Tuple

from kcir_codec import KCIRNode, encode_node
from toy_baseapi import encode_map_id, normalize_cover, cover_sig, CoverData
from toy_ref import h_node, h_obj
from nf_codec import build_obj_prim, build_obj_glue

# Tooling-only: make repo root importable so we can share proof scheme ids.
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if _REPO_ROOT not in sys.path:
    sys.path.append(_REPO_ROOT)

from tools.proof_schemes import SCHEME_TOY_ENUMERATE_V1  # type: ignore


def _u32le(x: int) -> bytes:
    return (x & 0xFFFFFFFF).to_bytes(4, 'little')


def _sha256(b: bytes) -> bytes:
    return hashlib.sha256(b).digest()


def _canon_json_bytes(v: Any) -> bytes:
    return json.dumps(v, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')


def prim_id(mask: int, value: Any) -> bytes:
    payload = _u32le(mask) + _canon_json_bytes(value)
    return _sha256(b"ToyPrim" + payload)


@dataclass
class Builder:
    env_sig: bytes
    uid: bytes
    out_dir: str

    # stores
    certs: Dict[bytes, bytes]
    obj: Dict[bytes, bytes]
    prims: Dict[bytes, dict]
    covers: Dict[bytes, dict]

    def __init__(self, out_dir: str, env_sig: bytes, uid: bytes):
        self.env_sig = env_sig
        self.uid = uid
        self.out_dir = out_dir
        self.certs = {}
        self.obj = {}
        self.prims = {}
        self.covers = {}

    def add_prim_obj(self, mask: int, value: Any) -> Tuple[bytes, bytes, bytes]:
        """Return (objRefDigest, primId, nodeRefDigest)."""
        pid = prim_id(mask, value)
        if pid not in self.prims:
            self.prims[pid] = {'mask': mask, 'value': value}
        obj_bytes = build_obj_prim(pid)
        obj_ref = h_obj(self.env_sig, self.uid, obj_bytes)
        self.obj[obj_ref] = obj_bytes
        # add an O_PRIM node so the object is constructible.
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x03,
            opcode=0x02,
            out=obj_ref,
            args=pid,
            deps=[],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return obj_ref, pid, nr

    def add_map(self, src: int, tgt: int) -> Tuple[bytes, bytes]:
        mid = encode_map_id(src, tgt)
        # M_LITERAL node
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x02,
            opcode=0x01,
            out=mid,
            args=mid,
            deps=[],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return mid, nr

    def add_cover(self, base: int, legs: List[int]) -> Tuple[bytes, bytes, CoverData]:
        cd = normalize_cover(base, legs)
        sig = cover_sig(cd)
        self.covers[sig] = {'baseMask': cd.baseMask, 'legs': cd.legs}
        # C_LITERAL node
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x01,
            opcode=0x01,
            out=sig,
            args=sig,
            deps=[],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return sig, nr, cd

    def add_glue_obj(
        self,
        *,
        cover_sig: bytes,
        local_obj_refs: List[bytes],
        cover_node_ref: bytes,
        local_node_refs: List[bytes],
    ) -> Tuple[bytes, bytes]:
        """Return (glueObjRefDigest, nodeRefDigest) for OBJ/O_MKGLUE.

        This is the first "proof-carrying descent trace" seam in the KCIR toy suite.
        The node depends on:
          - the cover literal node, and
          - one OBJ node per local.

        Semantic correctness (overlap compatibility + contractible uniqueness) is still
        checked by the Gate layer; this node is an *existence witness* candidate.
        """
        if len(local_obj_refs) != len(local_node_refs):
            raise ValueError('local_obj_refs and local_node_refs must have same length')

        obj_bytes = build_obj_glue(cover_sig, local_obj_refs)
        obj_ref = h_obj(self.env_sig, self.uid, obj_bytes)
        self.obj[obj_ref] = obj_bytes

        # Minimal encoding: args = wSig only; locals are carried by deps.
        args = cover_sig
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x03,
            opcode=0x04,  # O_MKGLUE
            out=obj_ref,
            args=args,
            deps=[cover_node_ref] + list(local_node_refs),
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return obj_ref, nr

    def unit_obj(self) -> bytes:
        """Return the ObjNF reference for Unit, and ensure bytes are present in the obj store."""
        unit_bytes = bytes([0x01])
        unit_ref = h_obj(self.env_sig, self.uid, unit_bytes)
        if unit_ref not in self.obj:
            self.obj[unit_ref] = unit_bytes
        return unit_ref

    def add_assert_overlap(self, *, ov_mask: int, left_node: bytes, right_node: bytes) -> bytes:
        """Add a toy Gate witness node that certifies pairwise overlap compatibility.

        Node:
          sort=OBJ (0x03)
          opcode=O_ASSERT_OVERLAP (0x05) [toy extension]
          args = ovMask:u32le
          deps = [left_node, right_node]
          out  = Unit

        The verifier contract checks that the two locals agree on the overlap mask.
        """
        unit_ref = self.unit_obj()
        args = _u32le(ov_mask)
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x03,
            opcode=0x05,
            out=unit_ref,
            args=args,
            deps=[left_node, right_node],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return nr

    def add_assert_triple(self, *, tri_mask: int, a_node: bytes, b_node: bytes, c_node: bytes) -> bytes:
        """Add a toy Gate witness node that certifies triple-overlap (cocycle) coherence.

        Node:
          sort=OBJ (0x03)
          opcode=O_ASSERT_TRIPLE (0x06) [toy extension]
          args = triMask:u32le
          deps = [a_node, b_node, c_node]
          out  = Unit
        """
        unit_ref = self.unit_obj()
        args = _u32le(tri_mask)
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x03,
            opcode=0x06,
            out=unit_ref,
            args=args,
            deps=[a_node, b_node, c_node],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return nr

    def add_assert_contractible(self, *, glue_node: bytes) -> bytes:
        """Add a toy Gate witness node that certifies contractible gluing.

        Node:
          sort=OBJ (0x03)
          opcode=O_ASSERT_CONTRACTIBLE (0x07) [toy extension]
          args = schemeId:Bytes32 || proofBytes
          deps = [glue_node]
          out  = Unit

        The verifier contract checks that the space of global candidates for the glue
        datum is contractible (exactly one glued value exists).
        """
        unit_ref = self.unit_obj()
        # Tooling default: enumeration-based contractibility checking.
        # Determinism rule: proofBytes must be empty.
        args = SCHEME_TOY_ENUMERATE_V1 + b''
        n = KCIRNode(
            env_sig=self.env_sig,
            uid=self.uid,
            sort=0x03,
            opcode=0x07,
            out=unit_ref,
            args=args,
            deps=[glue_node],
        )
        nb = encode_node(n)
        nr = h_node(nb)
        self.certs[nr] = nb
        return nr

    def write(self) -> None:
        os.makedirs(self.out_dir, exist_ok=True)
        for sub in ('certs', 'obj', 'prims', 'covers', 'gate'):
            os.makedirs(os.path.join(self.out_dir, sub), exist_ok=True)

        # certs
        for r, nb in self.certs.items():
            open(os.path.join(self.out_dir, 'certs', r.hex() + '.bin'), 'wb').write(nb)

        # obj
        for r, ob in self.obj.items():
            open(os.path.join(self.out_dir, 'obj', r.hex() + '.bin'), 'wb').write(ob)

        # prims
        for pid, ent in self.prims.items():
            open(os.path.join(self.out_dir, 'prims', pid.hex() + '.json'), 'w', encoding='utf-8').write(
                json.dumps(ent, indent=2, sort_keys=True)
            )

        # covers
        for cs, ent in self.covers.items():
            open(os.path.join(self.out_dir, 'covers', cs.hex() + '.json'), 'w', encoding='utf-8').write(
                json.dumps(ent, indent=2, sort_keys=True)
            )


def compile_one(case_dir: str, out_dir: str, env_sig: bytes, uid: bytes) -> None:
    case = json.load(open(os.path.join(case_dir, 'case.json'), 'r', encoding='utf-8'))

    # Read expected outcome (if present) so we can optionally emit proof-carrying witnesses
    # only for accepted cases.
    exp_path = os.path.join(case_dir, 'expect.json')
    exp_result = None
    if os.path.exists(exp_path):
        exp = json.load(open(exp_path, 'r', encoding='utf-8'))
        exp_result = exp.get('result', None)

    world = case['world']
    chk = case['check']
    kind = chk['kind']

    b = Builder(out_dir=out_dir, env_sig=env_sig, uid=uid)

    gate_check: Dict[str, Any] = {'schema': 1, 'world': world, 'check': {'kind': kind, 'tokenPath': chk.get('tokenPath', None)}}

    root_node_ref: bytes | None = None

    if kind == 'stability':
        gamma = int(chk['gammaMask'])
        a_val = chk['a']
        a_obj, _, a_node = b.add_prim_obj(gamma, a_val)
        f = chk['f']; g = chk['g']
        # Semantic toy cases use keys `src`/`tgt`.
        f_id, _ = b.add_map(int(f['src']), int(f['tgt']))
        g_id, _ = b.add_map(int(g['src']), int(g['tgt']))
        gate_check['check'].update({
            'gammaMask': gamma,
            'aObj': a_obj.hex(),
            'fMap': f_id.hex(),
            'gMap': g_id.hex(),
        })
        root_node_ref = a_node

    elif kind == 'locality':
        gamma = int(chk['gammaMask'])
        a_val = chk['a']
        a_obj, _, _ = b.add_prim_obj(gamma, a_val)
        legs = [int(x) for x in chk['legs']]
        cover_sig_b, cover_node, cd = b.add_cover(gamma, legs)
        gate_check['check'].update({
            'gammaMask': gamma,
            'aObj': a_obj.hex(),
            'coverSig': cover_sig_b.hex(),
        })
        root_node_ref = cover_node

    elif kind == 'descent':
        base = int(chk['baseMask'])
        legs_in = [int(x) for x in chk['legs']]
        locals_vals = chk['locals']
        if len(locals_vals) != len(legs_in):
            raise ValueError('locals length must match legs length')

        # Normalize cover order and reorder locals accordingly.
        cd_norm = normalize_cover(base, legs_in)
        leg_to_val = {lm: v for (lm, v) in zip(legs_in, locals_vals)}
        legs = list(cd_norm.legs)
        locals_norm = [leg_to_val[lm] for lm in legs]

        cover_sig_b, cover_node, _ = b.add_cover(base, legs)
        local_obj_refs: List[bytes] = []
        local_node_refs: List[bytes] = []
        for lm, v in zip(legs, locals_norm):
            oref, _, onode = b.add_prim_obj(lm, v)
            local_obj_refs.append(oref)
            local_node_refs.append(onode)

        # Record local node refs for gate tooling.
        local_nodes_hex = [n.hex() for n in local_node_refs]

        # Pairwise overlap-compatibility witnesses.
        overlap_node_refs: List[bytes] = []
        for i in range(len(legs)):
            for j in range(i + 1, len(legs)):
                ov = legs[i] & legs[j]
                overlap_node_refs.append(b.add_assert_overlap(
                    ov_mask=ov,
                    left_node=local_node_refs[i],
                    right_node=local_node_refs[j],
                ))

        # Triple-overlap (cocycle) witnesses.
        triple_node_refs: List[bytes] = []
        for i in range(len(legs)):
            for j in range(i + 1, len(legs)):
                for k in range(j + 1, len(legs)):
                    tri = legs[i] & legs[j] & legs[k]
                    triple_node_refs.append(b.add_assert_triple(
                        tri_mask=tri,
                        a_node=local_node_refs[i],
                        b_node=local_node_refs[j],
                        c_node=local_node_refs[k],
                    ))

        # Proof-carrying descent trace: build an explicit glue candidate object.
        glue_obj, glue_node = b.add_glue_obj(
            cover_sig=cover_sig_b,
            local_obj_refs=local_obj_refs,
            cover_node_ref=cover_node,
            local_node_refs=local_node_refs,
        )

        contractible_node: bytes | None = None
        if exp_result == 'accepted':
            # Proof-carrying contractible gluing witness.
            contractible_node = b.add_assert_contractible(glue_node=glue_node)

        local_objs = [o.hex() for o in local_obj_refs]
        gate_check['check'].update({
            'baseMask': base,
            'coverSig': cover_sig_b.hex(),
            'localObjs': local_objs,
            'localNodes': local_nodes_hex,
            'overlapNodes': [r.hex() for r in overlap_node_refs],
            'tripleNodes': [r.hex() for r in triple_node_refs],
            'glueObj': glue_obj.hex(),
            **({'contractibleNode': contractible_node.hex()} if contractible_node is not None else {}),
        })
        root_node_ref = contractible_node or glue_node

    else:
        raise ValueError(f'unknown kind: {kind}')

    # Write store
    b.write()

    # Write gate/check.json
    open(os.path.join(out_dir, 'gate', 'check.json'), 'w', encoding='utf-8').write(
        json.dumps(gate_check, indent=2, sort_keys=True)
    )

    # root.txt
    if root_node_ref is None:
        # fallback: first cert
        root_node_ref = sorted(b.certs.keys(), key=lambda x: x.hex())[0]
    open(os.path.join(out_dir, 'root.txt'), 'w', encoding='utf-8').write(root_node_ref.hex() + '\n')

    # Copy case/expect as reference
    shutil.copy2(os.path.join(case_dir, 'case.json'), os.path.join(out_dir, 'case.json'))
    exp_src = os.path.join(case_dir, 'expect.json')
    if os.path.exists(exp_src):
        shutil.copy2(exp_src, os.path.join(out_dir, 'expect.json'))


def main() -> None:
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument('--in', dest='in_dir', default='tests/toy/fixtures')
    ap.add_argument('--out', dest='out_dir', default='tests/kcir_toy/fixtures')
    args = ap.parse_args()

    env_sig = b'\x11' * 32
    uid = b'\x22' * 32

    os.makedirs(args.out_dir, exist_ok=True)

    case_ids = sorted([d for d in os.listdir(args.in_dir) if os.path.isdir(os.path.join(args.in_dir, d))])

    for cid in case_ids:
        src = os.path.join(args.in_dir, cid)
        dst = os.path.join(args.out_dir, cid)
        if os.path.exists(dst):
            shutil.rmtree(dst)
        os.makedirs(dst, exist_ok=True)
        compile_one(src, dst, env_sig=env_sig, uid=uid)

    print(f'[compile] wrote {len(case_ids)} fixtures to {args.out_dir}')


if __name__ == '__main__':
    main()
