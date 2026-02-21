"""Run the KCIR toy gate vector suite."""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Dict

from gate_from_fixture import run_gate_from_fixture
from kcir_store import FixtureStore
from kcir_verify import verify_all, VerifyError


def _load_json(path: str) -> Any:
    return json.load(open(path, 'r', encoding='utf-8'))


def _eq(a: Any, b: Any) -> bool:
    return json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)


def run_one(fixture_dir: str) -> bool:
    store = FixtureStore.load(fixture_dir)
    case = _load_json(os.path.join(fixture_dir, 'case.json'))
    world = case.get('world', 'sheaf_bits')
    # Core verify
    try:
        verify_all(store.certs, store.obj, store.covers, store.prims, world_name=world)
    except VerifyError as e:
        print(f'[core] FAIL {os.path.basename(fixture_dir)}: {e.code}: {e.msg}')
        return False

    # Gate check
    got = run_gate_from_fixture(fixture_dir)

    exp_path = os.path.join(fixture_dir, 'expect.json')
    if os.path.exists(exp_path):
        exp = _load_json(exp_path)
        if not _eq(got, exp):
            print(f'[gate] FAIL {os.path.basename(fixture_dir)}: output mismatch')
            print('--- expected ---')
            print(json.dumps(exp, indent=2, sort_keys=True))
            print('--- got ---')
            print(json.dumps(got, indent=2, sort_keys=True))
            return False

    print(f'[ok] {os.path.basename(fixture_dir)}')
    return True


def main() -> None:
    import argparse

    ap = argparse.ArgumentParser()
    ap.add_argument('--fixtures', default='tests/kcir_toy/fixtures')
    args = ap.parse_args()

    if not os.path.isdir(args.fixtures):
        print('fixtures directory not found; did you run compile_kcir_toy_fixtures.py?', file=sys.stderr)
        sys.exit(2)

    case_ids = sorted([d for d in os.listdir(args.fixtures) if os.path.isdir(os.path.join(args.fixtures, d))])
    ok = 0
    bad = 0
    for cid in case_ids:
        if run_one(os.path.join(args.fixtures, cid)):
            ok += 1
        else:
            bad += 1

    print(f'\nsummary: {ok} ok, {bad} failed')
    sys.exit(0 if bad == 0 else 1)


if __name__ == '__main__':
    main()
