#!/usr/bin/env python3
"""Validate projection-only statement-index -> binding lane invariants."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Dict, List, Sequence, Set

import check_statement_index

SCHEMA = 1
CHECK_KIND = "conformance.statement_projection_lane.v1"
FAILURE_CLASS_CONTRACT_UNBOUND = "statement_projection_contract_unbound"
FAILURE_CLASS_CONTRACT_VIOLATION = "statement_projection_contract_violation"
FAILURE_CLASS_AUTHORITY_VIOLATION = "statement_projection_authority_violation"
FAILURE_CLASS_BINDING_MISSING = "statement_projection_binding_missing"
FAILURE_CLASS_DIGEST_DRIFT = "statement_projection_digest_drift"


def parse_args(repo_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate projection-only statement-index/binding lane invariants."
    )
    parser.add_argument(
        "--statement-index",
        type=Path,
        default=repo_root / "artifacts" / "conformance" / "statement-index" / "latest.json",
        help="Statement-index artifact path",
    )
    parser.add_argument(
        "--bindings",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "KERNEL-STATEMENT-BINDINGS.json",
        help="Statement binding contract path",
    )
    parser.add_argument(
        "--kernel-doc",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "PREMATH-KERNEL.md",
        help="Kernel markdown path for fallback statement-index generation",
    )
    parser.add_argument(
        "--bd-module",
        type=Path,
        default=repo_root / "crates" / "premath-bd" / "src" / "spec_ir.rs",
        help="premath-bd spec IR module path",
    )
    parser.add_argument(
        "--bd-lib",
        type=Path,
        default=repo_root / "crates" / "premath-bd" / "src" / "lib.rs",
        help="premath-bd lib module path",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit deterministic JSON output",
    )
    return parser.parse_args()


def _load_json(path: Path) -> Dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: root must be an object")
    return payload


def load_or_build_statement_index(
    *,
    statement_index_path: Path,
    kernel_doc_path: Path,
) -> Dict[str, Any]:
    if statement_index_path.exists():
        return _load_json(statement_index_path)

    markdown_text = kernel_doc_path.read_text(encoding="utf-8")
    source_digest = hashlib.sha256(markdown_text.encode("utf-8")).hexdigest()
    generated = check_statement_index.evaluate_statement_index(
        markdown_text=markdown_text,
        doc_path="specs/premath/draft/PREMATH-KERNEL.md",
        source_digest=source_digest,
        extractor_digest="statement-projection-lane.fallback.v1",
    )
    if generated.get("result") != "accepted":
        raise ValueError(
            "fallback statement-index generation rejected: "
            f"{generated.get('failureClasses', [])}"
        )
    return generated


def evaluate_projection_lane(
    *,
    statement_index: Dict[str, Any],
    bindings: Dict[str, Any],
    bd_module_path: Path,
    bd_lib_path: Path,
) -> Dict[str, Any]:
    failures: Set[str] = set()
    errors: List[str] = []

    if not bd_module_path.exists():
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append(f"missing premath-bd module: {bd_module_path}")
    if not bd_lib_path.exists():
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append(f"missing premath-bd lib: {bd_lib_path}")
    elif "pub mod spec_ir;" not in bd_lib_path.read_text(encoding="utf-8"):
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append("crates/premath-bd/src/lib.rs missing `pub mod spec_ir;`")

    rows = statement_index.get("rows")
    if not isinstance(rows, list) or not rows:
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append("statement-index rows must be a non-empty list")
        rows = []

    typed_authority = statement_index.get("typedAuthority")
    if not isinstance(typed_authority, dict):
        failures.add(FAILURE_CLASS_AUTHORITY_VIOLATION)
        errors.append("statement-index typedAuthority must be an object")
    else:
        if str(typed_authority.get("kind", "")).strip() != "kcir.statement.v1":
            failures.add(FAILURE_CLASS_AUTHORITY_VIOLATION)
            errors.append("typedAuthority.kind must equal `kcir.statement.v1`")
        if str(typed_authority.get("refField", "")).strip() != "kcirRef":
            failures.add(FAILURE_CLASS_AUTHORITY_VIOLATION)
            errors.append("typedAuthority.refField must equal `kcirRef`")

    compatibility_alias = statement_index.get("compatibilityAlias")
    if not isinstance(compatibility_alias, dict):
        failures.add(FAILURE_CLASS_AUTHORITY_VIOLATION)
        errors.append("statement-index compatibilityAlias must be an object")
    else:
        if str(compatibility_alias.get("role", "")).strip() != "projection_only":
            failures.add(FAILURE_CLASS_AUTHORITY_VIOLATION)
            errors.append("compatibilityAlias.role must equal `projection_only`")

    statements: Dict[str, Dict[str, Any]] = {}
    for idx, row in enumerate(rows):
        if not isinstance(row, dict):
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"statement-index rows[{idx}] must be an object")
            continue
        statement_id = str(row.get("statementId", "")).strip()
        digest = str(row.get("digest", "")).strip()
        kcir_ref = str(row.get("kcirRef", "")).strip()
        if not statement_id:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"statement-index rows[{idx}].statementId must be non-empty")
            continue
        if statement_id in statements:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"duplicate statementId in statement-index: {statement_id}")
            continue
        if not digest:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"statement-index rows[{idx}].digest must be non-empty")
        if not kcir_ref.startswith("kcir1_"):
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(
                f"statement-index rows[{idx}].kcirRef must start with `kcir1_`"
            )
        statements[statement_id] = row

    binding_rows = bindings.get("bindings")
    if not isinstance(binding_rows, list) or not binding_rows:
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append("binding contract bindings must be a non-empty list")
        binding_rows = []

    checked_bindings = 0
    for idx, row in enumerate(binding_rows):
        if not isinstance(row, dict):
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"bindings[{idx}] must be an object")
            continue
        statement_id = str(row.get("statementId", "")).strip()
        expected_digest = str(row.get("statementDigest", "")).strip()
        if not statement_id:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"bindings[{idx}].statementId must be non-empty")
            continue
        if not expected_digest:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"bindings[{idx}].statementDigest must be non-empty")
            continue
        statement_row = statements.get(statement_id)
        if statement_row is None:
            failures.add(FAILURE_CLASS_BINDING_MISSING)
            errors.append(f"bindings[{idx}] references unknown statementId {statement_id}")
            continue
        observed_digest = str(statement_row.get("digest", "")).strip()
        if observed_digest != expected_digest:
            failures.add(FAILURE_CLASS_DIGEST_DRIFT)
            errors.append(
                f"bindings[{idx}] digest drift for {statement_id}: expected "
                f"{expected_digest}, observed {observed_digest}"
            )
        checked_bindings += 1

    result = "accepted" if not failures else "rejected"
    return {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": result,
        "failureClasses": sorted(failures),
        "summary": {
            "statementRows": len(rows),
            "statementIds": len(statements),
            "bindingRows": len(binding_rows),
            "checkedBindings": checked_bindings,
            "errors": len(errors),
        },
        "errors": errors,
    }


def main(argv: Sequence[str] | None = None) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)

    try:
        statement_index = load_or_build_statement_index(
            statement_index_path=args.statement_index.resolve(),
            kernel_doc_path=args.kernel_doc.resolve(),
        )
        bindings = _load_json(args.bindings.resolve())
        payload = evaluate_projection_lane(
            statement_index=statement_index,
            bindings=bindings,
            bd_module_path=args.bd_module.resolve(),
            bd_lib_path=args.bd_lib.resolve(),
        )
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": SCHEMA,
            "checkKind": CHECK_KIND,
            "result": "rejected",
            "failureClasses": [FAILURE_CLASS_CONTRACT_UNBOUND],
            "summary": {
                "statementRows": 0,
                "statementIds": 0,
                "bindingRows": 0,
                "checkedBindings": 0,
                "errors": 1,
            },
            "errors": [str(exc)],
        }

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        if payload["result"] == "accepted":
            print(
                "[statement-projection-lane] OK "
                f"(statements={payload['summary']['statementIds']}, "
                f"bindings={payload['summary']['checkedBindings']})"
            )
        else:
            print(
                "[statement-projection-lane] FAIL "
                f"(errors={payload['summary']['errors']})"
            )
            for error in payload["errors"]:
                print(f"  - {error}")
    return 0 if payload["result"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main())
