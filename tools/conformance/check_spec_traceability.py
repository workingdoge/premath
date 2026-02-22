#!/usr/bin/env python3
"""Validate draft spec traceability matrix integrity."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple

VALID_STATUS = {"covered", "instrumented", "gap"}
GAP_TARGET_RE = re.compile(r"^T-[A-Z]+-\d+$")
PIPE_SPLIT_RE = re.compile(r"\s*\|\s*")
CODE_REF_RE = re.compile(r"`([^`]+)`")


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description="Validate specs/premath/draft/SPEC-TRACEABILITY integrity.")
    parser.add_argument(
        "--draft-dir",
        type=Path,
        default=root / "specs" / "premath" / "draft",
        help=f"Draft spec directory (default: {root / 'specs' / 'premath' / 'draft'})",
    )
    parser.add_argument(
        "--matrix",
        type=Path,
        default=root / "specs" / "premath" / "draft" / "SPEC-TRACEABILITY.md",
        help=(
            "Traceability matrix markdown path "
            f"(default: {root / 'specs' / 'premath' / 'draft' / 'SPEC-TRACEABILITY.md'})"
        ),
    )
    return parser.parse_args()


def _extract_frontmatter_status(path: Path) -> str | None:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return None
    try:
        _, rest = text.split("---\n", 1)
        frontmatter, _ = rest.split("---\n", 1)
    except ValueError:
        return None
    for raw in frontmatter.splitlines():
        line = raw.strip()
        if line.startswith("status:"):
            _, value = line.split(":", 1)
            return value.strip()
    return None


def promoted_draft_specs(draft_dir: Path) -> List[str]:
    promoted: List[str] = []
    for path in sorted(draft_dir.iterdir()):
        if path.is_dir():
            continue
        name = path.name
        if name == "README.md":
            continue
        if path.suffix == ".md":
            if _extract_frontmatter_status(path) == "draft":
                promoted.append(name)
            continue
        if path.suffix == ".json":
            promoted.append(name)
    return promoted


def _strip_cell(cell: str) -> str:
    return cell.strip().strip("\u200b")


def _parse_table_rows(lines: Sequence[str], matrix_path: Path) -> List[Tuple[str, str, str, str]]:
    rows: List[Tuple[str, str, str, str]] = []
    in_matrix = False
    in_table = False
    for raw in lines:
        line = raw.rstrip("\n")
        if line.startswith("## 3. Traceability Matrix"):
            in_matrix = True
            continue
        if in_matrix and line.startswith("## "):
            break
        if not in_matrix:
            continue
        if line.lstrip().startswith("|"):
            in_table = True
        if not in_table:
            continue
        stripped = line.strip()
        if not stripped:
            continue
        if re.match(r"^\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|$", stripped):
            continue
        if not stripped.startswith("|"):
            continue
        parts = PIPE_SPLIT_RE.split(stripped.strip("|"))
        if len(parts) != 4:
            raise ValueError(f"{matrix_path}: malformed matrix row: {line}")
        spec_cell, surface_cell, status_cell, target_cell = (_strip_cell(p) for p in parts)
        if spec_cell == "Draft spec":
            continue
        spec_match = CODE_REF_RE.search(spec_cell)
        if spec_match is None:
            raise ValueError(f"{matrix_path}: first column must contain backticked spec name: {line}")
        spec_name = spec_match.group(1).strip()
        rows.append((spec_name, surface_cell, status_cell, target_cell))
    return rows


def validate_matrix(
    draft_specs: Sequence[str],
    rows: Sequence[Tuple[str, str, str, str]],
) -> List[str]:
    errors: List[str] = []
    draft_set = set(draft_specs)
    row_map: Dict[str, List[Tuple[str, str, str, str]]] = {}

    for row in rows:
        spec_name, _, status, target = row
        row_map.setdefault(spec_name, []).append(row)
        if status not in VALID_STATUS:
            errors.append(f"invalid status for {spec_name!r}: {status!r}")
        if status == "gap":
            if not GAP_TARGET_RE.match(target):
                errors.append(f"gap row for {spec_name!r} must use target ID T-*-*: got {target!r}")
        if spec_name not in draft_set:
            errors.append(f"matrix row references unknown draft spec: {spec_name!r}")

    for spec in sorted(draft_set):
        count = len(row_map.get(spec, []))
        if count == 0:
            errors.append(f"promoted draft spec missing from matrix: {spec!r}")
        elif count > 1:
            errors.append(f"promoted draft spec appears multiple times in matrix: {spec!r} ({count} rows)")

    return errors


def main() -> int:
    args = parse_args()
    draft_dir = args.draft_dir.resolve()
    matrix_path = args.matrix.resolve()

    if not draft_dir.exists() or not draft_dir.is_dir():
        print(f"[traceability-check] ERROR: draft directory missing: {draft_dir}")
        return 2
    if not matrix_path.exists():
        print(f"[traceability-check] ERROR: matrix file missing: {matrix_path}")
        return 2

    draft_specs = promoted_draft_specs(draft_dir)
    lines = matrix_path.read_text(encoding="utf-8").splitlines()
    rows = _parse_table_rows(lines, matrix_path)
    errors = validate_matrix(draft_specs, rows)

    if errors:
        print(f"[traceability-check] FAIL (draftSpecs={len(draft_specs)}, matrixRows={len(rows)}, errors={len(errors)})")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"[traceability-check] OK (draftSpecs={len(draft_specs)}, matrixRows={len(rows)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
