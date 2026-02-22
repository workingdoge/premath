# jsonl Directory

This directory is reserved for JSONL datasets used for local experimentation,
fixtures, or generated projection artifacts.

Rules:

- Do not treat this directory as canonical source-of-truth data.
- Keep normative fixtures under `tests/` and specs under `specs/`.
- Avoid checking in large transient runtime dumps.

Canonical issue-memory input for CLI workflows remains `.premath/issues.jsonl`
(or explicit `--issues <path>`).
