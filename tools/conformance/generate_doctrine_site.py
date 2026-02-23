#!/usr/bin/env python3
"""Generate canonical doctrine-site artifacts from one input contract."""

from __future__ import annotations

import argparse
from pathlib import Path

import doctrine_site_contract


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Generate specs/premath/draft/DOCTRINE-SITE.json and "
            "specs/premath/draft/DOCTRINE-OP-REGISTRY.json from "
            "DOCTRINE-SITE-INPUT.json and parsed declarations."
        )
    )
    parser.add_argument(
        "--input-map",
        "--source-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json",
        help="Canonical doctrine-site input JSON",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE.json",
        help="Output doctrine-site map JSON path",
    )
    parser.add_argument(
        "--operation-registry-output",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-OP-REGISTRY.json",
        help="Output doctrine operation-registry JSON path",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=None,
        help=(
            "Optional operation registry override input path "
            "(legacy/diagnostic surface; bypasses embedded input registry)"
        ),
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
    input_path = args.input_map.resolve()
    output_path = args.output.resolve()
    registry_output_path = args.operation_registry_output.resolve()
    registry_override_path = (
        args.operation_registry.resolve() if args.operation_registry else None
    )

    try:
        generated_site = doctrine_site_contract.generate_site_map(
            repo_root=repo_root,
            site_input_path=input_path,
            operation_registry_path=registry_override_path,
        )
        generated_registry = doctrine_site_contract.generate_operation_registry(
            repo_root=repo_root,
            site_input_path=input_path,
            operation_registry_path=registry_override_path,
        )
        generated_site_text = doctrine_site_contract.canonical_site_map_json(
            generated_site, pretty=True
        )
        generated_registry_text = doctrine_site_contract.canonical_operation_registry_json(
            generated_registry, pretty=True
        )
    except Exception as exc:  # noqa: BLE001
        print(f"[doctrine-site-generate] FAIL generate: {exc}")
        return 1

    if args.check:
        if not output_path.exists() or not registry_output_path.exists():
            missing = [
                str(path)
                for path in (output_path, registry_output_path)
                if not path.exists()
            ]
            print(
                "[doctrine-site-generate] FAIL missing output(s): "
                + ", ".join(missing)
            )
            return 1
        try:
            existing_site = doctrine_site_contract.load_json_object(output_path)
            existing_registry = doctrine_site_contract.load_json_object(registry_output_path)
            errors = doctrine_site_contract.equality_diff(generated_site, existing_site)
            errors.extend(
                doctrine_site_contract.operation_registry_equality_diff(
                    generated_registry, existing_registry
                )
            )
        except Exception as exc:  # noqa: BLE001
            print(f"[doctrine-site-generate] FAIL read output: {exc}")
            return 1
        if errors:
            print("[doctrine-site-generate] FAIL drift")
            for error in errors:
                print(f"  - {error}")
            return 1
        digest = doctrine_site_contract.site_map_digest(generated_site)
        registry_digest = doctrine_site_contract.operation_registry_digest(
            generated_registry
        )
        print(
            "[doctrine-site-generate] OK "
            f"(mode=check, site={output_path}, siteDigest={digest}, "
            f"registry={registry_output_path}, registryDigest={registry_digest})"
        )
        return 0

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(generated_site_text, encoding="utf-8")
    registry_output_path.parent.mkdir(parents=True, exist_ok=True)
    registry_output_path.write_text(generated_registry_text, encoding="utf-8")
    digest = doctrine_site_contract.site_map_digest(generated_site)
    registry_digest = doctrine_site_contract.operation_registry_digest(
        generated_registry
    )
    print(
        "[doctrine-site-generate] OK "
        f"(mode=write, site={output_path}, siteDigest={digest}, "
        f"registry={registry_output_path}, registryDigest={registry_digest})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
