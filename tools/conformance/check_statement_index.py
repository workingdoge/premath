#!/usr/bin/env python3
"""Build and validate deterministic kernel statement-index artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple

SCHEMA = 1
CHECK_KIND = "conformance.statement_index.v1"
ARTIFACT_KIND = "premath.statement_index.v1"
EXTRACTOR_ID = "statement-index.extractor.v1"
KCIR_STATEMENT_KIND = "kcir.statement.v1"
FAILURE_CLASS_DUPLICATE = "statement_index_duplicate_statement_id"
FAILURE_CLASS_MISSING_ROWS = "statement_index_missing_statement_rows"
FAILURE_CLASS_MISSING_CLASS = "statement_index_missing_statement_class"

REQUIRED_PREFIXES = ("DEF", "AX", "REQ", "REJ")
STATEMENT_PATTERN = re.compile(
    r"\[(KERNEL\.(DEF|AX|REQ|REJ)\.[A-Z0-9_]+(?:\.[0-9]+)+)\]"
)
HEADING_PATTERN = re.compile(r"^(#{1,6})\s+(.+?)\s*$")


def parse_args(repo_root: Path) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate and emit deterministic kernel statement-index artifact."
    )
    parser.add_argument(
        "--kernel-doc",
        type=Path,
        default=repo_root / "specs" / "premath" / "draft" / "PREMATH-KERNEL.md",
        help="Kernel markdown source path",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root / "artifacts" / "conformance" / "statement-index" / "latest.json",
        help="Artifact output path (written only on acceptance)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit deterministic JSON result",
    )
    return parser.parse_args()


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def stable_hash(value: Any) -> str:
    payload = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return sha256_text(payload)


def slugify_heading(value: str) -> str:
    lowered = value.strip().lower()
    lowered = re.sub(r"[^a-z0-9\\s-]", "", lowered)
    lowered = re.sub(r"\\s+", "-", lowered)
    lowered = re.sub(r"-{2,}", "-", lowered)
    return lowered.strip("-")


def extract_statement_block(lines: Sequence[str], index: int) -> str:
    block: List[str] = [lines[index].strip()]
    cursor = index + 1
    while cursor < len(lines):
        raw = lines[cursor]
        stripped = raw.strip()
        if not stripped:
            break
        if HEADING_PATTERN.match(stripped):
            break
        if STATEMENT_PATTERN.search(stripped):
            break
        if stripped.startswith("- [KERNEL."):
            break
        # Keep paragraph/list continuation text tied to the originating statement line.
        if raw.startswith("  ") or raw.startswith("\t") or not stripped.startswith("- "):
            block.append(stripped)
            cursor += 1
            continue
        break
    return " ".join(part for part in block if part).strip()


def _statement_type(prefix: str) -> str:
    return {
        "DEF": "Definition",
        "AX": "Axiom",
        "REQ": "Requirement",
        "REJ": "RejectionCriterion",
    }[prefix]


def compute_statement_kcir_ref(statement_core: Dict[str, Any]) -> str:
    payload = {
        "kind": KCIR_STATEMENT_KIND,
        "statement": statement_core,
    }
    return f"kcir1_{stable_hash(payload)}"


def evaluate_statement_index(
    *,
    markdown_text: str,
    doc_path: str,
    source_digest: str,
    extractor_digest: str,
) -> Dict[str, Any]:
    lines = markdown_text.splitlines()
    current_anchor = "top"
    rows: List[Dict[str, Any]] = []
    failure_classes: List[str] = []
    errors: List[str] = []
    seen_ids: Dict[str, int] = {}
    observed_prefixes: set[str] = set()

    for line_no, line in enumerate(lines, start=1):
        heading_match = HEADING_PATTERN.match(line.strip())
        if heading_match is not None:
            heading_title = heading_match.group(2)
            current_anchor = slugify_heading(heading_title) or "top"
        for match in STATEMENT_PATTERN.finditer(line):
            statement_id = match.group(1)
            prefix = match.group(2)
            observed_prefixes.add(prefix)
            if statement_id in seen_ids:
                failure_classes.append(FAILURE_CLASS_DUPLICATE)
                errors.append(
                    f"duplicate statement id {statement_id} at line {line_no} "
                    f"(first seen line {seen_ids[statement_id]})"
                )
                continue
            seen_ids[statement_id] = line_no
            statement_text = extract_statement_block(lines, line_no - 1)
            row_payload = {
                "statementId": statement_id,
                "docPath": doc_path,
                "anchor": current_anchor,
                "stmtType": _statement_type(prefix),
                "statementText": statement_text,
            }
            row_digest = stable_hash(row_payload)
            kcir_ref = compute_statement_kcir_ref(row_payload)
            row_payload["digest"] = row_digest
            row_payload["kcirRef"] = kcir_ref
            row_payload["line"] = line_no
            rows.append(row_payload)

    if not rows:
        failure_classes.append(FAILURE_CLASS_MISSING_ROWS)
        errors.append("no kernel statement rows were found")

    missing_prefixes = [prefix for prefix in REQUIRED_PREFIXES if prefix not in observed_prefixes]
    if missing_prefixes:
        failure_classes.append(FAILURE_CLASS_MISSING_CLASS)
        errors.append(
            "missing required statement classes: " + ", ".join(missing_prefixes)
        )

    rows_sorted = sorted(rows, key=lambda item: (item["statementId"], int(item["line"])))
    result = "accepted" if not failure_classes else "rejected"
    return {
        "schema": SCHEMA,
        "checkKind": CHECK_KIND,
        "artifactKind": ARTIFACT_KIND,
        "result": result,
        "failureClasses": sorted(set(failure_classes)),
        "errors": errors,
        "statementCount": len(rows_sorted),
        "rows": rows_sorted,
        "typedAuthority": {
            "kind": KCIR_STATEMENT_KIND,
            "refField": "kcirRef",
        },
        "compatibilityAlias": {
            "digestField": "digest",
            "role": "projection_only",
        },
        "lineage": {
            "sourceDigestField": "sourceDocuments[].sha256",
            "extractorDigestField": "extractor.sha256",
        },
        "sourceDocuments": [
            {
                "path": doc_path,
                "sha256": source_digest,
            }
        ],
        "extractor": {
            "id": EXTRACTOR_ID,
            "sha256": extractor_digest,
        },
    }


def _self_digest(script_path: Path) -> str:
    return sha256_bytes(script_path.read_bytes())


def evaluate_from_path(kernel_doc_path: Path) -> Dict[str, Any]:
    script_path = Path(__file__).resolve()
    repo_root = script_path.parents[2]
    markdown_text = kernel_doc_path.read_text(encoding="utf-8")
    source_digest = sha256_bytes(markdown_text.encode("utf-8"))
    try:
        doc_path = str(kernel_doc_path.relative_to(repo_root).as_posix())
    except ValueError:
        doc_path = str(kernel_doc_path.as_posix())
    extractor_digest = _self_digest(script_path)
    return evaluate_statement_index(
        markdown_text=markdown_text,
        doc_path=doc_path,
        source_digest=source_digest,
        extractor_digest=extractor_digest,
    )


def write_artifact(output_path: Path, payload: Dict[str, Any]) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv: Sequence[str]) -> int:
    repo_root = Path(__file__).resolve().parents[2]
    args = parse_args(repo_root)
    kernel_doc = args.kernel_doc.resolve()
    output_path = args.output.resolve()

    try:
        payload = evaluate_from_path(kernel_doc)
    except FileNotFoundError:
        err = {
            "schema": SCHEMA,
            "checkKind": CHECK_KIND,
            "result": "rejected",
            "failureClasses": [FAILURE_CLASS_MISSING_ROWS],
            "errors": [f"missing kernel doc: {kernel_doc}"],
        }
        if args.json:
            print(json.dumps(err, indent=2, ensure_ascii=False))
        else:
            print(f"[statement-index] FAIL (missing kernel doc: {kernel_doc})")
        return 1

    if payload["result"] == "accepted":
        write_artifact(output_path, payload)

    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    else:
        if payload["result"] == "accepted":
            print(
                "[statement-index] OK "
                f"(rows={payload['statementCount']}, output={output_path})"
            )
        else:
            print(
                "[statement-index] FAIL "
                f"(failureClasses={payload['failureClasses']}, errors={len(payload['errors'])})"
            )
            for err in payload["errors"]:
                print(f"  - {err}")
    return 0 if payload["result"] == "accepted" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
