# Harness Spec Promotion Map (Preflight)

Status: preflight
Scope: parallel-safe planning artifact for `bd-192`
Authority: non-normative (no semantic delta)

## 1. Goal

Create a mechanical promotion path from harness design docs to draft spec surfaces
with minimum encoding and maximum expressiveness.

This document does not change behavior. It only fixes ordering and mapping so
`bd-192` can land with low merge risk after `bd-190`.

## 2. Source -> Target Mapping

Canonical source set (`docs/design`):

- `TUSK-HARNESS-CONTRACT.md`
- `TUSK-HARNESS-SESSION.md`
- `TUSK-HARNESS-FEATURE-LEDGER.md`
- `TUSK-HARNESS-TRAJECTORY.md`
- `TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- `TUSK-HARNESS-RETRY-POLICY.md`

Canonical target set (`specs/premath/draft`, proposed for `bd-192`):

- `HARNESS-RUNTIME.md`
  - includes: boot/step/stop contract, session artifact, feature ledger, trajectory rows, multithread worker/coordinator loop.
- `HARNESS-RETRY-ESCALATION.md`
  - includes: retry-class table, escalation mapping, active-issue context fallback order, fail-closed classes.

Reason for two targets:

- one runtime contract surface,
- one retry/escalation control surface,
- avoids N-way drift while keeping command-level traceability explicit.

## 3. Command Surface Anchors

`HARNESS-RUNTIME.md` MUST anchor:

- `premath harness-session read|write|bootstrap`
- `premath harness-feature read|write|check|next`
- `premath harness-trajectory append|query`
- `python3 tools/harness/multithread_loop.py worker|coordinator`
- `mise run harness-worker-loop`
- `mise run harness-coordinator-loop`

`HARNESS-RETRY-ESCALATION.md` MUST anchor:

- `policies/control/harness-retry-policy-v1.json`
- `tools/ci/harness_retry_policy.py`
- `tools/ci/harness_escalation.py`
- wrapper call sites:
  - `tools/ci/pipeline_required.py`
  - `tools/ci/pipeline_instruction.py`

## 4. Doctrine-Site Routing Anchors

Promotion text MUST reference existing routed operations (no new ops in `bd-192`):

- `op/harness.session_read`
- `op/harness.session_write`
- `op/harness.session_bootstrap`
- `op/mcp.issue_claim`
- `op/mcp.issue_lease_renew`
- `op/mcp.issue_lease_release`
- `op/mcp.issue_discover`
- `op/mcp.issue_lease_projection`
- `op/mcp.dep_diagnostics`

Source of truth:

- `specs/premath/draft/DOCTRINE-OP-REGISTRY.json`
- `specs/premath/draft/DOCTRINE-SITE.json`

## 5. Merge-Safe Ordering

`bd-192` execution order:

1. Copy stable behavior text from design docs into proposed draft targets (no new
   semantics).
2. Add `SPEC-INDEX` references in informative/normative sections as appropriate.
3. Update `SPEC-TRACEABILITY` coverage rows for the promoted docs.
4. Keep operation IDs and failure-class strings byte-stable; if behavior changed,
   defer to follow-up issue.

## 6. Completion Checklist (for bd-192)

- [ ] `HARNESS-RUNTIME.md` added with command anchors and no behavior deltas.
- [ ] `HARNESS-RETRY-ESCALATION.md` added with policy and escalation table.
- [ ] `SPEC-INDEX.md` updated to include promoted harness draft docs.
- [ ] `SPEC-TRACEABILITY.md` updated with coverage rows for promoted docs.
- [ ] `docs/design/README.md` and `specs/premath/draft/README.md` references coherent.
- [ ] Validation passes:
  - `mise run traceability-check`
  - `mise run docs-coherence-check`
  - `mise run doctrine-check`
  - `python3 tools/ci/check_issue_graph.py`
