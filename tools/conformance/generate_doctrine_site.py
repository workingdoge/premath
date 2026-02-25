#!/usr/bin/env python3
"""Generate canonical doctrine-site artifacts from site-package sources."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import doctrine_site_contract

DOCTRINE_SITE_GENERATION_DIGEST_KIND = "premath.doctrine_site_generation_digest.v1"


def parse_args() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description=(
            "Generate specs/premath/draft/DOCTRINE-SITE-INPUT.json, "
            "Generate specs/premath/draft/DOCTRINE-SITE.json and "
            "specs/premath/draft/DOCTRINE-OP-REGISTRY.json from "
            "site-package source(s) and parsed declarations."
        )
    )
    parser.add_argument(
        "--packages-root",
        type=Path,
        default=repo_root / "specs" / "premath" / "site-packages",
        help="Site-package source root",
    )
    parser.add_argument(
        "--input-map",
        "--input-output",
        "--source-map",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-INPUT.json",
        help="Canonical doctrine-site input JSON output path",
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
        "--digest-output",
        type=Path,
        default=repo_root
        / "specs"
        / "premath"
        / "draft"
        / "DOCTRINE-SITE-GENERATION-DIGEST.json",
        help="Output doctrine generation-digest JSON path",
    )
    parser.add_argument(
        "--cutover-contract",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "DOCTRINE-SITE-CUTOVER.json",
        help="Doctrine-site migration/cutover contract JSON path",
    )
    parser.add_argument(
        "--operation-registry",
        type=Path,
        default=None,
        help=(
            "Optional operation registry override input path "
            "(compatibility-only surface; rejected when cutover phase disables overrides)"
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
    packages_root = args.packages_root.resolve()
    input_path = args.input_map.resolve()
    output_path = args.output.resolve()
    registry_output_path = args.operation_registry_output.resolve()
    digest_output_path = args.digest_output.resolve()
    cutover_contract_path = args.cutover_contract.resolve()
    registry_override_path = (
        args.operation_registry.resolve() if args.operation_registry else None
    )

    try:
        generated_input = doctrine_site_contract.generate_site_input_from_packages(
            repo_root=repo_root,
            packages_root=packages_root,
        )
        generated_input_text = doctrine_site_contract.canonical_site_input_json(
            generated_input, pretty=True
        )
        if not args.check:
            input_path.parent.mkdir(parents=True, exist_ok=True)
            input_path.write_text(generated_input_text, encoding="utf-8")
        generated_site = doctrine_site_contract.generate_site_map(
            repo_root=repo_root,
            site_input_path=input_path,
            operation_registry_path=registry_override_path,
            cutover_contract_path=cutover_contract_path,
        )
        generated_registry = doctrine_site_contract.generate_operation_registry(
            repo_root=repo_root,
            site_input_path=input_path,
            operation_registry_path=registry_override_path,
            cutover_contract_path=cutover_contract_path,
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

    expected_digest_payload = {
        "schema": 1,
        "digestKind": DOCTRINE_SITE_GENERATION_DIGEST_KIND,
        "source": {
            "packagesRoot": packages_root.relative_to(repo_root).as_posix(),
            "packageGlob": "**/SITE-PACKAGE.json",
            "generator": "tools/conformance/generate_doctrine_site.py",
            "cutoverContract": cutover_contract_path.relative_to(repo_root).as_posix(),
        },
        "artifacts": {
            "siteInput": {
                "path": input_path.relative_to(repo_root).as_posix(),
                "sha256": doctrine_site_contract.site_input_digest(generated_input),
            },
            "siteMap": {
                "path": output_path.relative_to(repo_root).as_posix(),
                "sha256": doctrine_site_contract.site_map_digest(generated_site),
            },
            "operationRegistry": {
                "path": registry_output_path.relative_to(repo_root).as_posix(),
                "sha256": doctrine_site_contract.operation_registry_digest(
                    generated_registry
                ),
            },
        },
    }

    if args.check:
        if (
            not input_path.exists()
            or not output_path.exists()
            or not registry_output_path.exists()
            or not digest_output_path.exists()
        ):
            missing = [
                str(path)
                for path in (input_path, output_path, registry_output_path, digest_output_path)
                if not path.exists()
            ]
            print(
                "[doctrine-site-generate] FAIL missing output(s): "
                + ", ".join(missing)
            )
            return 1
        try:
            existing_input = doctrine_site_contract.load_json_object(input_path)
            existing_site = doctrine_site_contract.load_json_object(output_path)
            existing_registry = doctrine_site_contract.load_json_object(registry_output_path)
            existing_digest = doctrine_site_contract.load_json_object(digest_output_path)
            errors = doctrine_site_contract.site_input_equality_diff(
                generated_input, existing_input
            )
            errors.extend(doctrine_site_contract.equality_diff(generated_site, existing_site))
            errors.extend(
                doctrine_site_contract.operation_registry_equality_diff(
                    generated_registry, existing_registry
                )
            )
            if existing_digest != expected_digest_payload:
                errors.append(
                    "roundtrip mismatch: tracked doctrine generation digest differs from generated output"
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
        input_digest = doctrine_site_contract.site_input_digest(generated_input)
        registry_digest = doctrine_site_contract.operation_registry_digest(
            generated_registry
        )
        print(
            "[doctrine-site-generate] OK "
            f"(mode=check, input={input_path}, inputDigest={input_digest}, "
            f"site={output_path}, siteDigest={digest}, "
            f"registry={registry_output_path}, registryDigest={registry_digest})"
        )
        return 0

    input_digest = doctrine_site_contract.site_input_digest(generated_input)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(generated_site_text, encoding="utf-8")
    registry_output_path.parent.mkdir(parents=True, exist_ok=True)
    registry_output_path.write_text(generated_registry_text, encoding="utf-8")
    digest_output_path.parent.mkdir(parents=True, exist_ok=True)
    digest_output_path.write_text(
        json.dumps(expected_digest_payload, indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )
    digest = doctrine_site_contract.site_map_digest(generated_site)
    registry_digest = doctrine_site_contract.operation_registry_digest(
        generated_registry
    )
    print(
        "[doctrine-site-generate] OK "
        f"(mode=write, input={input_path}, inputDigest={input_digest}, "
        f"site={output_path}, siteDigest={digest}, "
        f"registry={registry_output_path}, registryDigest={registry_digest})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
