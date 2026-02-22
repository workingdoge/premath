# Tusk Harness Feature Ledger

Status: draft
Scope: design-level, non-normative

## 1. Purpose

`HarnessFeatureLedger` is the typed progress lane for bounded harness work.

- one row per feature/work-slice,
- deterministic next-feature selection,
- explicit closure check for stop/boot discipline.

The ledger is operational memory only; witness/checker artifacts remain semantic
authority.

## 2. Canonical Artifact

- default path: `.premath/harness_feature_ledger.json`
- kind: `premath.harness.feature_ledger.v1`
- schema: `1`

## 3. Schema

Top-level:

- `schema: 1`
- `ledgerKind: "premath.harness.feature_ledger.v1"`
- `updatedAt: RFC3339`
- `sessionRef?: string`
- `features: FeatureRow[]`

Feature row:

- `featureId: string` (unique)
- `status: "pending" | "in_progress" | "blocked" | "completed"`
- `updatedAt: RFC3339`
- `issueId?: string`
- `summary?: string`
- `instructionRefs?: string[]`
- `verificationRefs?: string[]`

## 4. Deterministic Checks

`harness-feature check` validates:

- schema/kind header,
- non-empty + unique `featureId`,
- valid status domain,
- at most one `in_progress` feature,
- non-empty ref entries,
- `completed` rows require at least one `verificationRef`.

`--require-closure` adds a fail-closed closure condition:

- all rows must be `completed` with required verification refs.

## 5. Next-Feature Rule

`harness-feature next` computes deterministic next work:

1. lexicographically smallest `in_progress` feature, else
2. lexicographically smallest `pending` feature, else
3. no next feature (`null`).

`harness-session bootstrap` projects this result to include:

- `nextFeatureId`
- `featureClosureComplete`
- `featureCount`

## 6. Command Surface

- `premath harness-feature write --feature-id <id> --status <status> ... --json`
- `premath harness-feature read --path .premath/harness_feature_ledger.json --json`
- `premath harness-feature check --path .premath/harness_feature_ledger.json --require-closure --json`
- `premath harness-feature next --path .premath/harness_feature_ledger.json --json`

## 7. Related Docs

- `docs/design/TUSK-HARNESS-CONTRACT.md`
- `docs/design/TUSK-HARNESS-SESSION.md`
