# Tusk Identity

Status: draft
Scope: design-level, non-normative

## 1. Purpose

This document defines run identity and determinism boundaries for Tusk.

Primary goal: prevent semantic/control-plane leakage by making identity material explicit.

## 2. Reference split

Do not overload one `head_ref` for multiple planes.

- `context_id`: stable key for context object `Gamma` in world `C`.
- `data_head_ref`: canonical EventStore head (append-only substrate position).
- `ctx_ref`: context/version pointer from `ContextProvider` (JJ/Git/other lineage).

`head_ref` without qualifier should be avoided in new interfaces.

`context_id` identifies *which* context object is being interpreted.
`ctx_ref` identifies *which lineage state* is used to interpret that context.

## 3. Run base

A run opens against:

`RunBase = (world_id, context_id, ctx_ref, data_head_ref)`

Lifecycle rule:

- Open chooses `RunBase`.
- Run appends events and advances `data_head_ref`.
- Close may emit a new `ctx_ref` if context/version outputs are produced, but this is optional and explicit.

## 4. Canonical run identity fields

A run identity should include at least:

- `world_id`
- `unit_id`
- `parent_unit_id` (optional)
- `context_id`
- `intent_id`
- `cover_id`
- `ctx_ref`
- `data_head_ref`
- `adapter_id`
- `adapter_version`
- `policy_digest`
- `normalizer_id`

Derived artifacts (stdout, caches, view rows) are not identity material.

`cover_strategy_digest` is audit material by default.
It becomes identity material only under explicit hardening policy.

## 5. Deterministic IDs

Deterministic IDs should be computed from canonical serialization of identity material.

Examples:

- `run_id = digest(canonical(run_identity_fields))`
- `cover_id = digest(canonical(cover_identity_fields))`

`intent_id` must be stable:

- `intent_id = digest(canonical(IntentSpec))`
- natural-language prompts are not identity material
- NL inputs may compile to `IntentSpec`, but only canonical `IntentSpec` is hashed

Minimum `IntentSpec` fields should include:

- `intent_kind`
- `target_scope`
- `requested_outcomes`
- `constraints` (optional)

Hash function, canonicalization, and versioning should be explicit and stable.

## 6. Semantic binding rule

`normalizer_id` and `policy_digest` are semantic bindings, not operational metadata.

Include in `policy_digest` any parameter that can change:

- acceptance/rejection,
- equality/equivalence comparisons,
- contractibility outcomes.

Exclude from `policy_digest` any parameter that only changes:

- scheduling order,
- retry timing,
- queue selection.

`intent_id` should remain separate from `policy_digest`.

## 7. Refinement identity discipline

A refinement step is a morphism between run identities that changes exactly one axis:

- `cover_id`
- `ctx_ref`
- policy binding (`policy_digest` and/or `normalizer_id`)
- adapter binding (`adapter_version`)

`intent_id` changes are usually new-run boundaries, not refinement steps.

Each refinement record should carry:

- `parent_run_id`
- `refinement_axis`

This supports deterministic refinement ladders and comparable witnesses.

## 8. Backward compatibility note

Existing v0 payloads that carry only `head_ref` should be interpreted as:

- `head_ref == data_head_ref`

when no explicit `ctx_ref` is present.
