# Control-Plane Threat Model (v0)

This document is non-normative and implementation-facing.

Scope:

- instruction control-plane surfaces.
- Witness/projection integrity across checker adapters.
- Tracker mutation safety is outside Premath and belongs to Tusk/downstream.

Non-goals:

- Replacing kernel/Gate admissibility (semantic authority stays in `specs/`).
- Full host/runtime sandbox design for external runners.

## Trust Boundaries

Boundary summary:

- Kernel/Gate/Core obligations: semantic admissibility authority.
- Coherence checker: control-plane consistency checker.
- workflow wrappers and native CLI tools: execution/projection layer only.
- External tracker: operational state substrate owned outside Premath.

Assets:

- Canonical contracts (`COHERENCE-CONTRACT`, `CONTROL-PLANE-CONTRACT`).
- runtime witness artifacts (`artifacts/witness/*`).
- Tracker boundary integrity for normalized claims entering Premath.

## Threat Matrix

| ID | Threat | Primary impact | Current controls | Residual gap |
|---|---|---|---|---|
| CP-01 | Untyped proposal/checker input bypass | Unauthorized mutation/execution | `proposal-check`, `coherence-check`, and externalized mutation admission | Keep execution policy in the owning runtime site |
| CP-02 | Parallel semantic surfaces drift | Contradictory control-plane truth | `coherence-check`, `traceability-check`, `drift-budget-check` | Continue reducing duplicate wrappers during migrations |
| CP-03 | Unauthorized tracker mutation actions | Work graph corruption | External tracker admission plus `premath work-tracker-check` at the boundary | Keep tracker mutation tests in the owning Tusk/downstream site |
| CP-04 | Witness/projection integrity mismatch | False pass/fail claims | checker witness and projection parity through `coherence-check` / `drift-budget-check` | Keep projection schema migration discipline and semantic invariance tests strict |
| CP-05 | Dependency graph poisoning (cycle/self-loop) | Ready queue deadlock / hidden blockers | Owning tracker diagnostics | Keep graph diagnostics in the owning Tusk/downstream site |
| CP-06 | Cache closure drift for coherence/checker inputs | Stale checker semantics | Coherence cache-input closure + drift-budget cache checks | Keep closure updated when new checker input paths are introduced |
| CP-07 | Local/private artifact leakage into repo | Policy/compliance break | `repo-hygiene-check` | Keep ignore/policy lists synced with new local tooling |
| CP-08 | External runner profile misuse | Untrusted execution surface | profile split (`local`/`external`), canonical gate wrappers | Formal external runner hardening profile remains incremental |

## Hardening Matrix

| Control | Status | Enforced by |
|---|---|---|
| Contract/checker drift sentinels | Implemented | `premath drift-budget-check` |
| Lane ownership + cross-lane route checks | Implemented | `premath coherence-check` |
| Doctrine operation-route reachability | Implemented | `premath coherence-check` |
| Dependency mutation safety (remove/replace/cycle rejection) | Externalized | Tusk/downstream tracker |
| Schema/version and deprecation policy | In progress | lifecycle/coherence flows |
| Reviewer-gated policy hardening | Pending reviewer pool | governance rollout when reviewer pool exists |

## Operating Rule

Minimum encoding, maximum expressiveness:

- one authority surface per semantic claim,
- one deterministic projection path per consumer class,
- fail closed on drift.

Live roadmap source:

- Tusk/downstream tracker state
