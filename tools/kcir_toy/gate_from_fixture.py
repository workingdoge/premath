"""Gate checking for KCIR toy fixtures.

We *decode* the KCIR-shaped fixture back into the semantic case format used by
`tools/toy/toy_gate_check.py` and reuse that logic.

This keeps the toy suite small and ensures witness IDs match the semantic toy
suite (by construction).

Non-normative tooling.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Dict, List

from kcir_store import FixtureStore
from kcir_codec import decode_node
from nf_codec import parse_obj_nf, prim_id_of, glue_fields
from toy_baseapi import decode_map_id


# Import the semantic toy world + gate checker.
_THIS_DIR = os.path.dirname(__file__)
sys.path.append(os.path.join(_THIS_DIR, '..', 'toy'))

from toy_gate_check import run_case  # type: ignore
from toy_worlds import get_world  # type: ignore


def _bytes32(hex_str: str) -> bytes:
    b = bytes.fromhex(hex_str)
    if len(b) != 32:
        raise ValueError('expected bytes32 hex')
    return b


def _load_json(path: str) -> Any:
    return json.load(open(path, 'r', encoding='utf-8'))


def _value_for_objref(store: FixtureStore, obj_ref: bytes, *, world: Any, memo: Dict[bytes, Any]) -> Any:
    """Interpret an ObjNF reference into a semantic toy value.

    Non-normative tooling.

    - Prim: read from the prim store.
    - Glue: deterministically choose the first global candidate (if any) that
      restricts to the locals on the cover legs.
    """
    if obj_ref in memo:
        return memo[obj_ref]
    if obj_ref not in store.obj:
        raise ValueError('missing obj bytes for ref')
    obj_bytes = store.obj[obj_ref]
    n = parse_obj_nf(obj_bytes)
    if n.tag == 0x02:
        prim_id = prim_id_of(obj_bytes)
        if prim_id not in store.prims:
            raise ValueError('missing prim store entry')
        entry = store.prims[prim_id]
        memo[obj_ref] = entry['value']
        return memo[obj_ref]

    if n.tag == 0x06:
        w_sig, locals_refs = glue_fields(obj_bytes)
        if w_sig not in store.covers:
            raise ValueError('missing cover data for glue object')
        cov = store.covers[w_sig]
        base = int(cov['baseMask'])
        legs = [int(x) for x in cov['legs']]
        locals_vals = [_value_for_objref(store, r, world=world, memo=memo) for r in locals_refs]

        # Filter the world's enumeration.
        cands: List[Any] = []
        for cand in world.enumerate(base):
            ok = True
            for lm, lv in zip(legs, locals_vals):
                if not world.validate(lm, lv):
                    ok = False
                    break
                if world.restrict(lm, base, cand) != lv:
                    ok = False
                    break
            if ok:
                cands.append(cand)
        memo[obj_ref] = cands[0] if cands else None
        return memo[obj_ref]

    raise ValueError(f'unsupported ObjNF tag: {n.tag:#x}')


def to_semantic_case(store: FixtureStore, check_json: Dict[str, Any]) -> Dict[str, Any]:
    """Convert compiled gate/check.json to the semantic toy case schema."""
    world = check_json['world']
    w = get_world(world)
    memo: Dict[bytes, Any] = {}
    check = check_json['check']
    kind = check['kind']

    out_check: Dict[str, Any] = {'kind': kind, 'tokenPath': check.get('tokenPath', None)}

    if kind == 'stability':
        gamma = int(check['gammaMask'])
        a_ref = _bytes32(check['aObj'])
        f_id = _bytes32(check['fMap'])
        g_id = _bytes32(check['gMap'])
        f_src, f_tgt = decode_map_id(f_id)
        g_src, g_tgt = decode_map_id(g_id)
        # sanity
        if f_tgt != gamma or g_tgt != f_src:
            # still produce the case; semantic checker will likely reject
            pass
        out_check.update({
            'gammaMask': gamma,
            'a': _value_for_objref(store, a_ref, world=w, memo=memo),
            # Match the semantic toy case schema.
            'f': {'src': f_src, 'tgt': f_tgt},
            'g': {'src': g_src, 'tgt': g_tgt},
        })

    elif kind == 'locality':
        gamma = int(check['gammaMask'])
        a_ref = _bytes32(check['aObj'])
        cover_sig = _bytes32(check['coverSig'])
        if cover_sig not in store.covers:
            raise ValueError('missing cover data')
        legs = store.covers[cover_sig]['legs']
        out_check.update({
            'gammaMask': gamma,
            'a': _value_for_objref(store, a_ref, world=w, memo=memo),
            'legs': legs,
        })

    elif kind == 'descent':
        base = int(check['baseMask'])
        cover_sig = _bytes32(check['coverSig'])
        if cover_sig not in store.covers:
            raise ValueError('missing cover data')
        legs = store.covers[cover_sig]['legs']
        local_refs = [_bytes32(h) for h in check['localObjs']]
        locals_vals = [_value_for_objref(store, r, world=w, memo=memo) for r in local_refs]

        # Optional: certified overlap/cocycle witnesses (toy KCIR suite).
        # If present and structurally complete, we can skip recomputing overlap checks in the semantic gate checker.
        overlap_nodes_hex = check.get('overlapNodes', None)
        triple_nodes_hex = check.get('tripleNodes', None)
        contractible_node_hex = check.get('contractibleNode', None)
        local_nodes_hex = check.get('localNodes', None)

        if overlap_nodes_hex is not None and local_nodes_hex is not None:
            local_nodes = [_bytes32(h) for h in local_nodes_hex]
            overlap_nodes = [_bytes32(h) for h in overlap_nodes_hex]

            # Expected pair order is lexicographic (i<j).
            exp_pairs = []
            for i in range(len(legs)):
                for j in range(i + 1, len(legs)):
                    exp_pairs.append((i, j, int(legs[i]) & int(legs[j])))
            if len(overlap_nodes) != len(exp_pairs):
                raise ValueError('overlapNodes length mismatch')

            for idx, (i, j, ov) in enumerate(exp_pairs):
                nr = overlap_nodes[idx]
                if nr not in store.certs:
                    raise ValueError('missing overlap witness node bytes')
                nd = decode_node(store.certs[nr])
                if nd.sort != 0x03 or nd.opcode != 0x05:
                    raise ValueError('overlap witness node has wrong (sort,opcode)')
                if nd.deps != [local_nodes[i], local_nodes[j]]:
                    raise ValueError('overlap witness node deps mismatch')
                if int.from_bytes(nd.args[:4], 'little') != ov:
                    raise ValueError('overlap witness node mask mismatch')

            out_check['overlapCertified'] = True

        if triple_nodes_hex is not None and local_nodes_hex is not None:
            local_nodes = [_bytes32(h) for h in local_nodes_hex]
            triple_nodes = [_bytes32(h) for h in triple_nodes_hex]
            exp_triples = []
            for i in range(len(legs)):
                for j in range(i + 1, len(legs)):
                    for k in range(j + 1, len(legs)):
                        exp_triples.append((i, j, k, int(legs[i]) & int(legs[j]) & int(legs[k])))
            if len(triple_nodes) != len(exp_triples):
                raise ValueError('tripleNodes length mismatch')
            for idx, (i, j, k, tri) in enumerate(exp_triples):
                nr = triple_nodes[idx]
                if nr not in store.certs:
                    raise ValueError('missing triple witness node bytes')
                nd = decode_node(store.certs[nr])
                if nd.sort != 0x03 or nd.opcode != 0x06:
                    raise ValueError('triple witness node has wrong (sort,opcode)')
                if nd.deps != [local_nodes[i], local_nodes[j], local_nodes[k]]:
                    raise ValueError('triple witness node deps mismatch')
                if int.from_bytes(nd.args[:4], 'little') != tri:
                    raise ValueError('triple witness node mask mismatch')
            out_check['cocycleCertified'] = True

        # Optional: contractible gluing witness (toy KCIR suite).
        # If present, the semantic gate checker can skip enumerating candidates.
        if contractible_node_hex is not None:
            nr = _bytes32(contractible_node_hex)
            if nr not in store.certs:
                raise ValueError('missing contractible witness node bytes')
            nd = decode_node(store.certs[nr])
            if nd.sort != 0x03 or nd.opcode != 0x07:
                raise ValueError('contractible witness node has wrong (sort,opcode)')
            if len(nd.args) < 32:
                raise ValueError('contractible witness node args must begin with schemeId:Bytes32')
            if len(nd.deps) != 1:
                raise ValueError('contractible witness node must have exactly one dep')
            dep = nd.deps[0]
            if dep not in store.certs:
                raise ValueError('missing glue dep for contractible witness')
            gnd = decode_node(store.certs[dep])
            if gnd.sort != 0x03 or gnd.opcode != 0x04:
                raise ValueError('contractible witness dep must be OBJ/O_MKGLUE')
            # Tie the witness to the glue object referenced by the check.
            if 'glueObj' in check:
                glue_obj_ref = _bytes32(check['glueObj'])
                if gnd.out != glue_obj_ref:
                    raise ValueError('contractible witness dep glue out mismatch')
            out_check['contractibleCertified'] = True
        if 'glueObj' in check:
            glue_ref = _bytes32(check['glueObj'])
            out_check['glue'] = _value_for_objref(store, glue_ref, world=w, memo=memo)
        out_check.update({
            'baseMask': base,
            'legs': legs,
            'locals': locals_vals,
        })
    else:
        raise ValueError(f'unknown kind: {kind}')

    return {'schema': 1, 'world': world, 'check': out_check}


def run_gate_from_fixture(fixture_dir: str) -> Dict[str, Any]:
    store = FixtureStore.load(fixture_dir)
    gate_path = os.path.join(fixture_dir, 'gate', 'check.json')
    check_json = _load_json(gate_path)
    semantic_case = to_semantic_case(store, check_json)
    return run_case(semantic_case)
