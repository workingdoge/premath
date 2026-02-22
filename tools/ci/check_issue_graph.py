#!/usr/bin/env python3
"""Validate deterministic issue-graph contract invariants."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any, Dict, List, Sequence, Tuple


ACTIVE_STATUSES = {"open", "in_progress"}
EPIC_TITLE_PREFIX = "[EPIC]"
DEFAULT_NOTE_WARN_THRESHOLD = 2000
ACCEPTANCE_SECTION_RE = re.compile(r"(?im)^\s*acceptance(?:\s+criteria)?\s*:")
COMMAND_LINE_RE = re.compile(
    r"(?im)^\s*(?:[-*]\s*)?(?:`)?"
    r"(?:mise run|python3|cargo(?: run)?|premath|sh|nix develop -c|uv run|pytest)\b"
    r"[^`\n]*(?:`)?\s*$"
)
COMMAND_INLINE_RE = re.compile(
    r"`(?:mise run|python3|cargo(?: run)?|premath|sh|nix develop -c|uv run|pytest)\b[^`]*`",
    re.IGNORECASE,
)


def _norm_str(value: Any) -> str:
    if not isinstance(value, str):
        return ""
    return value.strip()


def _norm_type(value: Any) -> str:
    return _norm_str(value).lower()


def is_epic_title(title: str) -> bool:
    return title.startswith(EPIC_TITLE_PREFIX)


def has_acceptance_section(description: str) -> bool:
    return bool(ACCEPTANCE_SECTION_RE.search(description))


def has_verification_command(text: str) -> bool:
    return bool(COMMAND_LINE_RE.search(text) or COMMAND_INLINE_RE.search(text))


def parse_jsonl(path: Path) -> List[Dict[str, Any]]:
    rows: List[Dict[str, Any]] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{line_no}: invalid json ({exc.msg})") from exc
        if not isinstance(payload, dict):
            raise ValueError(f"{path}:{line_no}: row must be an object")
        payload["_line"] = line_no
        rows.append(payload)
    return rows


def evaluate_issue_graph(
    rows: Sequence[Dict[str, Any]],
    note_warn_threshold: int = DEFAULT_NOTE_WARN_THRESHOLD,
) -> Tuple[List[str], List[str]]:
    errors: List[str] = []
    warnings: List[str] = []
    for issue in rows:
        issue_id = _norm_str(issue.get("id")) or "<unknown-id>"
        line_no = issue.get("_line", "?")
        title = _norm_str(issue.get("title"))
        issue_type = _norm_type(issue.get("issue_type"))
        status = _norm_type(issue.get("status"))
        is_ephemeral = bool(issue.get("ephemeral", False))
        description = _norm_str(issue.get("description"))
        notes = _norm_str(issue.get("notes"))
        combined_text = "\n".join(part for part in (description, notes) if part)

        if is_epic_title(title) and issue_type != "epic":
            errors.append(
                f"{issue_id}@L{line_no}: epic_issue_type_mismatch "
                f"(title starts with [EPIC] but issue_type={issue_type or '<empty>'})"
            )

        if status in ACTIVE_STATUSES and not is_ephemeral:
            if not has_acceptance_section(description):
                errors.append(
                    f"{issue_id}@L{line_no}: issue_acceptance_missing "
                    "(active issue must include an Acceptance section)"
                )
            if not has_verification_command(combined_text):
                errors.append(
                    f"{issue_id}@L{line_no}: issue_verification_command_missing "
                    "(active issue must include at least one verification command)"
                )

        if len(notes) > note_warn_threshold:
            warnings.append(
                f"{issue_id}@L{line_no}: issue_notes_large "
                f"(notes_len={len(notes)} threshold={note_warn_threshold})"
            )

    return errors, warnings


def parse_args() -> argparse.Namespace:
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description="Check issue-graph contract invariants for .premath/issues.jsonl."
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=root,
        help=f"Repository root (default: {root})",
    )
    parser.add_argument(
        "--issues",
        type=Path,
        default=Path(".premath/issues.jsonl"),
        help="Issue graph JSONL path relative to --repo-root (default: .premath/issues.jsonl).",
    )
    parser.add_argument(
        "--note-warn-threshold",
        type=int,
        default=DEFAULT_NOTE_WARN_THRESHOLD,
        help=f"Warning threshold for issue note length (default: {DEFAULT_NOTE_WARN_THRESHOLD}).",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.repo_root.resolve()
    issues_path = (root / args.issues).resolve()
    if args.note_warn_threshold < 0:
        print("[issue-graph] FAIL (invalid --note-warn-threshold: must be >= 0)")
        return 1

    try:
        rows = parse_jsonl(issues_path)
    except Exception as exc:  # fail-closed by default
        print(f"[issue-graph] FAIL ({exc})")
        return 1

    errors, warnings = evaluate_issue_graph(rows, note_warn_threshold=args.note_warn_threshold)

    if errors:
        print(
            f"[issue-graph] FAIL (issues={len(rows)}, errors={len(errors)}, warnings={len(warnings)})"
        )
        for row in errors:
            print(f"  - {row}")
        for row in warnings:
            print(f"  - WARN {row}")
        return 1

    if warnings:
        print(
            f"[issue-graph] OK-WARN (issues={len(rows)}, errors=0, warnings={len(warnings)})"
        )
        for row in warnings:
            print(f"  - WARN {row}")
        return 0

    print(f"[issue-graph] OK (issues={len(rows)}, errors=0, warnings=0)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
