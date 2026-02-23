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
            "- generated map/registry roundtrip from canonical input,\n"
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
        "--input-map",
        "--source-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json",
        help="Canonical doctrine-site input JSON",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
        help="Tracked doctrine operation-registry JSON",
    )
    parser.add_argument(
        "--operation-registry-override",
        type=Path,
        default=None,
        help=(
            "Optional operation-registry override input path "
            "(legacy/diagnostic surface)"
        ),
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
    input_map_path = args.input_map.resolve()
    operation_registry_path = args.operation_registry.resolve()
    operation_registry_override_path = (
        args.operation_registry_override.resolve()
        if args.operation_registry_override
        else None
    )

    errors: list[str] = []

    try:
        generated_site = doctrine_site_contract.generate_site_map(
            repo_root=repo_root,
            site_input_path=input_map_path,
            operation_registry_path=operation_registry_override_path,
        )
        generated_registry = doctrine_site_contract.generate_operation_registry(
            repo_root=repo_root,
            site_input_path=input_map_path,
            operation_registry_path=operation_registry_override_path,
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[error] failed to generate doctrine site map: {exc}")
        return 1

    if args.write_generated:
        try:
            site_map_path.parent.mkdir(parents=True, exist_ok=True)
            site_map_path.write_text(
                doctrine_site_contract.canonical_site_map_json(generated_site, pretty=True),
                encoding="utf-8",
            )
            operation_registry_path.parent.mkdir(parents=True, exist_ok=True)
            operation_registry_path.write_text(
                doctrine_site_contract.canonical_operation_registry_json(
                    generated_registry, pretty=True
                ),
                encoding="utf-8",
            )
        except Exception as exc:  # noqa: BLE001
            print(f"[error] failed to write generated doctrine site artifacts: {exc}")
            return 1

    if not site_map_path.exists() or not operation_registry_path.exists():
        missing = [
            str(path)
            for path in (site_map_path, operation_registry_path)
            if not path.exists()
        ]
        print(f"[error] missing tracked doctrine site artifact(s): {', '.join(missing)}")
        print(
            "[hint] run: "
            "python3 tools/conformance/generate_doctrine_site.py"
        )
        return 1

    try:
        tracked_site = doctrine_site_contract.load_json_object(site_map_path)
        tracked_registry = doctrine_site_contract.load_json_object(operation_registry_path)
    except Exception as exc:  # noqa: BLE001
        print(f"[error] failed to load tracked doctrine site artifacts: {exc}")
        return 1

    site_roundtrip_errors = doctrine_site_contract.equality_diff(generated_site, tracked_site)
    registry_roundtrip_errors = doctrine_site_contract.operation_registry_equality_diff(
        generated_registry, tracked_registry
    )
    if site_roundtrip_errors or registry_roundtrip_errors:
        errors.extend(site_roundtrip_errors)
        errors.extend(registry_roundtrip_errors)
        errors.append(
            "tracked doctrine artifacts drifted from generated source; run "
            "`python3 tools/conformance/generate_doctrine_site.py`"
        )

    errors.extend(
        doctrine_site_contract.validate_site_map(
            repo_root=repo_root,
            site_map=tracked_site,
        )
    )

    if errors:
        for error in errors:
            print(f"[error] {error}")
        print(f"[fail] doctrine site check failed (errors={len(errors)})")
        return 1

    nodes, edges, covers, operations = doctrine_site_contract.summarize_site_map(tracked_site)
    digest = doctrine_site_contract.site_map_digest(tracked_site)
    registry_digest = doctrine_site_contract.operation_registry_digest(tracked_registry)
    print(
        "[ok] doctrine site check passed "
        f"(nodes={nodes}, edges={edges}, covers={covers}, operations={operations}, "
        f"siteDigest={digest[:12]}, registryDigest={registry_digest[:12]})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
