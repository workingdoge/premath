#!/usr/bin/env python3
"""Validate doctrine-site coherence with generation-first roundtrip checks."""

from __future__ import annotations

import argparse
from pathlib import Path

import doctrine_site_contract


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Validate doctrine-site coherence:\n"
            "- generated map roundtrip from source + op registry,\n"
            "- declaration/morphism/edge/cover coherence,\n"
            "- doctrine root reachability to operation nodes."
        )
    )
    parser.add_argument(
        "--site-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json",
        help="Tracked doctrine site map JSON",
    )
    parser.add_argument(
        "--source-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-SOURCE.json",
        help="Source doctrine site topology JSON",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=None,
        help="Optional operation registry override path",
    )
    parser.add_argument(
        "--write-generated",
        action="store_true",
        help="Write generated map to --site-map before validation.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[2]
    site_map_path = args.site_map.resolve()
    source_map_path = args.source_map.resolve()
    operation_registry_path = args.operation_registry.resolve() if args.operation_registry else None

    errors: list[str] = []

    try:
        generated = doctrine_site_contract.generate_site_map(
            repo_root=repo_root,
            source_map_path=source_map_path,
            operation_registry_path=operation_registry_path,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[error] failed to generate doctrine site map: {exc}")
        return 1

    if args.write_generated:
        try:
            site_map_path.parent.mkdir(parents=True, exist_ok=True)
            site_map_path.write_text(
                doctrine_site_contract.canonical_site_map_json(generated, pretty=True),
                encoding="utf-8",
            )
        except Exception as exc:  # noqa: BLE001
            print(f"[error] failed to write generated doctrine site map: {exc}")
            return 1

    if not site_map_path.exists():
        print(f"[error] missing tracked doctrine site map: {site_map_path}")
        print(
            "[hint] run: "
            f"python3 tools/conformance/generate_doctrine_site.py --output {site_map_path}"
        )
        return 1

    try:
        tracked = doctrine_site_contract.load_json_object(site_map_path)
    except Exception as exc:  # noqa: BLE001
        print(f"[error] failed to load tracked doctrine site map: {exc}")
        return 1

    roundtrip_errors = doctrine_site_contract.equality_diff(generated, tracked)
    if roundtrip_errors:
        errors.extend(roundtrip_errors)
        errors.append(
            "tracked map drifted from generated source; run "
            "`python3 tools/conformance/generate_doctrine_site.py`"
        )

    errors.extend(
        doctrine_site_contract.validate_site_map(
            repo_root=repo_root,
            site_map=tracked,
        )
    )

    if errors:
        for error in errors:
            print(f"[error] {error}")
        print(f"[fail] doctrine site check failed (errors={len(errors)})")
        return 1

    nodes, edges, covers, operations = doctrine_site_contract.summarize_site_map(tracked)
    digest = doctrine_site_contract.site_map_digest(tracked)
    print(
        "[ok] doctrine site check passed "
        f"(nodes={nodes}, edges={edges}, covers={covers}, operations={operations}, digest={digest[:12]})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

