---
slug: raw
shortname: BEAM-COORDINATION
title: workingdoge.com/premath/BEAM-COORDINATION
name: Premath BEAM Coordination and Lease Protocol
status: raw
category: Standards Track
tags:
  - premath
  - coordination
  - lease
  - beam
  - otp
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Scope

This specification defines a coordination profile for multi-agent execution
where:

- BEAM/OTP provides supervision, scheduling, and failure recovery,
- Premath Rust command surfaces remain authority for mutation/check decisions,
- issue-memory and witness lanes remain deterministic and replayable.

This document is a runtime/control profile. It does not redefine kernel
admissibility authority.

## 2. Role split inside CI/Control

This profile adopts the existing one-layer, two-role split:

- check role: `draft/PREMATH-COHERENCE` (`premath coherence-check`),
- execute/attest role: `raw/PREMATH-CI` + `raw/CI-TOPOS`
  (`pipeline_*`, `run_*`, verify/decide surfaces).

Coordinator orchestration MUST treat both as control-plane roles and MUST NOT
treat either as semantic admissibility authority.

## 3. Authority boundary

Authority remains:

1. semantic admissibility: `draft/PREMATH-KERNEL` + `draft/GATE` +
   `draft/BIDIR-DESCENT`,
2. mutation admissibility: instruction-linked, capability-scoped host actions,
3. attestation lineage: deterministic witness/effect rows bound by
   `normalizerId + policyDigest`.

BEAM coordinator processes are orchestration surfaces only.

## 4. Runtime components

Minimum component split:

- `beam.coordinator`: claim/dispatch/retry/escalation orchestration,
- `beam.worker_sup`: bounded worker lifecycle supervision,
- `premath.authority`: Rust authority command surface (`premath-cli`/MCP),
- `memory.authority`: `.premath/issues.jsonl`,
- `evidence.projection`: harness session/trajectory + CI witness artifacts.

Deployments MAY run as one binary/process topology, but boundaries above MUST
remain explicit.

## 5. Instance and subagent model

An execution instance is a leased context:

```text
Instance {
  instanceId
  issueId
  worktreeRef|runtimeRef
  leaseId
  leaseOwner
  leaseTtl
  fenceToken
  policyDigest
}
```

Subagents MAY execute inside one instance only via capability-scoped subleases:

```text
Sublease {
  parentLeaseId
  subleaseId
  capabilityScope
  expiresAt
}
```

Subagents MUST NOT bypass host-action mutation boundaries.

## 6. Lease protocol (v0)

Canonical lifecycle:

```text
unclaimed -> active -> renewed -> released
                    \-> expired -> reclaim
                    \-> contended -> reconcile
```

Protocol requirements:

1. one active writer lease per issue/instance at a time,
2. lease renew/release are explicit protocol actions,
3. all mutation actions MUST carry current lease/fence context,
4. stale or mismatched fence context MUST reject deterministically,
5. expired leases MUST require reclaim before new mutation attempts.

Current phase-3 transport boundary:

- `issue.lease_renew` and `issue.lease_release` are MCP-only host actions,
- local REPL execution that needs these actions MUST escalate to MCP transport,
- local execution that attempts forbidden transport MUST fail closed as
  `control_plane_host_action_mcp_transport_required`.

## 7. Host-action bindings

Coordinator mutation paths MUST route through existing authority actions:

- issue lane: `issue.claim`, `issue.lease_renew`, `issue.lease_release`,
  `issue.update`, `issue.discover`,
- dependency lane: `dep.add`, `dep.remove`, `dep.replace`,
- check/attest lane: `instruction.check`, `instruction.run`,
  `coherence.check`, `required.*`.
- runtime fiber lane (transport lifecycle): `fiber.spawn`, `fiber.join`,
  `fiber.cancel`.

Coordinator implementations MUST use contract-bound command/tool mappings from
`draft/CONTROL-PLANE-CONTRACT.json` (`hostActionSurface`,
`commandSurface`, `pipelineWrapperSurface`).

### 7.1 World-route binding for BEAM lease lifecycle

BEAM lease orchestration MUST be world-bound, not ad-hoc:

| Host action | Operation ID | Route family | World ID | Morphism row |
| --- | --- | --- | --- | --- |
| `issue.claim` | `op/mcp.issue_claim` | `route.issue_claim_lease` | `world.lease.v1` | `wm.control.lease.mutation` |
| `issue.lease_renew` | `op/mcp.issue_lease_renew` | `route.issue_claim_lease` | `world.lease.v1` | `wm.control.lease.mutation` |
| `issue.lease_release` | `op/mcp.issue_lease_release` | `route.issue_claim_lease` | `world.lease.v1` | `wm.control.lease.mutation` |
| `issue.discover` | `op/mcp.issue_discover` | `route.issue_claim_lease` | `world.lease.v1` | `wm.control.lease.mutation` |

Coordinator startup and policy/contract reload SHOULD fail closed when this
binding drifts by running the canonical world-check surface:

