#!/usr/bin/env python3
"""Validate statement-index bindings to obligations/checkers/vectors."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Set

SCHEMA = 1
CHECK_KIND = "conformance.statement_binding.v1"
FAILURE_CLASS_CONTRACT_VIOLATION = "statement_binding_contract_violation"
FAILURE_CLASS_CONTRACT_UNBOUND = "statement_binding_contract_unbound"
FAILURE_CLASS_DIGEST_DRIFT = "statement_binding_digest_drift"
FAILURE_CLASS_REQUIRED_MISSING = "statement_binding_required_missing"
TARGET_KINDS = {"obligation", "checker", "vector"}


def parse_args(repo_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate typed statement bindings.")
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
        help="Typed binding contract path",
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


def _as_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label} must be a non-empty string")
    return value.strip()


def _as_bool(value: Any, *, default: bool = False) -> bool:
    if value is None:
        return default
    if isinstance(value, bool):
        return value
    raise ValueError("required must be boolean when provided")


def _as_string_list(value: Any, label: str) -> List[str]:
    if not isinstance(value, list):
        raise ValueError(f"{label} must be a list")
    out: List[str] = []
    for idx, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise ValueError(f"{label}[{idx}] must be a non-empty string")
        out.append(item.strip())
    return out


def _load_obligation_ids(repo_root: Path) -> Set[str]:
    out: Set[str] = set()

    contract_path = repo_root / "specs" / "premath" / "draft" / "COHERENCE-CONTRACT.json"
    payload = _load_json(contract_path)
    obligations = payload.get("obligations")
    if isinstance(obligations, list):
        for row in obligations:
            if isinstance(row, dict):
                raw = row.get("id")
                if isinstance(raw, str) and raw.strip():
                    out.add(raw.strip())

    control_plane_path = repo_root / "specs" / "premath" / "draft" / "CONTROL-PLANE-CONTRACT.json"
    control_plane = _load_json(control_plane_path)
    stage2 = control_plane.get("evidenceStage2Authority")
    if isinstance(stage2, dict):
        route = stage2.get("bidirEvidenceRoute")
        if isinstance(route, dict):
            required = route.get("requiredObligations")
            if isinstance(required, list):
                for raw in required:
                    if isinstance(raw, str) and raw.strip():
                        out.add(raw.strip())
    return out


def evaluate_statement_bindings(
    *,
    statement_index: Dict[str, Any],
    binding_contract: Dict[str, Any],
    repo_root: Path,
) -> Dict[str, Any]:
    failures: Set[str] = set()
    errors: List[str] = []
    rows_out: List[Dict[str, Any]] = []
    obligation_ids = _load_obligation_ids(repo_root)

    index_rows = statement_index.get("rows")
    if not isinstance(index_rows, list):
        failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        errors.append("statement-index rows must be a list")
        index_rows = []
    index_by_id: Dict[str, Dict[str, Any]] = {}
    for idx, row in enumerate(index_rows):
        if not isinstance(row, dict):
            failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
            errors.append(f"statement-index rows[{idx}] must be an object")
            continue
        statement_id = row.get("statementId")
        digest = row.get("digest")
        if not isinstance(statement_id, str) or not statement_id.strip():
            failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
            errors.append(f"statement-index rows[{idx}].statementId must be non-empty")
            continue
        if not isinstance(digest, str) or not digest.strip():
            failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
            errors.append(f"statement-index rows[{idx}].digest must be non-empty")
            continue
        index_by_id[statement_id.strip()] = row

    relation_kinds = _as_string_list(binding_contract.get("relationKinds", []), "bindings.relationKinds")
    relation_kind_set = set(relation_kinds)
    required_statement_ids = _as_string_list(
        binding_contract.get("requiredStatementIds", []),
        "bindings.requiredStatementIds",
    )

    bindings = binding_contract.get("bindings")
    if not isinstance(bindings, list):
        failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
        errors.append("bindings.bindings must be a list")
        bindings = []

    required_seen: Set[str] = set()
    for idx, row in enumerate(bindings):
        row_errors: List[str] = []
        if not isinstance(row, dict):
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(f"bindings.bindings[{idx}] must be an object")
            continue
        try:
            statement_id = _as_string(row.get("statementId"), f"bindings.bindings[{idx}].statementId")
            expected_digest = _as_string(row.get("statementDigest"), f"bindings.bindings[{idx}].statementDigest")
            relation_kind = _as_string(row.get("relationKind"), f"bindings.bindings[{idx}].relationKind")
            target_kind = _as_string(row.get("targetKind"), f"bindings.bindings[{idx}].targetKind")
            target_ref = _as_string(row.get("targetRef"), f"bindings.bindings[{idx}].targetRef")
            required = _as_bool(row.get("required"), default=False)
        except ValueError as exc:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            errors.append(str(exc))
            continue

        if relation_kind not in relation_kind_set:
            row_errors.append(f"unknown relationKind `{relation_kind}`")
        if target_kind not in TARGET_KINDS:
            row_errors.append(f"unknown targetKind `{target_kind}`")

        if statement_id not in index_by_id:
            row_errors.append(f"statement `{statement_id}` not found in statement-index")
            failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        else:
            observed_digest = str(index_by_id[statement_id].get("digest", "")).strip()
            if observed_digest != expected_digest:
                row_errors.append(
                    f"digest drift for `{statement_id}` (expected {expected_digest}, got {observed_digest})"
                )
                failures.add(FAILURE_CLASS_DIGEST_DRIFT)

        if target_kind in {"checker", "vector"}:
            target_path = (repo_root / target_ref).resolve()
            if not target_path.exists():
                row_errors.append(f"target path does not exist: {target_ref}")
                failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)
        if target_kind == "obligation" and obligation_ids and target_ref not in obligation_ids:
            row_errors.append(
                f"obligation `{target_ref}` not declared in known obligation registries"
            )
            failures.add(FAILURE_CLASS_CONTRACT_UNBOUND)

        status = "ok"
        if row_errors:
            failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
            status = "invalid"
            errors.extend([f"binding[{idx}]: {msg}" for msg in row_errors])
        if required:
            required_seen.add(statement_id)
        rows_out.append(
            {
                "index": idx,
                "statementId": statement_id,
                "required": required,
                "status": status,
                "errors": row_errors,
            }
        )

    missing_required = sorted(set(required_statement_ids) - required_seen)
    if missing_required:
        failures.add(FAILURE_CLASS_REQUIRED_MISSING)
        failures.add(FAILURE_CLASS_CONTRACT_VIOLATION)
        errors.append(
            "requiredStatementIds missing required bindings: " + ", ".join(missing_required)
        )

    result = "accepted" if not failures else "rejected"
    return {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "result": result,
        "failureClasses": sorted(failures),
        "errors": errors,
        "bindingRows": rows_out,
    }


def main(argv: Sequence[str]) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    try:
        statement_index = _load_json(args.statement_index.resolve())
        binding_contract = _load_json(args.bindings.resolve())
        output = evaluate_statement_bindings(
            statement_index=statement_index,
            binding_contract=binding_contract,
            repo_root=repo_root,
        )
    except Exception as exc:  # noqa: BLE001
        payload = {
            "schema": SCHEMA,
            "checkKind": CHECK_KIND,
            "result": "rejected",
            "failureClasses": [FAILURE_CLASS_CONTRACT_UNBOUND],
            "errors": [str(exc)],
            "bindingRows": [],
        }
        if args.json:
            print(json.dumps(payload, indent=2, ensure_ascii=False))
        else:
            print(f"[statement-bindings] FAIL (error={exc})")
        return 1

    if args.json:
        print(json.dumps(output, indent=2, ensure_ascii=False))
    else:
        if output["result"] == "accepted":
            print(
                "[statement-bindings] OK "
                f"(rows={len(output['bindingRows'])})"
            )
        else:
            print(
                "[statement-bindings] FAIL "
                f"(failureClasses={output['failureClasses']}, errors={len(output['errors'])})"
            )
            for err in output["errors"]:
                print(f"  - {err}")
    return 0 if output["result"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
