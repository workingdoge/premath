#!/usr/bin/env python3
"""Export GitHub provider variables as provider-neutral Premath CI refs."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

from provider_env import map_github_to_premath_env  # type: ignore  # noqa: E402


def main() -> int:
    mapped = map_github_to_premath_env(os.environ)
    if len(sys.argv) > 1 and sys.argv[1] == "--json":
        json.dump(mapped, sys.stdout, sort_keys=True, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")
        return 0

    for key in sorted(mapped):
        print(f"{key}={mapped[key]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
