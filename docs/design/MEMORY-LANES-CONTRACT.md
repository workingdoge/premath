# Memory Lanes Contract

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one operational memory model with explicit lane ownership so agent work
state stays coherent across CLI/MCP/CI/docs surfaces.

Principle:

- minimum canonical encoding,
- maximum derived expressiveness.

## 2. Canonical lane map

| Lane | Authority owner | Canonical substrate | Deterministic query/projection surface | Primary consumers |
| --- | --- | --- | --- | --- |
| issue graph lane | `premath-bd` core semantics (`issue/dep` operations) | `.premath/issues.jsonl` (and replay/event projections derived from it) | `premath issue list`, `premath issue ready`, `premath issue blocked`, `premath issue check`, `premath issue backend-status`; MCP `issue_list`, `issue_ready`, `issue_check`, `issue_backend_status` | harness boot/step planning, retry/escalation mutation paths, CI hygiene checks |
| operations lane | operator conventions and rollout evidence (non-semantic authority) | `.premath/OPERATIONS.md` | stable markdown row projection by UTC-date rows (`rg '^\| [0-9]{4}-[0-9]{2}-[0-9]{2} ' .premath/OPERATIONS.md`) plus section anchors | operators, governance audits, branch-policy/release operations |
| doctrine/decision lane | spec + policy authority | `specs/premath/*`, `specs/process/decision-log.md` | `mise run doctrine-check`, `mise run traceability-check`, `mise run docs-coherence-check`, deterministic decision-log section anchors | checker/coherence contract evolution, capability/lifecycle governance |

## 3. Lane glue rules

1. Issue rows carry working state and compact provenance refs only.
2. Operations entries carry execution evidence and should include related issue
   IDs (and decision IDs when applicable).
3. Doctrine/decision entries carry boundary/lifecycle decisions and must link to
   affected issue IDs and command surfaces.
4. No lane is allowed to self-authorize semantic admissibility outside checker +
   discharge + witness flows.

## 4. Write discipline

### 4.1 `.premath/issues.jsonl` (issue graph lane)

Write here:

- open/in-progress/blocked/closed work state,
- acceptance criteria + verification commands,
- concise notes with refs to operations evidence and decision/spec updates.

Do not write here:

- long command transcripts,
- rollout log tables,
- normative semantic claims that belong in spec/decision artifacts.

### 4.2 `.premath/OPERATIONS.md` (operations lane)

Write here:

- stable runbooks and hygiene conventions,
- rollout evidence rows (date, operation, issue linkage, URLs/artifact refs),
- short operational notes that help repeatability.

Do not write here:

- authoritative issue dependency state,
- semantic doctrine decisions,
- checker/Gate admissibility outcomes as authority claims.

### 4.3 `specs/*` + `decision-log.md` (doctrine/decision lane)

Write here:

- lifecycle/boundary decisions,
- normative contract changes and capability/lane constraints,
- deterministic references to executable checks.

Do not write here:

- per-run operational noise,
- mutable task state that belongs in issue memory.

## 5. Migration slice (from implied conventions in `AGENTS.md`)

1. Keep `AGENTS.md` as command-surface index and quick policy reminders.
2. Treat this document as canonical write-discipline contract for work memory.
3. Keep `.premath/OPERATIONS.md` evidence rows issue-linked and decision-linked
   where relevant.
4. Keep issue notes compact; move oversized historical note payloads to stable
   refs (`bd-129`).
5. Promote to normative spec only after typed operations-lane projection becomes
   a required machine interface.

## 6. Verification commands

- `cargo run --package premath-cli -- issue check --issues .premath/issues.jsonl --json`
- `mise run docs-coherence-check`
- `mise run ci-hygiene-check`
