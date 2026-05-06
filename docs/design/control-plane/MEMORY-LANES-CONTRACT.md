# Memory Lanes Contract

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one operational memory model with explicit lane ownership so agent work
state stays coherent across checker/docs surfaces.

Topology note:

- `ATLAS.md` is the site-of-sites staging note for cross-repo covers and
  authority boundaries.
- `TOPOLOGY-V2.md` is the shape-first placement companion for this contract.
- This document describes the current operational memory lanes.
- It does not make Premath the owner of graph-shaped tracker memory or future
  simplex-native tracker work.

Principle:

- minimum canonical encoding,
- maximum derived expressiveness.

## 2. Canonical lane map

| Lane | Authority owner | Canonical substrate | Deterministic query/projection surface | Primary consumers |
| --- | --- | --- | --- | --- |
| tracker lane | Tusk/downstream tracker semantics | external tracker substrate | external tracker projection plus `premath work-tracker-check` at the Premath boundary | work readiness, dependency review, operator scheduling |
| operations lane | operator conventions and rollout evidence (non-semantic authority) | `.premath/OPERATIONS.md` | stable markdown row projection by UTC-date rows (`rg '^\| [0-9]{4}-[0-9]{2}-[0-9]{2} ' .premath/OPERATIONS.md`) plus section anchors | operators, governance audits, release operations |
| doctrine/decision lane | spec + policy authority | `specs/premath/*`, `specs/process/decision-log.md` | `premath traceability-check`, `premath coherence-check`, `premath drift-budget-check`, deterministic decision-log section anchors | checker/coherence contract evolution, capability/lifecycle governance |

## 3. Lane glue rules

1. Tracker rows carry working state and compact provenance refs in the owning
   Tusk/downstream tracker.
2. Operations entries carry execution evidence and should include tracker refs
   and decision IDs when applicable.
3. Doctrine/decision entries carry boundary/lifecycle decisions and must link to
   affected issue IDs and command surfaces.
4. No lane is allowed to self-authorize semantic admissibility outside checker +
   discharge + witness flows.

## 4. Write discipline

### 4.1 External Tracker Lane

Write in the owning tracker:

- open/in-progress/blocked/closed work state,
- acceptance criteria + verification commands,
- concise notes with refs to operations evidence and decision/spec updates.

Do not put in Premath specs or operations notes:

- long command transcripts,
- rollout log tables,
- normative semantic claims that belong in spec/decision artifacts.

### 4.2 `.premath/OPERATIONS.md` (operations lane)

Write here:

- stable runbooks and hygiene conventions,
- rollout evidence rows (date, operation, issue linkage, URLs/artifact refs),
- short operational notes that help repeatability.

Do not write here:

- authoritative tracker dependency state,
- semantic doctrine decisions,
- checker/Gate admissibility outcomes as authority claims.

### 4.3 `specs/*` + `decision-log.md` (doctrine/decision lane)

Write here:

- lifecycle/boundary decisions,
- normative contract changes and capability/lane constraints,
- deterministic references to executable checks.

Do not write here:

- per-run operational noise,
- mutable task state that belongs in the owning tracker.

## 5. Migration slice (from implied conventions in `AGENTS.md`)

1. Keep `AGENTS.md` as command-surface index and quick policy reminders.
2. Treat this document as canonical write-discipline contract for work memory.
3. Keep `.premath/OPERATIONS.md` evidence rows tracker-linked and decision-linked
   where relevant.
4. Keep tracker notes compact; move oversized historical note payloads to stable
   refs in the owning tracker.
5. Promote to normative spec only after typed operations-lane projection becomes
   a required machine interface.

## 6. Verification commands

- `premath drift-budget-check`
- `premath repo-hygiene-check`
