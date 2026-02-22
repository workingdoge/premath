#!/usr/bin/env python3
"""Generate canonical doctrine-site map from source + operation registry."""

from __future__ import annotations

import argparse
from pathlib import Path

import doctrine_site_contract


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Generate specs/premath/draft/DOCTRINE-SITE.json from "
            "DOCTRINE-SITE-SOURCE.json + DOCTRINE-OP-REGISTRY.json and parsed declarations."
        )
    )
    parser.add_argument(
        "--source-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-SOURCE.json",
        help="Source doctrine-site topology JSON",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json",
        help="Output doctrine-site JSON path",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=None,
        help="Optional operation registry override path",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Do not write output; fail unless existing output equals generated content.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[2]
    source_path = args.source_map.resolve()
    output_path = args.output.resolve()
    op_registry_path = args.operation_registry.resolve() if args.operation_registry else None

    try:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=repo_root,
            source_map_path=source_path,
            operation_registry_path=op_registry_path,
        )
        generated_text = doctrine_site_contract.canonical_site_map_json(generated, pretty=True)
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-site-generate] FAIL generate: {exc}")
        return 1

    if args.check:
        if not output_path.exists():
            print(f"[doctrine-site-generate] FAIL missing output: {output_path}")
            return 1
        try:
            existing = doctrine_site_contract.load_json_object(output_path)
            errors = doctrine_site_contract.equality_diff(generated, existing)
        except Exception as exc:  # noqa: BLE001
            print(f"[doctrine-site-generate] FAIL read output: {exc}")
            return 1
        if errors:
            print("[doctrine-site-generate] FAIL drift")
            for error in errors:
                print(f"  - {error}")
            return 1
        digest = doctrine_site_contract.site_map_digest(generated)
        print(
            "[doctrine-site-generate] OK "
            f"(mode=check, output={output_path}, digest={digest})"
        )
        return 0

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(generated_text, encoding="utf-8")
    digest = doctrine_site_contract.site_map_digest(generated)
    print(
        "[doctrine-site-generate] OK "
        f"(mode=write, output={output_path}, digest={digest})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

