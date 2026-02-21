"""Minimal KCIR verifier for the KCIR toy suite.

This verifies only a small subset of the kernel stack, enough for the toy gate suite:

- KCIR node decode (legacy fixed32)
- global invariants: envSig/uid consistent, no cycles
- opcode contracts (subset):
  - COVER/C_LITERAL (0x01/0x01)
  - MAP/M_LITERAL   (0x02/0x01)
  - OBJ/O_UNIT      (0x03/0x01)
  - OBJ/O_PRIM      (0x03/0x02)
  - OBJ/O_MKGLUE    (0x03/0x04)
  - OBJ/O_ASSERT_OVERLAP (0x03/0x05) [toy gate witness]
  - OBJ/O_ASSERT_TRIPLE  (0x03/0x06) [toy gate witness]
  - OBJ/O_ASSERT_CONTRACTIBLE (0x03/0x07) [toy gate witness]

It is non-normative tooling.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Optional, Set, Tuple

import os
import sys

from kcir_codec import KCIRNode, decode_node
from nf_codec import build_obj_prim, build_obj_glue, parse_obj_nf, prim_id_of
from toy_baseapi import CoverData, validate_cover, decode_map_id, cover_len
from toy_ref import h_node, h_obj

# Tooling-only: make repo root importable so we can share proof scheme ids.
_REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
if _REPO_ROOT not in sys.path:
    sys.path.append(_REPO_ROOT)

from tools.proof_schemes import SCHEME_TOY_ENUMERATE_V1  # type: ignore

# Import the semantic toy worlds.
_THIS_DIR = os.path.dirname(__file__)
sys.path.append(os.path.join(_THIS_DIR, '..', 'toy'))
from toy_worlds import get_world  # type: ignore


class VerifyError(Exception):
    def __init__(self, code: str, msg: str):
        super().__init__(msg)
        self.code = code
        self.msg = msg


@dataclass
class VerifyResult:
    env_sig: bytes
    uid: bytes
    # overlays
    obj_overlay: Dict[bytes, bytes]


def _parse_cover_data(obj: dict) -> CoverData:
    base = int(obj['baseMask'])
    legs = [int(x) for x in obj['legs']]
    return CoverData(baseMask=base, legs=legs)


def _u32le_to_int(b: bytes) -> int:
    if len(b) != 4:
        raise VerifyError('kcir_v2.contract_violation', 'expected u32le')
    return int.from_bytes(b, 'little', signed=False)


def _read_prim_entry(prims: Dict[bytes, dict], prim_id: bytes) -> Tuple[int, object]:
    if prim_id not in prims:
        raise VerifyError('kcir_v2.data_unavailable', 'missing prim store entry')
    ent = prims[prim_id]
    mask = int(ent.get('mask', 0))
    value = ent.get('value', None)
    return mask, value


def verify_all(
    certs: Dict[bytes, bytes],
    obj_store: Dict[bytes, bytes],
    covers: Dict[bytes, dict],
    prims: Dict[bytes, dict],
    *,
    world_name: str,
) -> VerifyResult:
    """Verify all nodes in `certs`.

    This is fixture-mode verification: we verify every provided node.

    Returns overlays for any constructed ObjNF values.
    """
    world = get_world(world_name)
    if not certs:
        # degenerate fixture
        return VerifyResult(env_sig=b'\x00'*32, uid=b'\x00'*32, obj_overlay={})

    # Decode all nodes and check their refs.
    nodes: Dict[bytes, KCIRNode] = {}
    env_sig: Optional[bytes] = None
    uid: Optional[bytes] = None

    for ref, nb in certs.items():
        nd = decode_node(nb)
        # ref check
        exp_ref = h_node(nb)
        if exp_ref != ref:
            raise VerifyError('kcir_v2.digest_mismatch', 'node digest mismatch')
        if env_sig is None:
            env_sig, uid = nd.env_sig, nd.uid
        else:
            if nd.env_sig != env_sig or nd.uid != uid:
                raise VerifyError('kcir_v2.env_uid_mismatch', 'envSig/uid mismatch across nodes')
        nodes[ref] = nd

    assert env_sig is not None and uid is not None

    # Cycle check with DFS over deps.
    temp: Set[bytes] = set()
    perm: Set[bytes] = set()

    def dfs(r: bytes):
        if r in perm:
            return
        if r in temp:
            raise VerifyError('kcir_v2.dep_cycle', 'dependency cycle detected')
        temp.add(r)
        nd = nodes[r]
        for d in nd.deps:
            if d not in nodes:
                raise VerifyError('kcir_v2.store_missing_node', 'missing dep node')
            dfs(d)
        temp.remove(r)
        perm.add(r)

    for r in list(nodes.keys()):
        dfs(r)

    obj_overlay: Dict[bytes, bytes] = {}

    # Verify each node's local contract.
    for r, nd in nodes.items():
        if nd.sort == 0x01 and nd.opcode == 0x01:
            # COVER/C_LITERAL: args=coverSig
            if len(nd.args) != 32:
                raise VerifyError('kcir_v2.contract_violation', 'C_LITERAL args must be 32 bytes')
            cover_sig = nd.args
            if nd.out != cover_sig:
                raise VerifyError('kcir_v2.contract_violation', 'C_LITERAL out must equal coverSig')
            if cover_sig not in covers:
                raise VerifyError('kcir_v2.data_unavailable', 'cover data missing')
            cd = _parse_cover_data(covers[cover_sig])
            if not validate_cover(cover_sig, cd):
                raise VerifyError('kcir_v2.contract_violation', 'invalid cover data for coverSig')

        elif nd.sort == 0x02 and nd.opcode == 0x01:
            # MAP/M_LITERAL: args=mapId
            if len(nd.args) != 32:
                raise VerifyError('kcir_v2.contract_violation', 'M_LITERAL args must be 32 bytes')
            map_id = nd.args
            if nd.out != map_id:
                raise VerifyError('kcir_v2.contract_violation', 'M_LITERAL out must equal mapId')
            # decode and validate
            decode_map_id(map_id)  # raises on error

        elif nd.sort == 0x03 and nd.opcode == 0x01:
            # OBJ/O_UNIT: args empty
            if len(nd.args) != 0:
                raise VerifyError('kcir_v2.contract_violation', 'O_UNIT args must be empty')
            if nd.deps:
                raise VerifyError('kcir_v2.contract_violation', 'O_UNIT deps must be empty')
            obj_bytes = bytes([0x01])
            exp_out = h_obj(env_sig, uid, obj_bytes)
            if nd.out != exp_out:
                raise VerifyError('kcir_v2.contract_violation', 'O_UNIT out mismatch')
            obj_overlay[nd.out] = obj_bytes

        elif nd.sort == 0x03 and nd.opcode == 0x02:
            # OBJ/O_PRIM: args=primId
            if len(nd.args) != 32:
                raise VerifyError('kcir_v2.contract_violation', 'O_PRIM args must be 32 bytes')
            prim_id = nd.args
            obj_bytes = build_obj_prim(prim_id)
            exp_out = h_obj(env_sig, uid, obj_bytes)
            if nd.out != exp_out:
                raise VerifyError('kcir_v2.contract_violation', 'O_PRIM out mismatch')
            obj_overlay[nd.out] = obj_bytes
            # If prims store is present, ensure the prim is declared (availability).
            if prim_id not in prims:
                raise VerifyError('kcir_v2.data_unavailable', 'missing prim store entry for O_PRIM')
            # If the fixture includes an obj store, it must match.
            if obj_store and nd.out in obj_store and obj_store[nd.out] != obj_bytes:
                raise VerifyError('kcir_v2.store_missing_obj_nf', 'obj store bytes mismatch for constructed object')

        elif nd.sort == 0x03 and nd.opcode == 0x04:
            # OBJ/O_MKGLUE: args = wSig (32 bytes); locals are carried by deps.
            if len(nd.args) != 32:
                raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE args must be exactly wSig (32 bytes)')
            w_sig = nd.args

            if w_sig not in covers:
                raise VerifyError('kcir_v2.data_unavailable', 'O_MKGLUE missing cover data for wSig')
            cd = _parse_cover_data(covers[w_sig])
            if not validate_cover(w_sig, cd):
                raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE invalid cover data for wSig')
            n_locals = cover_len(cd)

            # Proof-carrying trace: require deps = [coverNode] + [localObjNodes...]
            if len(nd.deps) != 1 + n_locals:
                raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE deps must be cover + one dep per local')
            cover_dep = nodes.get(nd.deps[0])
            if cover_dep is None or not (cover_dep.sort == 0x01 and cover_dep.opcode == 0x01 and cover_dep.out == w_sig):
                raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE first dep must be COVER/C_LITERAL with out=wSig')
            locals_refs: list[bytes] = []
            for i in range(n_locals):
                dep = nodes.get(nd.deps[1 + i])
                if dep is None or dep.sort != 0x03:
                    raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE local dep must be OBJ node')
                if dep.opcode not in (0x02, 0x04):
                    raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE local deps must be O_PRIM or O_MKGLUE')
                locals_refs.append(dep.out)

            obj_bytes = build_obj_glue(w_sig, locals_refs)
            exp_out = h_obj(env_sig, uid, obj_bytes)
            if nd.out != exp_out:
                raise VerifyError('kcir_v2.contract_violation', 'O_MKGLUE out mismatch')
            obj_overlay[nd.out] = obj_bytes
            if obj_store and nd.out in obj_store and obj_store[nd.out] != obj_bytes:
                raise VerifyError('kcir_v2.store_missing_obj_nf', 'obj store bytes mismatch for constructed glue object')

        elif nd.sort == 0x03 and nd.opcode == 0x05:
            # OBJ/O_ASSERT_OVERLAP: args = ovMask:u32le; deps = [leftObjNode, rightObjNode]; out = Unit
            if len(nd.args) != 4:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP args must be ovMask:u32le (4 bytes)')
            if len(nd.deps) != 2:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP deps must be exactly 2 OBJ nodes')

            ov_mask = _u32le_to_int(nd.args)

            left = nodes.get(nd.deps[0])
            right = nodes.get(nd.deps[1])
            if left is None or right is None or left.sort != 0x03 or right.sort != 0x03:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP deps must be OBJ nodes')

            # Only defined for Prim in the toy suite.
            left_obj_bytes = obj_overlay.get(left.out) or obj_store.get(left.out)
            right_obj_bytes = obj_overlay.get(right.out) or obj_store.get(right.out)
            if left_obj_bytes is None or right_obj_bytes is None:
                raise VerifyError('kcir_v2.store_missing_obj_nf', 'O_ASSERT_OVERLAP missing ObjNF bytes for dep')

            left_nf = parse_obj_nf(left_obj_bytes)
            right_nf = parse_obj_nf(right_obj_bytes)
            if left_nf.tag != 0x02 or right_nf.tag != 0x02:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP deps must be Prim objects in toy suite')
            l_pid = prim_id_of(left_obj_bytes)
            r_pid = prim_id_of(right_obj_bytes)
            l_mask, l_val = _read_prim_entry(prims, l_pid)
            r_mask, r_val = _read_prim_entry(prims, r_pid)
            exp_ov = l_mask & r_mask
            if ov_mask != exp_ov:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP ovMask does not match masks of deps')

            if not world.validate(l_mask, l_val) or not world.validate(r_mask, r_val):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP prim value ill-typed for its declared mask')
            lr = world.restrict(ov_mask, l_mask, l_val)
            rr = world.restrict(ov_mask, r_mask, r_val)
            if lr is None or rr is None:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP restriction undefined on overlap')
            if not world.validate(ov_mask, lr) or not world.validate(ov_mask, rr):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP restricted value ill-typed on overlap')
            if not world.equal(lr, rr):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP overlap values do not agree')

            unit_bytes = bytes([0x01])
            unit_ref = h_obj(env_sig, uid, unit_bytes)
            if nd.out != unit_ref:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_OVERLAP out must be Unit')
            obj_overlay[nd.out] = unit_bytes

        elif nd.sort == 0x03 and nd.opcode == 0x06:
            # OBJ/O_ASSERT_TRIPLE: args = triMask:u32le; deps = [aObjNode,bObjNode,cObjNode]; out = Unit
            if len(nd.args) != 4:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE args must be triMask:u32le (4 bytes)')
            if len(nd.deps) != 3:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE deps must be exactly 3 OBJ nodes')
            tri_mask = _u32le_to_int(nd.args)

            a = nodes.get(nd.deps[0])
            b = nodes.get(nd.deps[1])
            c = nodes.get(nd.deps[2])
            if a is None or b is None or c is None or a.sort != 0x03 or b.sort != 0x03 or c.sort != 0x03:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE deps must be OBJ nodes')

            a_bytes = obj_overlay.get(a.out) or obj_store.get(a.out)
            b_bytes = obj_overlay.get(b.out) or obj_store.get(b.out)
            c_bytes = obj_overlay.get(c.out) or obj_store.get(c.out)
            if a_bytes is None or b_bytes is None or c_bytes is None:
                raise VerifyError('kcir_v2.store_missing_obj_nf', 'O_ASSERT_TRIPLE missing ObjNF bytes for dep')

            a_nf = parse_obj_nf(a_bytes)
            b_nf = parse_obj_nf(b_bytes)
            c_nf = parse_obj_nf(c_bytes)
            if a_nf.tag != 0x02 or b_nf.tag != 0x02 or c_nf.tag != 0x02:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE deps must be Prim objects in toy suite')
            a_pid = prim_id_of(a_bytes)
            b_pid = prim_id_of(b_bytes)
            c_pid = prim_id_of(c_bytes)
            a_mask, a_val = _read_prim_entry(prims, a_pid)
            b_mask, b_val = _read_prim_entry(prims, b_pid)
            c_mask, c_val = _read_prim_entry(prims, c_pid)
            exp_tri = a_mask & b_mask & c_mask
            if tri_mask != exp_tri:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE triMask does not match masks of deps')

            if not world.validate(a_mask, a_val) or not world.validate(b_mask, b_val) or not world.validate(c_mask, c_val):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE prim value ill-typed for its declared mask')
            ar = world.restrict(tri_mask, a_mask, a_val)
            br = world.restrict(tri_mask, b_mask, b_val)
            cr = world.restrict(tri_mask, c_mask, c_val)
            if ar is None or br is None or cr is None:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE restriction undefined on triple-overlap')
            if not world.validate(tri_mask, ar) or not world.validate(tri_mask, br) or not world.validate(tri_mask, cr):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE restricted value ill-typed on triple-overlap')
            if not (world.equal(ar, br) and world.equal(br, cr)):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE triple-overlap values do not agree')

            unit_bytes = bytes([0x01])
            unit_ref = h_obj(env_sig, uid, unit_bytes)
            if nd.out != unit_ref:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_TRIPLE out must be Unit')
            obj_overlay[nd.out] = unit_bytes

        elif nd.sort == 0x03 and nd.opcode == 0x07:
            # OBJ/O_ASSERT_CONTRACTIBLE: args = schemeId:Bytes32 || proofBytes; deps=[glueNode]; out=Unit
            if len(nd.args) < 32:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE args must begin with schemeId:Bytes32')
            scheme_id = nd.args[:32]
            proof = nd.args[32:]
            # In the toy suite we currently support only the enumeration-based scheme.
            if scheme_id != SCHEME_TOY_ENUMERATE_V1:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE unsupported proof scheme id')
            if len(nd.deps) != 1:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE deps must be exactly one OBJ node (the glue candidate)')
            glue_node = nodes.get(nd.deps[0])
            if glue_node is None or glue_node.sort != 0x03 or glue_node.opcode != 0x04:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE dep must be OBJ/O_MKGLUE')

            glue_obj_ref = glue_node.out
            glue_obj_bytes = obj_overlay.get(glue_obj_ref) or obj_store.get(glue_obj_ref)
            if glue_obj_bytes is None:
                raise VerifyError('kcir_v2.store_missing_obj_nf', 'O_ASSERT_CONTRACTIBLE missing ObjNF bytes for glue object')
            glue_nf = parse_obj_nf(glue_obj_bytes)
            if glue_nf.tag != 0x06:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE dep out must be an ObjNF Glue')

            # Extract cover + locals from the Glue object.
            from nf_codec import glue_fields  # local import to avoid cycles
            w_sig, locals_refs = glue_fields(glue_obj_bytes)
            if w_sig not in covers:
                raise VerifyError('kcir_v2.data_unavailable', 'O_ASSERT_CONTRACTIBLE missing cover data for glue')
            cd = _parse_cover_data(covers[w_sig])
            if not validate_cover(w_sig, cd):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE invalid cover data')
            if len(locals_refs) != cover_len(cd):
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE locals length mismatch with cover')

            # Load local semantic values (toy suite currently requires Prim locals).
            locals_vals: list[object] = []
            for idx, (leg_mask, obj_ref) in enumerate(zip(cd.legs, locals_refs)):
                ob = obj_overlay.get(obj_ref) or obj_store.get(obj_ref)
                if ob is None:
                    raise VerifyError('kcir_v2.store_missing_obj_nf', 'O_ASSERT_CONTRACTIBLE missing ObjNF bytes for local')
                onf = parse_obj_nf(ob)
                if onf.tag != 0x02:
                    raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE locals must be Prim objects in toy suite')
                pid = prim_id_of(ob)
                pmask, pval = _read_prim_entry(prims, pid)
                if pmask != leg_mask:
                    raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE prim mask does not match cover leg mask')
                if not world.validate(leg_mask, pval):
                    raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE local value ill-typed for leg mask')
                locals_vals.append(pval)

            # Proof hook: delegate contractibility checking to the world/scheme.
            ok = world.verify_contractible(
                scheme_id,
                proof,
                base_mask=cd.baseMask,
                legs=cd.legs,
                locals_vals=locals_vals,
            )
            if not ok:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE failed: glue space not contractible')

            unit_bytes = bytes([0x01])
            unit_ref = h_obj(env_sig, uid, unit_bytes)
            if nd.out != unit_ref:
                raise VerifyError('kcir_v2.contract_violation', 'O_ASSERT_CONTRACTIBLE out must be Unit')
            obj_overlay[nd.out] = unit_bytes

        else:
            raise VerifyError('kcir_v2.unsupported_opcode', f'unsupported (sort,opcode)=({nd.sort:#x},{nd.opcode:#x})')

    return VerifyResult(env_sig=env_sig, uid=uid, obj_overlay=obj_overlay)
