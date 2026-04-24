#!/usr/bin/env python3
"""Validate the typed control-plane contract loader surface."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import control_plane_contract


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate CONTROL-PLANE-CONTRACT.json through the shared loader."
    )
    parser.add_argument(
        "--contract",
        type=Path,
        default=control_plane_contract.CONTROL_PLANE_CONTRACT_PATH,
        help="Control-plane contract path.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit a compact JSON status payload.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        loaded = control_plane_contract.load_control_plane_contract(args.contract)
    except Exception as exc:
        if args.json:
            print(
                json.dumps(
                    {
                        "result": "rejected",
                        "contractPath": str(args.contract),
                        "reason": str(exc),
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        else:
            print(f"[control-plane-contract] FAIL ({exc})")
        return 1

    payload = {
        "result": "accepted",
        "contractPath": str(args.contract),
        "contractKind": loaded["contractKind"],
        "runtimeRouteCount": len(
            loaded["runtimeRouteBindings"]["requiredOperationRoutes"]
        ),
        "replHostActionCount": len(
            loaded["replHostActionBindings"]["actions"]
        ),
    }
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print(
            "[control-plane-contract] OK "
            f"(runtimeRoutes={payload['runtimeRouteCount']}, "
            f"replHostActions={payload['replHostActionCount']})"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
