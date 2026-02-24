# Control-Plane Threat Model (v0)

This document is non-normative and implementation-facing.

Scope:

- CI/instruction/issue-mutation control-plane surfaces.
- Witness/projection integrity across CLI/MCP/CI adapters.
- Dependency graph mutation safety for long-running issue memory.

Non-goals:

- Replacing kernel/Gate admissibility (semantic authority stays in `specs/`).
- Full host/runtime sandbox design for external runners.

## Trust Boundaries

Boundary summary:

- Kernel/Gate/BIDIR: semantic admissibility authority.
- Coherence checker: control-plane consistency checker.
- CI wrappers + MCP tools: execution/transport layer only.
- Issue memory (`.premath/issues.jsonl`): operational state substrate.

Assets:

- Canonical contracts (`COHERENCE-CONTRACT`, `CONTROL-PLANE-CONTRACT`).
- CI witness artifacts (`artifacts/ciwitness/*`).
- Observation projection (`artifacts/observation/*`).
- Dependency graph integrity (no hidden cycles/self-loops).

## Threat Matrix

| ID | Threat | Primary impact | Current controls | Residual gap |
|---|---|---|---|---|
| CP-01 | Instruction envelope bypass / untyped execution | Unauthorized mutation/execution | `instruction_check`, `instruction_run`, mutation-policy instruction linking | Tighten policy rollout + reviewer governance once reviewer pool is available |
| CP-02 | Parallel semantic surfaces drift | Contradictory control-plane truth | `coherence-check`, `docs-coherence-check`, `ci-drift-budget-check`, doctrine-site + MCP parity checks (`doctrine-site-check`, `doctrine-mcp-parity-check`) | Continue reducing duplicate wrappers during migrations |
| CP-03 | Unauthorized issue mutation actions | Work graph corruption | Instruction-linked capability claims (`capabilities.change_morphisms.*`) | Expand threat tests for new mutation actions over time |
| CP-04 | Witness/projection integrity mismatch | False pass/fail claims | required witness/decision verify path, observation semantic invariance checks | Keep projection schema migration discipline and semantic invariance tests strict |
| CP-05 | Dependency graph poisoning (cycle/self-loop) | Ready queue deadlock / hidden blockers | `dep add` cycle/self-loop rejection, `dep diagnostics`, MCP `dep_add` cycle rejection, MCP `dep_diagnostics` scoped cycle checks | Keep graph diagnostics + readiness policy coverage growing with new dep operations |
| CP-06 | Cache closure drift for coherence/control-plane loaders | Stale checker semantics | Coherence cache-input closure + drift-budget cache checks | Keep closure updated when new loader paths are introduced |
| CP-07 | Local/private artifact leakage into repo | Policy/compliance break | `ci-hygiene-check`, branch-policy checks | Keep ignore/policy lists synced with new local tooling |
| CP-08 | External runner profile misuse | Untrusted execution surface | profile split (`local`/`external`), canonical gate wrappers | Formal external runner hardening profile remains incremental |

## Hardening Matrix

| Control | Status | Enforced by |
|---|---|---|
| Typed instruction gate before execution | Implemented | `mise run ci-instruction-check`, instruction pipeline |
| Contract/checker drift sentinels | Implemented | `mise run ci-drift-budget-check` |
| Lane ownership + cross-lane route checks | Implemented | `premath coherence-check`, conformance vectors |
| Doctrine operation-route parity for MCP surfaces | Implemented | `mise run doctrine-check` (`doctrine-site-check`, `doctrine-mcp-parity-check`) |
| Dependency mutation safety (remove/replace/cycle rejection) | Implemented | `premath dep *`, MCP `dep_add/dep_remove/dep_replace` |
| Schema/version and deprecation policy | In progress | lifecycle/coherence flows + issue-memory roadmap |
| Reviewer-gated policy hardening | Pending reviewer pool | governance rollout when reviewer pool exists |

## Operating Rule

Minimum encoding, maximum expressiveness:

- one authority surface per semantic claim,
- one deterministic projection path per consumer class,
- fail closed on drift.

Live roadmap source:

- `.premath/issues.jsonl`
- `premath issue ready` / `premath issue list`