```bash
cargo run --package premath-cli -- world-registry-check \
  --site-input specs/premath/draft/DOCTRINE-SITE-INPUT.json \
  --operations specs/premath/draft/DOCTRINE-OP-REGISTRY.json \
  --control-plane-contract specs/premath/draft/CONTROL-PLANE-CONTRACT.json \
  --required-route-family route.issue_claim_lease \
  --required-route-binding route.issue_claim_lease=op/mcp.issue_claim \
  --required-route-binding route.issue_claim_lease=op/mcp.issue_lease_renew \
  --required-route-binding route.issue_claim_lease=op/mcp.issue_lease_release \
  --required-route-binding route.issue_claim_lease=op/mcp.issue_discover \
  --json
```

This check delegates to kernel-owned world semantics and MUST remain the only
authority lane for world binding verdicts.

### 7.2 World-route binding for structured concurrency fibers

Fiber lifecycle orchestration SHOULD be world-bound through one route family:

| Host action | Operation ID | Route family | World ID | Morphism row |
| --- | --- | --- | --- | --- |
| `fiber.spawn` | `op/transport.fiber_spawn` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |
| `fiber.join` | `op/transport.fiber_join` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |
| `fiber.cancel` | `op/transport.fiber_cancel` | `route.fiber.lifecycle` | `world.fiber.v1` | `wm.control.fiber.lifecycle` |

These rows are transport/runtime orchestration bindings. They MUST NOT bypass
semantic admissibility authority.

## 8. Descent execution model

Coordinator decomposition MUST follow cover/descent shape:

1. choose cover `{Gamma_i -> Gamma}` from ready claims/worktrees,
2. run bounded local sections per worker leg,
3. validate overlap compatibility on pullbacks,
4. glue accepted sections or emit deterministic obstruction.

Compatibility MUST be checked via existing surfaces (for example
`harness-join-check`, dependency diagnostics, required/coherence verification).

## 9. Witness and effect requirements

Every host action SHOULD emit one deterministic host-effect envelope:

- `schema = premath.host_effect.v0`,
- `action`, `argsDigest`, `resultClass`,
- `failureClasses[]`, `witnessRefs[]`,
- `policyDigest` and `instructionRef` for mutation-capable actions.

Each envelope MUST bind to canonical trajectory storage:

- `.premath/harness_trajectory.jsonl`,
- row kind `premath.harness.step.v1`.

Trajectories are projection artifacts and MUST NOT be treated as semantic
authority by themselves.

## 10. OTP supervision profile

Recommended process shape:

```text
premath_coord_sup
  |- lease_dispatcher (gen_server)
  |- worker_sup (dynamic supervisor)
  |- evidence_writer (gen_server)
  \- authority_client (Rust boundary client)
```

Supervision constraints:

1. worker crashes MUST not mutate authority state implicitly,
2. retries MUST be policy-driven and bounded,
3. terminal outcomes MUST route through deterministic escalation actions
   (`issue_discover` / `mark_blocked` / `stop`),
4. restart behavior MUST preserve replayable lineage refs.

## 11. Rust boundary modes

Coordinator/Rust integration MAY use:

- NIF boundary (`rustler`, https://docs.rs/rustler/latest/rustler/) for short
  deterministic calls,
- generic transport RPC boundary (for example gRPC) for remote/sidecar command
  transport,
- sidecar/port boundary for long-running or blocking operations.

Either mode MUST preserve identical authority payload semantics and failure-class
outputs for fixed inputs.

Repository reference implementation lane:

- `crates/premath-transport` provides transport-facing lease bridge functions and
  optional `rustler_nif` export `dispatch(request_json)` over canonical
  `action + payload` envelopes for `issue.claim`, `issue.lease_renew`, and
  `issue.lease_release`.

When NIF mode is used:

- long CPU/IO NIF work SHOULD use `#[rustler::nif(schedule = "DirtyCpu")]` or
  `#[rustler::nif(schedule = "DirtyIo")]`, or be routed to sidecar mode,
- NIFs MUST be transport adapters only (no alternate mutation authority lane;
  no direct write authority over `.premath/issues.jsonl` outside canonical
  Premath command/library surfaces),
- NIF return payloads MUST preserve deterministic failure-class and witness-ref
  projections used by CLI/MCP surfaces for equivalent inputs.

## 12. Unified Evidence binding

This profile MUST satisfy `draft/UNIFICATION-DOCTRINE` §10 and §12:

1. operational artifact families factor through one evidence family `Ev`,
2. no parallel admissibility route is introduced,
3. missing/ambiguous/unbound factorization routes fail closed.

Minimum fail-closed classes are:

- `unification.evidence_factorization.missing`,
- `unification.evidence_factorization.ambiguous`,
- `unification.evidence_factorization.unbound`.

## 13. Conformance targets for promotion

Promotion from raw SHOULD require:

1. deterministic lease protocol vectors (golden/adversarial/invariance),
2. explicit fence-token stale/contended/expired rejection vectors,
3. coordinator retry/escalation parity vectors against policy contract,
4. host-action transport boundary vectors (`mcp` vs `local-repl`),
5. typed witness lineage parity across CLI/MCP/coordinator entrypoints.

## 14. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`,
- `dm.profile.execution` (coordinator/runtime backend neutrality),
- `dm.commitment.attest` (witness/effect lineage stability),
- `dm.presentation.projection` (adapter/transport neutrality).

Not preserved:

- `dm.transport.world` / `dm.transport.location` (runtime transport layer),
- `dm.refine.context` / `dm.refine.cover` (kernel/runtime semantic layers).
