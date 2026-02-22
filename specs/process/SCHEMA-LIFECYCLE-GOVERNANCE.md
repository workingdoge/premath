# Schema Lifecycle Governance

Status: draft
Scope: process-level governance contract

## 1. Purpose

Define governance semantics for control-plane schema lifecycle transitions so
rollover and freeze states are explicit, auditable, and reproducible.

Normative bindings:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md` §5.1
- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json` (`schemaLifecycle`)
- `specs/process/decision-log.md`

## 2. Governance Shape

`schemaLifecycle` MUST include:

```json
{
  "activeEpoch": "YYYY-MM",
  "governance": {
    "mode": "rollover | freeze",
    "decisionRef": "decision-XXXX",
    "owner": "<non-empty>",
    "rolloverCadenceMonths": "<required in rollover mode>",
    "freezeReason": "<required in freeze mode>"
  },
  "kindFamilies": { "...": "..." }
}
```

`decisionRef` and `owner` are required accountability fields for all modes.

## 3. Mode Semantics

### 3.1 `rollover`

Required:

1. at least one compatibility alias exists in `kindFamilies`,
2. one shared `supportUntilEpoch` across aliases,
3. runway (`supportUntilEpoch - activeEpoch`) is positive,
4. `rolloverCadenceMonths` is present and within `1..12`,
5. runway does not exceed `rolloverCadenceMonths`.

Forbidden:

- `freezeReason`.

### 3.2 `freeze`

Required:

1. no active compatibility aliases in `kindFamilies`,
2. `freezeReason` is present and non-empty.

Forbidden:

- `rolloverCadenceMonths`.

## 4. Decision and Audit Contract

For every governance transition (`rollover <-> freeze`) or cadence change:

1. append a decision-log entry with:
   - transition reason,
   - affected contract path(s),
   - verification commands.
2. update `schemaLifecycle.governance.decisionRef` to that decision ID.
3. keep issue linkage in `.premath/issues.jsonl` notes for operational context.

## 5. Reproducible Operator Flow

1. edit `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`,
2. run:
   - `mise run ci-drift-budget-check`
   - `mise run coherence-check`
   - `mise run docs-coherence-check`
3. append decision-log entry and update `decisionRef`,
4. record rollout evidence in `.premath/OPERATIONS.md` when a live system
   transition is executed.
